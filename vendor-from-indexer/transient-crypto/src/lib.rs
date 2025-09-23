#![deny(unreachable_pub)]
//#![deny(warnings)]
#![deny(missing_docs)]

//! This crate collects cryptographic primitives used in Midnight's ledger.
//! All primitives, including zero-knowledge, curve choice, and crypto-aware
//! data structures are defined here, and should be added here to decouple from
//! any specific implementation.

#[macro_use]
extern crate tracing;

pub mod commitment;
pub mod curve;
pub mod encryption;
pub mod fab;
pub mod hash;
mod macros;
pub mod merkle_tree;
pub mod proofs;
pub mod repr;

/// Re-export of `base-crypto` lib.
pub use base_crypto;
