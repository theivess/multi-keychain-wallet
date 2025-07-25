use bitcoin::{Address, Amount, FeeRate, OutPoint, Transaction, TxOut, Psbt};
use crate::bdk_chain::CanonicalizationParams;
use alloc::vec::Vec;

use crate::multi_keychain::{Wallet, errors::{WalletError, TxBuilderError}};

pub struct TxBuilder<'a, K: Ord> {
    wallet: &'a mut Wallet<K>,
    recipients: Vec<(Address, Amount)>,
    fee_rate: Option<FeeRate>,
    preferred_keychain: Option<K>,
    drain_wallet: bool,
    utxos: Vec<OutPoint>,
}

impl<'a, K> TxBuilder<'a, K>
where
    K: core::fmt::Debug + Clone + Ord,
{
    pub fn new(wallet: &'a mut Wallet<K>) -> Self {
        Self {
            wallet,
            recipients: Vec::new(),
            fee_rate: None,
            preferred_keychain: None,
            drain_wallet: false,
            utxos: Vec::new(),
        }
    }

    pub fn add_recipient(mut self, address: Address, amount: Amount) -> Self {
        self.recipients.push((address, amount));
        self
    }

    pub fn fee_rate(mut self, fee_rate: FeeRate) -> Self {
        self.fee_rate = Some(fee_rate);
        self
    }

    pub fn prefer_keychain(mut self, keychain: K) -> Self {
        self.preferred_keychain = Some(keychain);
        self
    }

    pub fn drain_wallet(mut self) -> Self {
        self.drain_wallet = true;
        self
    }

    pub fn add_utxo(mut self, outpoint: OutPoint) -> Self {
        self.utxos.push(outpoint);
        self
    }


    fn get_available_utxos(&self) -> Result<Vec<LocalUtxo<K>>, WalletError> {
        let chain = self.wallet.local_chain();
        let tx_graph = self.wallet.tx_graph();
        let tip = chain.tip().block_id();
        let params = CanonicalizationParams::default();

        let mut utxos = Vec::new();

        for ((keychain, index), outpoint) in tx_graph.index.outpoints() {
            if let Some(preferred) = &self.preferred_keychain {
                if keychain != preferred {
                    continue;
                }
            }

            if let Some(tx_node) = tx_graph.graph().get_tx_node(outpoint.txid) {
                if let Some(txout) = tx_node.tx.output.get(outpoint.vout as usize) {
                    let is_unspent = tx_graph.graph()
                        .filter_chain_unspents(chain, tip, params.clone(), [((), *outpoint)].iter().cloned())
                        .next()
                        .is_some();

                    if is_unspent {
                        utxos.push(LocalUtxo {
                            outpoint: *outpoint,
                            txout: txout.clone(),
                            keychain: keychain.clone(),
                            derivation_index: *index,
                        });
                    }
                }
            }
        }

        Ok(utxos)
    }

    fn select_coins(&self, mut utxos: Vec<LocalUtxo<K>>, fee_rate: FeeRate) -> Result<Vec<LocalUtxo<K>>, WalletError> {
        if utxos.is_empty() {
            return Err(TxBuilderError::NoUtxos.into());
        }

        // Sort by value (largest first)
        utxos.sort_by(|a, b| b.txout.value.cmp(&a.txout.value));

        if self.drain_wallet {
            return Ok(utxos);
        }

        let target: Amount = self.recipients.iter().map(|(_, amount)| *amount).sum();
        let mut selected = Vec::new();
        let mut selected_value = Amount::ZERO;

        for utxo in utxos {
            selected.push(utxo);
            selected_value += selected.last().unwrap().txout.value;

            let estimated_fee = fee_rate.fee_vb(self.estimate_tx_size(selected.len(), self.recipients.len())).unwrap_or(Amount::ZERO);

            if selected_value >= target + estimated_fee {
                break;
            }
        }

        let final_fee = fee_rate.fee_vb(self.estimate_tx_size(selected.len(), self.recipients.len())).unwrap_or(Amount::ZERO);
        if selected_value < target + final_fee {
            return Err(TxBuilderError::InsufficientFunds {
                required: (target + final_fee).to_sat(),
                available: selected_value.to_sat()
            }.into());
        }

        Ok(selected)
    }

    fn estimate_tx_size(&self, inputs: usize, outputs: usize) -> u64 {
        // Simplified transaction size estimation
        let base_size = 10u64; // version, locktime, etc.
        let input_size = inputs as u64 * 148; // approximate P2WPKH input size
        let output_size = outputs as u64 * 34; // approximate output size
        base_size + input_size + output_size
    }

    fn create_psbt(&mut self, selected_utxos: Vec<LocalUtxo<K>>, fee_rate: FeeRate) -> Result<(Psbt, TransactionDetails), WalletError> {
        let selected_value: Amount = selected_utxos.iter().map(|u| u.txout.value).sum();
        let target_value: Amount = self.recipients.iter().map(|(_, amount)| *amount).sum();
        let estimated_fee = fee_rate.fee_vb(self.estimate_tx_size(selected_utxos.len(), self.recipients.len())).unwrap_or(Amount::ZERO);

        let mut tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: Vec::new(),
            output: Vec::new(),
        };

        // Add inputs
        for utxo in &selected_utxos {
            tx.input.push(bitcoin::TxIn {
                previous_output: utxo.outpoint,
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: bitcoin::Witness::new(),
            });
        }

        // Add outputs
        if self.drain_wallet {
            if let Some((address, _)) = self.recipients.first() {
                tx.output.push(TxOut {
                    value: selected_value - estimated_fee,
                    script_pubkey: address.script_pubkey(),
                });
            }
        } else {
            for (address, amount) in &self.recipients {
                tx.output.push(TxOut {
                    value: *amount,
                    script_pubkey: address.script_pubkey(),
                });
            }

            // Add change if needed
            let change = selected_value - target_value - estimated_fee;
            if change > Amount::from_sat(546) { // dust threshold
                if let Some(keychain) = selected_utxos.first().map(|u| u.keychain.clone()) {
                    if let Some(((_, _), change_addr)) = self.wallet.reveal_next_address(keychain) {
                        tx.output.push(TxOut {
                            value: change,
                            script_pubkey: change_addr.script_pubkey(),
                        });
                    }
                }
            }
        }

        let psbt = Psbt::from_unsigned_tx(tx)
            .map_err(|_| TxBuilderError::PsbtCreation)?;

        let details = TransactionDetails {
            txid: psbt.unsigned_tx.compute_txid(),
            sent: if self.drain_wallet { selected_value - estimated_fee } else { target_value },
            received: Amount::ZERO,
            fee: Some(estimated_fee),
        };

        Ok((psbt, details))
    }

    pub fn finish(mut self) -> Result<(Psbt, TransactionDetails), WalletError> {
        if self.recipients.is_empty() && !self.drain_wallet {
            return Err(TxBuilderError::NoRecipients.into());
        }

        let available_utxos = self.get_available_utxos()?;
        let fee_rate = self.fee_rate.unwrap_or(FeeRate::from_sat_per_vb_unchecked(1));

        // Simple coin selection
        let selected_utxos = self.select_coins(available_utxos, fee_rate)?;
        let (psbt, details) = self.create_psbt(selected_utxos, fee_rate)?;

        Ok((psbt, details))
    }
}

#[derive(Debug, Clone)]
pub struct LocalUtxo<K> {
    pub outpoint: OutPoint,
    pub txout: TxOut,
    pub keychain: K,
    pub derivation_index: u32,
}

#[derive(Debug, Clone)]
pub struct TransactionDetails {
    pub txid: bitcoin::Txid,
    pub sent: Amount,
    pub received: Amount,
    pub fee: Option<Amount>,
}