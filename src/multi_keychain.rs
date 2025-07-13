//! Module containing the multi-keychain [`Wallet`].

mod changeset;
pub mod keyring;
mod wallet;
pub mod errors;

pub use changeset::*;
pub use keyring::KeyRing;
pub use wallet::*;

/// Alias for [`DescriptorId`](bdk_chain::DescriptorId).
pub(crate) type Did = crate::bdk_chain::DescriptorId;
