#![deny(unreachable_pub)]
#![deny(warnings)]

//! This crate implements transaction assembly and semantics for Midnight as a prototype.

#[macro_use]
extern crate tracing;

#[cfg(feature = "transaction-construction")]
pub mod construct;
pub mod error;
#[path = "tracing.rs"]
mod ledger_tracing;
mod prior_versions;
#[cfg(feature = "proving")]
pub mod prove;
#[cfg(feature = "transaction-semantics")]
pub mod semantics;
pub mod structure;
mod utils;
#[cfg(feature = "verifying")]
pub mod verify;

pub use ledger_tracing::{LogLevel, init_logger};

#[cfg(feature = "test-utilities")]
pub mod test_utilities;

/// Re-exports
pub use {
    coin_structure::{self, base_crypto, serialize, storage, transient_crypto},
    introspection, onchain_runtime, zswap,
};
