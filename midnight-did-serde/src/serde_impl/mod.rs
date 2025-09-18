use midnight_did::dlt::ContractStateDeserializer;

use crate::serde_impl::v6::ContractStateDeserializerV6;

#[derive(Debug, Clone)]
pub struct DefaultContractStateDeserializer;

impl ContractStateDeserializer for DefaultContractStateDeserializer {
    fn deserialize(
        &self,
        did: &midnight_did::did::MidnightDid,
        state: midnight_did::dlt::ContractState,
    ) -> Result<identus_did_core::DidDocument, Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }
}

