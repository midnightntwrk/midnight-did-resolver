use onchain_vm::storage::db::DB;

use crate::base_crypto::fab::InvalidBuiltinDecode;
use crate::vm_error::OnchainProgramError;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum TranscriptRejected<D: DB> {
    Execution(OnchainProgramError<D>),
    Decode(InvalidBuiltinDecode),
    FinalStackWrongLength,
    WeakStateReturned,
    EffectDecodeError,
}

impl<D: DB> From<OnchainProgramError<D>> for TranscriptRejected<D> {
    fn from(err: OnchainProgramError<D>) -> Self {
        TranscriptRejected::Execution(err)
    }
}

impl<D: DB> From<InvalidBuiltinDecode> for TranscriptRejected<D> {
    fn from(err: InvalidBuiltinDecode) -> Self {
        TranscriptRejected::Decode(err)
    }
}

impl<D: DB> Display for TranscriptRejected<D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use TranscriptRejected::*;
        match self {
            FinalStackWrongLength => write!(formatter, "final stack must be of length 3"),
            WeakStateReturned => write!(
                formatter,
                "result state is was weak, and cannot be persisted"
            ),
            EffectDecodeError => write!(formatter, "failed to decode effect object"),
            Execution(err) => err.fmt(formatter),
            Decode(err) => err.fmt(formatter),
        }
    }
}

impl<D: DB> Error for TranscriptRejected<D> {}
