use identus_did_core::{DidDocument, DidDocumentMetadata};
use midnight_did::did::MidnightDid;
use midnight_did::dlt::ContractStateDeserializer;

mod compact_v0_8;
mod didcontract_v1;

#[derive(Debug, Clone)]
pub struct DefaultContractStateDeserializer;

impl ContractStateDeserializer for DefaultContractStateDeserializer {
    fn deserialize(
        &self,
        did: &MidnightDid,
        state: &midnight_did::dlt::ContractState,
    ) -> Result<(DidDocumentMetadata, DidDocument), Box<dyn std::error::Error + Send + Sync>> {
        didcontract_v1::DidContractDeserializer.deserialize(did, state)
    }
}
