//! Defines the primitives of the field-aligned binary representation, where
//! values are represented as sequences of binary strings, that are tied to an
//! alignment which can be used to interpret them either as binary data, or a
//! sequence of field elements for proving.

mod alignments;
mod conversions;
mod encoding;
mod serialize;

pub use alignments::*;
pub use conversions::InvalidBuiltinDecode;
pub use encoding::*;
pub use serialize::*;
