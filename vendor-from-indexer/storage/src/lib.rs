#![deny(unreachable_pub)]
//#![deny(warnings)]
#![deny(missing_docs)]
//! Merkle-ized data structures and persistent disk storage.
//!
//! This crate provides storage primitives, primarily maps, for use
//! in larger data structures. It also exposes its deduplicated memory arena,
//! [`Arena`](arena::Arena), and pointers within it, [`Sp`](arena::Sp).

pub mod arena;
pub mod backend;
pub mod db;
pub mod merkle_patricia_trie;
pub mod storable;
pub mod storage;

pub use macros::Storable;
pub use storable::{Storable, WellBehavedHasher};
pub use storage::Storage;

mod cache;

#[cfg(any(test, feature = "stress-test"))]
mod test;

// Stress testing utilities. Needs to be pub since we call it from a bin
// target. But not meant to be consumed by library users.
#[cfg(feature = "stress-test")]
pub mod stress_test;

/// The default storage mechanism.
pub type DefaultHasher = sha2::Sha256;
/// The default database.
pub type DefaultDB = db::InMemoryDB<DefaultHasher>;

/// Re-export of `base-crypto` lib.
pub use base_crypto;

/// Re-export of `serialize` lib.
pub use base_crypto::serialize;
