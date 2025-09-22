use midnight_did::dlt::ContractStateDeserializer;
use midnight_ledger_v4::onchain_runtime::state::{self, ContractState};
use midnight_ledger_v4::serialize::{NetworkId, deserialize};
use midnight_ledger_v4::storage::DefaultDB;

use crate::serde_impl::didcontract_v1::DidContract;

pub struct DidV1ContractStateDeserializer;

impl ContractStateDeserializer for DidV1ContractStateDeserializer {
    fn deserialize(
        &self,
        did: &midnight_did::did::MidnightDid,
        state: midnight_did::dlt::ContractState,
    ) -> Result<identus_did_core::DidDocument, Box<dyn std::error::Error + Send + Sync>> {
        let network_id = match did.network() {
            midnight_did::did::MidnightNetwork::Undeployed => NetworkId::Undeployed,
            midnight_did::did::MidnightNetwork::Devnet => NetworkId::DevNet,
            midnight_did::did::MidnightNetwork::Testnet => NetworkId::TestNet,
            midnight_did::did::MidnightNetwork::Mainnet => NetworkId::MainNet,
        };
        let bytes = state.inner().to_bytes();
        let contract_state: ContractState<DefaultDB> = deserialize(bytes.as_slice(), network_id)?;
        let state_value = contract_state.data;

        todo!()
    }
}
