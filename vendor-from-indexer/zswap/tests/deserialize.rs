#[cfg(test)]
mod tests {
    use coin_structure::storage::db::InMemoryDB;
    use midnight_zswap::Offer;
    use midnight_zswap::ledger::State as LedgerState;
    use midnight_zswap::local::State as LocalState;
    use midnight_zswap::serialize::test_file_deserialize;

    #[test]
    fn deserialize_offer() {
        //let mut rng = thread_rng();
        //let offer: Offer<()> = Offer {
        //    inputs: vec![Input {
        //        nullifier: Nullifier(rng.gen()),
        //        value_commitment: Pedersen::blinding_component(&mut rng).0,
        //        contract_address: Some(coin_structure::contract::Address(rng.gen())),
        //        merkle_tree_root: MerkleTreeDigest(rng.gen()),
        //        proof: (),
        //    }],
        //    outputs: vec![Output {
        //        coin_com: Commitment(rng.gen()),
        //        value_commitment: Pedersen::blinding_component(&mut rng).0,
        //        contract_address: None,
        //        ciphertext: Some(CoinCiphertext {
        //            c: EmbeddedGroupAffine::generator() * rng.gen::<EmbeddedFr>(),
        //            ciph: rng.gen(),
        //        }),
        //        proof: (),
        //    }],
        //    transient: vec![Transient {
        //        nullifier: Nullifier(rng.gen()),
        //        coin_com: Commitment(rng.gen()),
        //        value_commitment_input: Pedersen::blinding_component(&mut rng).0,
        //        value_commitment_output: Pedersen::blinding_component(&mut rng).0,
        //        contract_address: Some(coin_structure::contract::Address(rng.gen())),
        //        ciphertext: None,
        //        proof_input: (),
        //        proof_output: (),
        //    }],
        //    deltas: vec![(NATIVE_TOKEN, -500)],
        //};
        //gen_static_serialize_file(&offer).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Offer_4_0.bin");
        let _: Offer<()> = test_file_deserialize(path).unwrap();
    }

    #[test]
    #[ignore = "storage does not currently support backwards compatibility"]
    fn deserialize_ledger_state() {
        //let mut rng = thread_rng();
        //let st: LedgerState = LedgerState {
        //    coin_coms: MerkleTree::blank(32)
        //        .update_hash(0, rng.gen(), None)
        //        .update_hash(1, rng.gen(), None)
        //        .update_hash(2, rng.gen(), None)
        //        .update_hash(3, rng.gen(), None)
        //        .update_hash(
        //            4,
        //            rng.gen(),
        //            Some(coin_structure::contract::Address(rng.gen())),
        //        ),
        //    coin_coms_set: vec![(Commitment(rng.gen()), ()); 5].into_iter().collect(),
        //    first_free: 5,
        //    nullifiers: vec![(Nullifier(rng.gen()), ()); 5].into_iter().collect(),
        //    past_roots: vec![(MerkleTreeDigest(rng.gen()), ()); 5]
        //        .into_iter()
        //        .collect(),
        //};
        //gen_static_serialize_file(&st).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/LedgerState_4_0.bin");
        let _: LedgerState<InMemoryDB> = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn deserialize_local_state() {
        //let mut rng = thread_rng();
        //let st: LocalState = LocalState::new();
        //gen_static_serialize_file(&st).unwrap();
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/LocalState_5_0.bin");
        let _: LocalState<InMemoryDB> = test_file_deserialize(path).unwrap();
    }
}
