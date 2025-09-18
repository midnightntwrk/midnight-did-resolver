#![deny(unreachable_pub)]
#![deny(warnings)]
// Proptest derive triggers this.
#![allow(non_local_definitions)]

pub mod state;

pub use coin_structure;
pub use coin_structure::base_crypto;
pub use coin_structure::serialize;
pub use coin_structure::storage;
pub use coin_structure::transient_crypto;
