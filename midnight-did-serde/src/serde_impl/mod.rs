use midnight_did::dlt::{ContractState, ContractStateDeserializer};
use midnight_onchain_runtime_v6::state::ContractState as ContractStateV6;
use midnight_serialize_v6::tagged_deserialize as tagged_deserialize_v6;
use midnight_storage_v6::DefaultDB as DefaultDBV6;

#[derive(Debug, Clone)]
pub struct DefaultContractStateDeserializer;

impl ContractStateDeserializer for DefaultContractStateDeserializer {
    fn deserialize(
        &self,
        _did: &midnight_did::did::MidnightDid,
        _state: midnight_did::dlt::ContractState,
    ) -> Result<identus_did_core::DidDocument, Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }
}

pub fn _deserialize_contract_state_v6(contract_state: ContractState) -> std::io::Result<ContractStateV6<DefaultDBV6>> {
    let bytes = contract_state.inner().to_bytes();
    tagged_deserialize_v6(&mut bytes.as_slice())
}
