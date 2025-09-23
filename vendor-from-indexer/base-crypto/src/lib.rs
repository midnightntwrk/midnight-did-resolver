#![deny(unreachable_pub)]
//#![deny(warnings)]
#![deny(missing_docs)]
// Proptest derive triggers this.
#![allow(non_local_definitions)]

//! This crate collects cryptographic primitives used in Midnight's ledger.
//! All primitives, including zero-knowledge, curve choice, and crypto-aware
//! data structures are defined here, and should be added here to decouple from
//! any specific implementation.

#[cfg(feature = "data-provider")]
pub mod data_provider;
pub mod fab;
pub mod hash;
pub mod repr;
pub mod rng;
pub mod signatures;

/// Re-export of [`serialize`] lib.
pub use serialize;

pub use repr::*;
