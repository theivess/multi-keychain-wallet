// File: src/multi_keychain/tx_builder.rs
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