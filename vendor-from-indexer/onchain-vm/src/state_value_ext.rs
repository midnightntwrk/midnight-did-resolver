use runtime_state::storage::db::DB;

use crate::base_crypto::fab::AlignedValue;
use crate::error::OnchainProgramError;
use crate::runtime_state::state::StateValue;
use std::sync::Arc;

pub trait StateValueExt<D: DB> {
    fn as_cell(&self) -> Result<Arc<AlignedValue>, OnchainProgramError<D>>;
    fn as_cell_ref(&self) -> Result<&AlignedValue, OnchainProgramError<D>>;
}

impl<D: DB> StateValueExt<D> for StateValue<D> {
    fn as_cell(&self) -> Result<Arc<AlignedValue>, OnchainProgramError<D>> {
        match self {
            StateValue::Cell(value) => Ok(value.clone()),
            _ => Err(OnchainProgramError::ExpectedCell(self.clone())),
        }
    }

    fn as_cell_ref(&self) -> Result<&AlignedValue, OnchainProgramError<D>> {
        match self {
            StateValue::Cell(value) => Ok(value),
            _ => Err(OnchainProgramError::ExpectedCell(self.clone())),
        }
    }
}
