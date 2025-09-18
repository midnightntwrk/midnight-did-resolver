use midnight_base_crypto_v6::fab::Value;
use midnight_did::dlt::ContractStateDeserializer;
use midnight_onchain_runtime_v6::state::{ContractState, StateValue};
use midnight_serialize_v6::tagged_deserialize;
use midnight_storage_v6::DefaultDB;
use midnight_storage_v6::storage::Array;

pub struct ContractStateDeserializerV6;

impl ContractStateDeserializer for ContractStateDeserializerV6 {
    fn deserialize(
        &self,
        _did: &midnight_did::did::MidnightDid,
        state: midnight_did::dlt::ContractState,
    ) -> Result<identus_did_core::DidDocument, Box<dyn std::error::Error + Send + Sync>> {
        let bytes = state.inner().to_bytes();
        let contract_state: ContractState<DefaultDB> = tagged_deserialize(bytes.as_slice())?;
        let charged_state = contract_state.data;
        let state_value = &*charged_state.get();

        match *&state_value {
            StateValue::Null => todo!(),
            StateValue::Cell(sp) => todo!(),
            StateValue::Map(hash_map) => todo!(),
            StateValue::Array(array) => todo!(),
            StateValue::BoundedMerkleTree(merkle_tree) => todo!(),
            _ => todo!(),
        }

        let state_value: StateValue = StateValue::Array(Array::new_from_slice(&[StateValue::Null, StateValue::Null]));
        // let json = serde_json::to_value(state_value)?;
        let json_str = serde_json::to_string_pretty(&state_value).unwrap();
        println!("---");
        println!("{}", json_str);

        todo!("implement")
    }
}

struct CompactError(String);

trait CompactType: Sized {
    fn from_value(value: Value) -> Result<Self, CompactError>;
}

struct CompactTypeBoolean(bool);
struct CompactTypeBytes<const N: usize>([u8; N]);

impl CompactType for CompactTypeBoolean {
    fn from_value(mut value: Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop().map(|i| i.0);
        let Some(val) = maybe_val else {
            Err(CompactError("expected Boolean".to_string()))?
        };
        if val.len() > 1 || (val.len() == 1 && val[0] != 1) {
            Err(CompactError("expected Boolean".to_string()))?
        }
        Ok(Self(val.len() == 1))
    }
}

impl<const N: usize> CompactType for CompactTypeBytes<N> {
    fn from_value(mut value: Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop().map(|i| i.0);
        let Some(val) = maybe_val else {
            Err(CompactError(format!("expected Bytes[{N}]")))?
        };
        let Ok(array) = val.try_into() else {
            Err(CompactError(format!("expected Bytes[{N}]")))?
        };
        Ok(Self(array))
    }
}
