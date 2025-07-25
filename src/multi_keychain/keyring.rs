use bdk_chain::{DescriptorExt, Merge};
use bdk_wallet::descriptor::IntoWalletDescriptor;
use bitcoin::{
    secp256k1::{All, Secp256k1},
    Network,
};
use miniscript::{Descriptor, DescriptorPublicKey};
use serde::{Deserialize, Serialize};

use crate::bdk_chain;
use crate::collections::BTreeMap;
use crate::multi_keychain::{Did, errors::KeyRingError};

/// KeyRing.
#[derive(Debug, Clone)]
pub struct KeyRing<K> {
    pub(crate) secp: Secp256k1<All>,
    pub(crate) network: Network,
    pub(crate) descriptors: BTreeMap<K, Descriptor<DescriptorPublicKey>>,
}

impl<K> KeyRing<K>
where
    K: Ord + Clone,
{
    /// Construct new [`KeyRing`] with the provided `network`.
    pub fn new(network: Network) -> Self {
        Self {
            secp: Secp256k1::new(),
            network,
            descriptors: BTreeMap::default(),
        }
    }

    /// Add descriptor with validation
    pub fn add_descriptor_validated(
        &mut self,
        keychain: K,
        descriptor: impl IntoWalletDescriptor
    ) -> Result<(), KeyRingError> {
        let (descriptor, _) = descriptor
            .into_wallet_descriptor(&self.secp, self.network)
            .map_err(|_| KeyRingError::DescriptorParsing)?;

        if descriptor.is_multipath() {
            return Err(KeyRingError::MultipathDescriptorNotAllowed);
        }

        if self.descriptors.contains_key(&keychain) {
            return Err(KeyRingError::DuplicateDescriptor);
        }

        // Validate we can derive a script pubkey (this is the proper validation)
        descriptor.at_derivation_index(0)
            .map_err(|_| KeyRingError::AddressGeneration)?;

        self.descriptors.insert(keychain, descriptor);
        Ok(())
    }

    /// Add descriptor, must not be [multipath](miniscript::Descriptor::is_multipath).
    pub fn add_descriptor(&mut self, keychain: K, descriptor: impl IntoWalletDescriptor) {
        let descriptor = descriptor
            .into_wallet_descriptor(&self.secp, self.network)
            .expect("err: invalid descriptor")
            .0;
        assert!(
            !descriptor.is_multipath(),
            "err: Use `add_multipath_descriptor` instead"
        );

        self.descriptors.insert(keychain, descriptor);
    }

    /// Validate the entire keyring
    pub fn validate(&self) -> Result<(), KeyRingError> {
        if self.descriptors.is_empty() {
            return Err(KeyRingError::EmptyKeyRing);
        }

        for (_, descriptor) in &self.descriptors {
            // Test that we can derive at index 0
            descriptor.at_derivation_index(0)
                .map_err(|_| KeyRingError::AddressGeneration)?;
        }

        Ok(())
    }

    /// Check if keychain exists
    pub fn contains_keychain(&self, keychain: &K) -> bool {
        self.descriptors.contains_key(keychain)
    }

    /// Get descriptor count
    pub fn descriptor_count(&self) -> usize {
        self.descriptors.len()
    }

    /// Get a descriptor for a specific keychain
    pub fn get_descriptor(&self, keychain: &K) -> Option<&Descriptor<DescriptorPublicKey>> {
        self.descriptors.get(keychain)
    }

    /// Remove a keychain and return whether it existed
    pub fn remove_keychain(&mut self, keychain: &K) -> bool {
        self.descriptors.remove(keychain).is_some()
    }

    /// Check if keyring is empty
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    /// List all keychains
    pub fn keychains(&self) -> impl Iterator<Item = &K> {
        self.descriptors.keys()
    }

    /// Initial changeset.
    pub fn initial_changeset(&self) -> ChangeSet<K> {
        ChangeSet {
            network: Some(self.network),
            descriptors: self.descriptors.clone(),
        }
    }

    /// Construct from changeset.
    pub fn from_changeset(changeset: ChangeSet<K>) -> Option<Self> {
        Some(Self {
            secp: Secp256k1::new(),
            network: changeset.network?,
            descriptors: changeset.descriptors,
        })
    }
}

impl KeyRing<Did> {
    /// Add multipath descriptor with validation
    pub fn add_multipath_descriptor_validated(
        &mut self,
        descriptor: impl IntoWalletDescriptor
    ) -> Result<(), KeyRingError> {
        let (descriptor, _) = descriptor
            .into_wallet_descriptor(&self.secp, self.network)
            .map_err(|_| KeyRingError::DescriptorParsing)?;

        if !descriptor.is_multipath() {
            return Err(KeyRingError::SingleDescriptorNotAllowed);
        }

        let descriptors = descriptor
            .into_single_descriptors()
            .map_err(|_| KeyRingError::DescriptorParsing)?;

        for descriptor in descriptors {
            let did = descriptor.descriptor_id();

            if self.descriptors.contains_key(&did) {
                return Err(KeyRingError::DuplicateDescriptor);
            }

            // Validate we can derive a script pubkey
            descriptor.at_derivation_index(0)
                .map_err(|_| KeyRingError::AddressGeneration)?;

            self.descriptors.insert(did, descriptor);
        }

        Ok(())
    }

    /// Add multipath descriptor.
    pub fn add_multipath_descriptor(&mut self, descriptor: impl IntoWalletDescriptor) {
        let descriptor = descriptor
            .into_wallet_descriptor(&self.secp, self.network)
            .expect("err: invalid descriptor")
            .0;
        assert!(
            descriptor.is_multipath(),
            "err: Use `add_descriptor` instead"
        );
        let descriptors = descriptor
            .into_single_descriptors()
            .expect("err: invalid descriptor");
        for descriptor in descriptors {
            let did = descriptor.descriptor_id();
            self.descriptors.insert(did, descriptor);
        }
    }
}

/// Represents changes to the `KeyRing`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet<K: Ord> {
    /// Network.
    pub network: Option<Network>,
    /// Added descriptors.
    pub descriptors: BTreeMap<K, Descriptor<DescriptorPublicKey>>,
}

impl<K: Ord> Default for ChangeSet<K> {
    fn default() -> Self {
        Self {
            network: None,
            descriptors: BTreeMap::default(),
        }
    }
}

impl<K: Ord> Merge for ChangeSet<K> {
    fn merge(&mut self, other: Self) {
        if self.network.is_none() {
            self.network = other.network;
        }
        self.descriptors.extend(other.descriptors);
    }

    fn is_empty(&self) -> bool {
        self.network.is_none() && self.descriptors.is_empty()
    }
}