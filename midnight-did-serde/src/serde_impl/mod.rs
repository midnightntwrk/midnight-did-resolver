use midnight_did::dlt::ContractStateDeserializer;

mod compact_v08;
mod didcontract_v1;
mod ledger_v4;

#[derive(Debug, Clone)]
pub struct DefaultContractStateDeserializer;

impl ContractStateDeserializer for DefaultContractStateDeserializer {
    fn deserialize(
        &self,
        did: &midnight_did::did::MidnightDid,
        state: midnight_did::dlt::ContractState,
    ) -> Result<identus_did_core::DidDocument, Box<dyn std::error::Error + Send + Sync>> {
        ledger_v4::DidV1ContractStateDeserializer.deserialize(did, state)
    }
}
