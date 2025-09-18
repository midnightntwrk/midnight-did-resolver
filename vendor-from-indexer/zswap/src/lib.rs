#![deny(unreachable_pub)]
#![deny(warnings)]
#![allow(unused_imports)]

#[macro_use]
extern crate tracing;
#[cfg(any(feature = "proving", feature = "verifying"))]
#[macro_use]
extern crate lazy_static;

pub(crate) const ZSWAP_TREE_HEIGHT: u8 = 32;

#[cfg(any(feature = "proof-verifying", feature = "offer-construction"))]
pub(crate) fn ciphertext_to_field(c: &CoinCiphertext) -> transient_crypto::curve::Fr {
    use transient_crypto::hash::{transient_commit, transient_hash};
    transient_commit(
        &c.ciph[..],
        transient_hash(&[c.c.x().unwrap_or(0.into()), c.c.y().unwrap_or(0.into())]),
    )
}

//mod core;
#[cfg(feature = "offer-construction")]
mod construct;
pub mod error;
pub mod keys;
#[cfg(feature = "ledger")]
pub mod ledger;
pub mod local;
#[cfg(feature = "proving")]
pub mod prove;
mod structure;
#[cfg(feature = "verifying")]
pub mod verify;

use coin_structure::storage::db::DB;
#[cfg(any(feature = "offer-construction", feature = "proof-verifying"))]
use midnight_onchain_runtime::{ops::Op, result_mode::ResultMode};

#[cfg(any(feature = "offer-construction", feature = "proof-verifying"))]
pub(crate) fn filter_invalid<M: ResultMode<D>, I: Iterator<Item = Op<M, D>>, D: DB>(
    iter: I,
) -> impl Iterator<Item = Op<M, D>> {
    iter.filter(|op| match op {
        Op::Idx { path, .. } => !path.is_empty(),
        Op::Ins { n, .. } => *n != 0,
        _ => true,
    })
}

pub use structure::*;

pub use coin_structure::{self, base_crypto, serialize, storage, transient_crypto};
