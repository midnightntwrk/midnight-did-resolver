use onchain_vm::storage::db::DB;

use crate::cost_model::DUMMY_COST_MODEL;
use crate::ops::*;
use crate::result_mode::*;
use crate::vm_error::OnchainProgramError;
use crate::vm_value::VmValue;

#[allow(clippy::type_complexity)]
pub fn run_program<D: DB>(
    initial: &[VmValue<D>],
    program: &[Op<ResultModeGather, D>],
) -> Result<(Vec<VmValue<D>>, Vec<GatherEvent<D>>), OnchainProgramError<D>> {
    let res = crate::vm::run_program(initial, program, None, &DUMMY_COST_MODEL)?;
    Ok((res.stack, res.events))
}
