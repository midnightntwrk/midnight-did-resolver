use lazy_static::lazy_static;
use midnight_ledger::base_crypto::fab::AlignedValue;
use midnight_ledger::base_crypto::rng::SplittableRng;
use midnight_ledger::construct::{ContractCallPrototype, PreTranscript, partition_transcripts};
use midnight_ledger::storage::storage::{Array, HashMap};
use midnight_ledger::structure::{
    ContractCalls, ContractDeploy, DUMMY_PARAMETERS, LedgerState, Transaction,
};
use midnight_ledger::test_utilities::{Resolver, test_resolver, tx_prove, verifier_key};
use midnight_ledger::transient_crypto::fab::ValueReprAlignedValue;
use midnight_ledger::transient_crypto::merkle_tree::{MerkleTree, leaf_hash};
use midnight_ledger::transient_crypto::proofs::{KeyLocation, ProofPreimage as BaseProofPreimage};
use midnight_ledger::verify::WellFormedStrictness;
use onchain_runtime::context::QueryContext;
use onchain_runtime::ops::{Key, Op, key};
use onchain_runtime::program_fragments::{
    HistoricMerkleTree_check_root, HistoricMerkleTree_insert,
};
use onchain_runtime::result_mode::{ResultModeGather, ResultModeVerify};
use onchain_runtime::state::{ContractOperation, ContractState, StateValue, stval};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::borrow::Cow;
use std::sync::Arc;
use zswap::storage::db::{DB, InMemoryDB};

lazy_static! {
    static ref RESOLVER: Resolver = test_resolver("simple-merkle-tree");
}

fn program_with_results<D: DB>(
    prog: &[Op<ResultModeGather, D>],
    results: &[AlignedValue],
) -> Vec<Op<ResultModeVerify, D>> {
    let mut res_iter = results.iter();
    prog.iter()
        .map(|op| op.clone().translate(|()| res_iter.next().unwrap().clone()))
        .collect()
}

#[tokio::test]
#[allow(unused_assignments, clippy::redundant_clone)]
async fn simple_merkle_tree() {
    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();

    // Part 1: Deploy
    let root = MerkleTree::<()>::blank(10).root();
    let store_op = ContractOperation::new(verifier_key(&RESOLVER, "store").await);
    let check_op = ContractOperation::new(verifier_key(&RESOLVER, "check").await);
    let contract = ContractState {
        data: stval!([[{MT(10) {}}, (0u64), {root => null}]]),
        operations: HashMap::new()
            .insert(b"store"[..].into(), store_op.clone())
            .insert(b"check"[..].into(), check_op.clone()),
        maintenance_authority: Default::default(),
    };
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let (tx, addr) = {
        // Create partial deploy tx
        let deploy = ContractDeploy::new(&mut rng, contract);
        let addr = deploy.address();
        let tx = tx_prove(
            rng.split(),
            &Transaction::from(ContractCalls::new(&mut rng).add_deploy(deploy)),
            &RESOLVER,
        )
        .await
        .unwrap();
        tx.well_formed(&ledger_state, strictness).unwrap();
        (tx, addr)
    };
    ledger_state = ledger_state.assert_apply(&tx);
    assert!(ledger_state.index(addr).is_some());

    // Part 2: Store 1
    let entry1 = 12u32;
    let tx = {
        let transcripts = partition_transcripts(
            &[PreTranscript {
                context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
                program: &HistoricMerkleTree_insert!([key!(0u8)], false, 10, u32, entry1.clone()),
                comm_comm: None,
            }],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        let call = ContractCallPrototype {
            address: addr,
            entry_point: b"store"[..].into(),
            op: store_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![],
            input: entry1.into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("store")),
        };
        let pre_tx =
            Transaction::from(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call));
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        tx.well_formed(&ledger_state, strictness).unwrap();
        tx
    };
    // dbg!(&tx);
    ledger_state = ledger_state.assert_apply(&tx);

    // Part 2: Check the path.
    let contract_state = ledger_state.index(addr).unwrap();
    let composite_tree_var = if let StateValue::Array(arr) = &contract_state.data {
        &arr[0]
    } else {
        unreachable!()
    };
    let real_tree_var = if let StateValue::Array(arr) = composite_tree_var {
        &arr[0]
    } else {
        unreachable!()
    };
    let path = if let StateValue::BoundedMerkleTree(tree) = real_tree_var {
        tree.find_path_for_leaf(entry1).unwrap()
    } else {
        unreachable!()
    };
    let tx_check = {
        let mut transcripts = partition_transcripts(
            &[PreTranscript {
                context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
                program: &program_with_results(
                    &HistoricMerkleTree_check_root!([key!(0u8)], false, 10, u32, path.root()),
                    &[true.into()],
                ),
                comm_comm: None,
            }],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        if let Some(ref mut transcript) = transcripts[0].0 {
            transcript.gas += transcript.gas / 5;
        }
        let call = ContractCallPrototype {
            address: addr,
            entry_point: b"check"[..].into(),
            op: check_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![path.into()],
            input: entry1.into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("check")),
        };
        let pre_tx =
            Transaction::from(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call));
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        tx.well_formed(&ledger_state, strictness).unwrap();
        tx
    };
    // dbg!(&tx_check);
    ledger_state = ledger_state.assert_apply(&tx_check);

    // Part 3: Another insert
    let entry2 = 42u32;
    let transcripts = partition_transcripts(
        &[PreTranscript {
            context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
            program: &HistoricMerkleTree_insert!([key!(0u8)], false, 10, u32, entry2.clone()),
            comm_comm: None,
        }],
        &DUMMY_PARAMETERS,
    )
    .unwrap();
    let tx = {
        let call = ContractCallPrototype {
            address: addr,
            entry_point: b"store"[..].into(),
            op: store_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![],
            input: entry2.into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("store")),
        };
        let pre_tx =
            Transaction::from(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call));
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        tx.well_formed(&ledger_state, strictness).unwrap();
        tx
    };
    // dbg!(&tx);
    ledger_state = ledger_state.assert_apply(&tx);

    // Part 4: Old check should work.
    ledger_state = ledger_state.assert_apply(&tx_check);
}
