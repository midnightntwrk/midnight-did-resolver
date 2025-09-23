#[cfg(test)]
mod tests {
    use coin_structure::serialize::RECURSION_LIMIT;
    use midnight_onchain_runtime::serialize::{
        NetworkId, deserialize, serialize, test_file_deserialize,
    };
    use midnight_onchain_runtime::state::{ContractState, StateValue};
    use midnight_onchain_runtime::transcript::Transcript;
    use onchain_vm::ops::*;
    use onchain_vm::result_mode::*;
    use onchain_vm::storage::db::InMemoryDB;

    #[test]
    fn deserialize_transcript() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Transcript_2_0.bin");
        let _: Transcript<InMemoryDB> = test_file_deserialize(path).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Transcript_2_1.bin");
        let _: Transcript<InMemoryDB> = test_file_deserialize(path).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Transcript_2_2.bin");
        let _: Transcript<InMemoryDB> = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn deserialize_ops() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Ops_2_0.bin");
        let _: Op<ResultModeGather, InMemoryDB> = test_file_deserialize(path).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Ops_2_1.bin");
        let _: Op<ResultModeGather, InMemoryDB> = test_file_deserialize(path).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Ops_2_2.bin");
        let _: Op<ResultModeGather, InMemoryDB> = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn deserialize_state_value() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/StateValue_2_0.bin");
        let _: StateValue<InMemoryDB> = test_file_deserialize(path).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/StateValue_2_1.bin");
        let _: StateValue<InMemoryDB> = test_file_deserialize(path).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/StateValue_2_2.bin");
        let _: StateValue<InMemoryDB> = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn deserialize_contract_state() {
        //let state: ContractState = ContractState::default();
        //gen_static_serialize_file(&state).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/ContractState_3_0.bin");
        let _: ContractState<InMemoryDB> = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn massive_recursive_state_value_deserialization() {
        fn state_value(n: u32) -> StateValue<InMemoryDB> {
            let mut statevalue = StateValue::Null;
            for _ in 0..n {
                statevalue = StateValue::Array(vec![statevalue].into());
            }

            statevalue
        }

        let mut bytes: Vec<u8> = Vec::new();
        {
            let state_value = state_value(RECURSION_LIMIT + 1);
            assert!(serialize(&state_value, &mut bytes, NetworkId::Undeployed).is_ok());
        }
        let result: Result<StateValue<InMemoryDB>, std::io::Error> =
            deserialize(bytes.as_slice(), NetworkId::Undeployed);
        assert!(result.is_err())
    }
}
