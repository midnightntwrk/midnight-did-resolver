#[cfg(test)]
mod tests {
    use midnight_ledger::serialize::test_file_deserialize;
    use midnight_ledger::structure::{LedgerState, ProofPreimage, Transaction};
    use zswap::storage::db::InMemoryDB;

    #[test]
    #[ignore = "storage does not currently support backwards compatibility"]
    fn deserialize_ledger_state() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/LedgerState_4_0.bin");
        let _: LedgerState<InMemoryDB> = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn deserialize_transaction() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Transaction_4_0.bin");
        let _: Transaction<ProofPreimage, InMemoryDB> = test_file_deserialize(path).unwrap();
    }
}
