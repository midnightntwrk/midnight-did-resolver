use onchain_vm::storage::db::DB;

use crate::context::QueryContext;
use crate::cost_model::CostModel;
use crate::error::TranscriptRejected;
use crate::ops::Op;
use crate::result_mode::{GatherEvent, ResultModeGather};
use crate::state::ContractState;
use crate::storage::storage::Map;

pub trait ContractStateExt<D: DB> {
    #[allow(clippy::type_complexity)]
    fn query(
        &self,
        query: &[Op<ResultModeGather, D>],
        cost_model: &CostModel,
    ) -> Result<(ContractState<D>, Vec<GatherEvent<D>>), TranscriptRejected<D>>;
}

impl<D: DB> ContractStateExt<D> for ContractState<D> {
    fn query(
        &self,
        query: &[Op<ResultModeGather, D>],
        cost_model: &CostModel,
    ) -> Result<(ContractState<D>, Vec<GatherEvent<D>>), TranscriptRejected<D>> {
        let qc = QueryContext {
            state: self.data.clone(),
            address: Default::default(),
            effects: Default::default(),
            com_indicies: Map::new(),
            block: Default::default(),
        };
        let res = qc.query(query, None, cost_model)?;
        Ok((
            ContractState {
                data: res.context.state,
                ..self.clone()
            },
            res.events,
        ))
    }
}
