#![deny(unreachable_pub)]
#![deny(warnings)]
// Proptest derive triggers this.
#![allow(non_local_definitions)]

mod deserializable;
mod serializable;
mod util;
mod versioned;

pub use crate::deserializable::{Deserializable, RECURSION_LIMIT, deserialize};
pub use crate::serializable::{Serializable, serialize, serialized_size};
pub use crate::util::{
    NetworkId, ReadExt, VecExt, check_injected_version, gen_static_serialize_file,
    test_file_deserialize,
};
#[cfg(feature = "proptest")]
pub use crate::util::{NoSearch, NoStrategy};
pub use crate::versioned::{Version, Versioned};
pub use macros::{Deserializable, Serializable, Versioned};
