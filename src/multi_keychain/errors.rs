use bdk_wallet::descriptor::DescriptorError;
use bitcoin::Network;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRingError {
    /// Attempted to add a descriptor that already exists for this keychain
    DuplicateDescriptor,
    /// The provided descriptor is invalid - multipath when single expected
    MultipathDescriptorNotAllowed,
    /// The provided descriptor is invalid - single when multipath expected  
    SingleDescriptorNotAllowed,
    /// Network mismatch between descriptor and keyring
    NetworkMismatch { expected: Network, found: Network },
    /// Keyring is empty when an operation requires descriptors
    EmptyKeyRing,
    /// Keychain not found in the keyring
    KeychainNotFound,
    /// Descriptor parsing failed
    DescriptorParsing,
    /// Address generation failed
    AddressGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// SQLite database error
    Database,
    /// Serialization failed
    Serialization,
    /// Deserialization failed
    Deserialization,
    /// File system error
    FileSystem,
    /// Data corruption detected
    DataCorruption,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxBuilderError {
    /// No recipients specified
    NoRecipients,
    /// Insufficient funds
    InsufficientFunds { required: u64, available: u64 },
    /// No UTXOs available
    NoUtxos,
    /// Fee rate too low
    FeeTooLow,
    /// Fee rate too high
    FeeTooHigh,
    /// Output below dust threshold
    DustOutput,
    /// Invalid recipient address
    InvalidRecipient,
    /// PSBT creation failed
    PsbtCreation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningError {
    /// Missing private key for signing
    MissingPrivateKey,
    /// Invalid signature
    InvalidSignature,
    /// PSBT is already finalized
    AlreadyFinalized,
    /// Signing failed
    SigningFailed,
    /// Input not found
    InputNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressGenerationError {
    /// No more addresses available (reached derivation limit)
    DerivationLimit,
    /// Keychain not found
    KeychainNotFound,
    /// Descriptor error
    Descriptor,
    /// Network incompatible
    NetworkIncompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletError {
    /// KeyRing related error
    KeyRing(KeyRingError),
    /// Persistence related error
    Persistence(PersistenceError),
    /// Transaction building error
    TxBuilder(TxBuilderError),
    /// Signing error
    Signing(SigningError),
    /// Address generation error
    AddressGeneration(AddressGenerationError),
}

// Only implement Display and Error traits when std is available
#[cfg(feature = "std")]
mod display_impls {
    use super::*;
    use std::error::Error;
    use std::fmt;

    impl fmt::Display for KeyRingError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                KeyRingError::DuplicateDescriptor => {
                    write!(f, "Descriptor already exists for this keychain")
                }
                KeyRingError::MultipathDescriptorNotAllowed => write!(
                    f,
                    "Multipath descriptor not allowed, use add_multipath_descriptor instead"
                ),
                KeyRingError::SingleDescriptorNotAllowed => write!(
                    f,
                    "Single descriptor not allowed, use add_descriptor instead"
                ),
                KeyRingError::NetworkMismatch { expected, found } => write!(
                    f,
                    "Network mismatch: expected {}, found {}",
                    expected, found
                ),
                KeyRingError::EmptyKeyRing => write!(f, "KeyRing is empty"),
                KeyRingError::KeychainNotFound => write!(f, "Keychain not found in keyring"),
                KeyRingError::DescriptorParsing => write!(f, "Failed to parse descriptor"),
                KeyRingError::AddressGeneration => {
                    write!(f, "Failed to generate address from descriptor")
                }
            }
        }
    }

    impl fmt::Display for PersistenceError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                PersistenceError::Database => write!(f, "Database error"),
                PersistenceError::Serialization => write!(f, "Serialization failed"),
                PersistenceError::Deserialization => write!(f, "Deserialization failed"),
                PersistenceError::FileSystem => write!(f, "File system error"),
                PersistenceError::DataCorruption => write!(f, "Data corruption detected"),
            }
        }
    }

    impl fmt::Display for TxBuilderError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TxBuilderError::NoRecipients => write!(f, "No recipients specified"),
                TxBuilderError::InsufficientFunds {
                    required,
                    available,
                } => {
                    write!(
                        f,
                        "Insufficient funds: required {} sats, available {} sats",
                        required, available
                    )
                }
                TxBuilderError::NoUtxos => write!(f, "No UTXOs available"),
                TxBuilderError::FeeTooLow => write!(f, "Fee rate too low"),
                TxBuilderError::FeeTooHigh => write!(f, "Fee rate too high"),
                TxBuilderError::DustOutput => write!(f, "Output below dust threshold"),
                TxBuilderError::InvalidRecipient => write!(f, "Invalid recipient address"),
                TxBuilderError::PsbtCreation => write!(f, "PSBT creation failed"),
            }
        }
    }

    impl fmt::Display for SigningError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SigningError::MissingPrivateKey => write!(f, "Missing private key for signing"),
                SigningError::InvalidSignature => write!(f, "Invalid signature"),
                SigningError::AlreadyFinalized => write!(f, "PSBT is already finalized"),
                SigningError::SigningFailed => write!(f, "Signing failed"),
                SigningError::InputNotFound => write!(f, "Input not found"),
            }
        }
    }

    impl fmt::Display for AddressGenerationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                AddressGenerationError::DerivationLimit => write!(f, "Reached derivation limit"),
                AddressGenerationError::KeychainNotFound => write!(f, "Keychain not found"),
                AddressGenerationError::Descriptor => write!(f, "Descriptor error"),
                AddressGenerationError::NetworkIncompatible => write!(f, "Network incompatible"),
            }
        }
    }

    impl fmt::Display for WalletError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                WalletError::KeyRing(e) => write!(f, "KeyRing error: {}", e),
                WalletError::Persistence(e) => write!(f, "Persistence error: {}", e),
                WalletError::TxBuilder(e) => write!(f, "Transaction builder error: {}", e),
                WalletError::Signing(e) => write!(f, "Signing error: {}", e),
                WalletError::AddressGeneration(e) => write!(f, "Address generation error: {}", e),
            }
        }
    }

    impl Error for KeyRingError {}
    impl Error for PersistenceError {}
    impl Error for TxBuilderError {}
    impl Error for SigningError {}
    impl Error for AddressGenerationError {}
    impl Error for WalletError {}
}

// Conversions (always available)
impl From<KeyRingError> for WalletError {
    fn from(err: KeyRingError) -> Self {
        WalletError::KeyRing(err)
    }
}

impl From<PersistenceError> for WalletError {
    fn from(err: PersistenceError) -> Self {
        WalletError::Persistence(err)
    }
}

impl From<TxBuilderError> for WalletError {
    fn from(err: TxBuilderError) -> Self {
        WalletError::TxBuilder(err)
    }
}

impl From<SigningError> for WalletError {
    fn from(err: SigningError) -> Self {
        WalletError::Signing(err)
    }
}

impl From<AddressGenerationError> for WalletError {
    fn from(err: AddressGenerationError) -> Self {
        WalletError::AddressGeneration(err)
    }
}

// External error conversions
impl From<DescriptorError> for KeyRingError {
    fn from(_: DescriptorError) -> Self {
        KeyRingError::DescriptorParsing
    }
}

#[cfg(feature = "rusqlite")]
impl From<bdk_chain::rusqlite::Error> for PersistenceError {
    fn from(_: bdk_chain::rusqlite::Error) -> Self {
        PersistenceError::Database
    }
}
