#![deny(unreachable_pub)]
#![deny(warnings)]
// Proptest derive triggers this.
#![allow(non_local_definitions)]

#[macro_use]
extern crate tracing;

pub mod context;
pub mod error;
#[rustfmt::skip]
#[path = "../vendored/program_fragments.rs"]
pub mod program_fragments;
pub mod contract_state_ext;
pub mod test_utilities;
pub mod transcript;

pub use coin_structure;
pub use coin_structure::base_crypto;
pub use coin_structure::serialize;
pub use coin_structure::storage;
pub use coin_structure::transient_crypto;

pub use onchain_runtime_state::state;

pub use onchain_vm::cost_model;
pub use onchain_vm::error as vm_error;
pub use onchain_vm::ops;
pub use onchain_vm::result_mode;
pub use onchain_vm::state_value_ext;
pub use onchain_vm::vm;
pub use onchain_vm::vm_value;

use base_crypto::fab::AlignedValue;
use transient_crypto::curve::Fr;
use transient_crypto::hash::transient_commit;

pub fn communication_commitment(input: AlignedValue, output: AlignedValue, rand: Fr) -> Fr {
    transient_commit(&AlignedValue::concat([&input, &output]), rand)
}
