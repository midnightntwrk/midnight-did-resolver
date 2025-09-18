use crate::base_crypto::fab::AlignedValue;
use crate::error::OnchainProgramError;
use crate::runtime_state::state::StateValue;
use crate::serialize::{Deserializable, Serializable};
use derive_where::derive_where;
use runtime_state::storage::db::DB;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait ResultMode<D: DB>: Clone + Debug {
    type ReadResult: Eq + PartialEq + Clone + Debug + Serializable + Deserializable;
    type Event;
    fn process_read(
        result: &Self::ReadResult,
        real: &AlignedValue,
    ) -> Result<Option<Self::Event>, OnchainProgramError<D>>;
    fn process_log(event: &StateValue<D>) -> Option<Self::Event>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResultModeVerify;

impl<D: DB> ResultMode<D> for ResultModeVerify {
    type ReadResult = AlignedValue;
    type Event = ();
    fn process_read(
        expected: &Self::ReadResult,
        actual: &AlignedValue,
    ) -> Result<Option<Self::Event>, OnchainProgramError<D>> {
        if expected != actual {
            Err(OnchainProgramError::ReadMismatch {
                expected: expected.clone(),
                actual: actual.clone(),
            })
        } else {
            Ok(None)
        }
    }
    fn process_log(_event: &StateValue<D>) -> Option<Self::Event> {
        None
    }
}

#[derive_where(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        rename_all = "camelCase",
        tag = "tag",
        content = "content",
        bound(
            serialize = "StateValue<D>: Serialize",
            deserialize = "StateValue<D>: Deserialize<'de>"
        )
    )
)]
pub enum GatherEvent<D: DB> {
    Read(AlignedValue),
    Log(StateValue<D>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResultModeGather;

impl<D: DB> ResultMode<D> for ResultModeGather {
    type ReadResult = ();
    type Event = GatherEvent<D>;
    fn process_read(
        (): &Self::ReadResult,
        real: &AlignedValue,
    ) -> Result<Option<Self::Event>, OnchainProgramError<D>> {
        Ok(Some(GatherEvent::Read(real.clone())))
    }
    fn process_log(event: &StateValue<D>) -> Option<Self::Event> {
        Some(GatherEvent::Log(event.clone()))
    }
}
