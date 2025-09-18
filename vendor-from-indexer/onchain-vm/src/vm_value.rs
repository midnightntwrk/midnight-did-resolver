use crate::base_crypto::fab::AlignedValue;
use crate::error::OnchainProgramError;
use crate::runtime_state::state::StateValue;
use crate::state_value_ext::*;
use derive_where::derive_where;
use std::fmt::Debug;
use std::ops::BitAnd;
use std::sync::Arc;

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ValueStrength {
    Weak,
    Strong,
}

use ValueStrength::*;
use runtime_state::storage::db::DB;

impl BitAnd for ValueStrength {
    type Output = ValueStrength;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Strong, Strong) => Strong,
            _ => Weak,
        }
    }
}

impl Debug for ValueStrength {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Weak => write!(formatter, "#"),
            Strong => Ok(()),
        }
    }
}

#[derive_where(Eq, PartialEq, Clone)]
pub struct VmValue<D: DB> {
    pub strength: ValueStrength,
    pub value: StateValue<D>,
}

impl<D: DB> VmValue<D> {
    pub(crate) fn as_cell(&self) -> Result<Arc<AlignedValue>, OnchainProgramError<D>> {
        self.value.as_cell()
    }

    pub(crate) fn as_cell_ref(&self) -> Result<&AlignedValue, OnchainProgramError<D>> {
        self.value.as_cell_ref()
    }
}

impl<D: DB> Debug for VmValue<D> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}{:?}", self.strength, self.value)
    }
}

impl<D: DB> VmValue<D> {
    pub fn new(strength: ValueStrength, value: StateValue<D>) -> Self {
        VmValue { strength, value }
    }
}

#[macro_export]
macro_rules! vmval {
    (# $($val:tt)*) => {
        VmValue {
            strength: ValueStrength::Weak,
            value: stval!($($val)*),
        }
    };
    ($($val:tt)*) => {
        VmValue {
            strength: ValueStrength::Strong,
            value: stval!($($val)*),
        }
    };
}
