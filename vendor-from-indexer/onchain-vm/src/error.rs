use crate::base_crypto::fab::{AlignedValue, InvalidBuiltinDecode};
use crate::runtime_state::state::StateValue;
use crate::storage::db::DB;
use derive_where::derive_where;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive_where(Debug, Clone, PartialEq, Eq)]
pub enum OnchainProgramError<D: DB> {
    RanOffStack,
    RanPastProgramEnd,
    ExpectedCell(StateValue<D>),
    Decode(InvalidBuiltinDecode),
    ArithmeticOverflow,
    TooLongForEqual,
    TypeError,
    OutOfGas,
    BoundsExceeded,
    InvalidArgs,
    MissingKey,
    CacheMiss,
    AttemptedArrayDelete,
    ReadMismatch {
        expected: AlignedValue,
        actual: AlignedValue,
    },
}

impl<D: DB> From<Infallible> for OnchainProgramError<D> {
    fn from(err: Infallible) -> OnchainProgramError<D> {
        match err {}
    }
}

impl<D: DB> From<InvalidBuiltinDecode> for OnchainProgramError<D> {
    fn from(err: InvalidBuiltinDecode) -> OnchainProgramError<D> {
        OnchainProgramError::Decode(err)
    }
}

impl<D: DB> Display for OnchainProgramError<D> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use OnchainProgramError::*;
        match self {
            RanOffStack => write!(f, "ran off stack"),
            RanPastProgramEnd => write!(f, "ran past end of program"),
            ExpectedCell(state_value) => {
                let descriptor = match state_value {
                    StateValue::Null => Some("null"),
                    StateValue::Cell(_) => Some("cell"),
                    StateValue::BoundedMerkleTree(_) => Some("bounded Merkle tree"),
                    StateValue::Map(_) => Some("map"),
                    StateValue::Array(_) => Some("array"),
                    _ => None,
                };
                match descriptor {
                    Some(d) => write!(f, "expected a cell, received {d}"),
                    None => write!(f, "bug found: unexpected state value"),
                }
            }
            ArithmeticOverflow => write!(f, "arithmetic overflow"),
            TooLongForEqual => write!(f, "data is too long for equality check"),
            TypeError => write!(f, "invalid operation for type"),
            OutOfGas => write!(f, "ran out of gas budget"),
            BoundsExceeded => write!(f, "exceeded structure bounds"),
            InvalidArgs => write!(f, "invalid argument to primitive operation"),
            MissingKey => write!(f, "key not found"),
            CacheMiss => write!(f, "value declared to be in cache wasn't"),
            AttemptedArrayDelete => write!(f, "attempted to remove from an array type"),
            ReadMismatch { expected, actual } => write!(
                f,
                "mismatch between expected ({expected:?}) and actual ({actual:?}) read"
            ),
            Decode(err) => err.fmt(f),
        }
    }
}

impl<D: DB> Error for OnchainProgramError<D> {}
