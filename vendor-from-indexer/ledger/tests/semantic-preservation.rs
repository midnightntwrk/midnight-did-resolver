use lazy_static::lazy_static;
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use midnight_ledger::base_crypto::fab::AlignedValue;
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use midnight_ledger::base_crypto::rng::SplittableRng;
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use midnight_ledger::construct::{ContractCallPrototype, PreTranscript, partition_transcripts};
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use midnight_ledger::onchain_runtime::{
    HistoricMerkleTree_insert,
    context::QueryContext,
    ops::{Key, Op, key},
    state::{ContractOperation, ContractState, StateValue, stval},
};
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use midnight_ledger::serialize::serialize;
use midnight_ledger::serialize::{NetworkId, deserialize};
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use midnight_ledger::storage::storage::{Array, HashMap};
#[cfg(all(feature = "transaction-construction", feature = "proving"))]
use midnight_ledger::structure::{ContractCalls, ContractDeploy, DUMMY_PARAMETERS};
use midnight_ledger::structure::{LedgerState, Proof, Transaction};
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use midnight_ledger::test_utilities::tx_prove;
use midnight_ledger::test_utilities::{Resolver, test_resolver};
#[cfg(all(feature = "transaction-construction", feature = "proving"))]
use midnight_ledger::transient_crypto::fab::ValueReprAlignedValue;
#[cfg(all(feature = "transaction-construction", feature = "proving"))]
use midnight_ledger::transient_crypto::merkle_tree::{MerkleTree, leaf_hash};
#[cfg(all(feature = "transaction-construction", feature = "proving"))]
use midnight_ledger::transient_crypto::proofs::KeyLocation;
#[cfg(all(feature = "transaction-construction", feature = "proving"))]
use midnight_ledger::transient_crypto::proofs::ProofPreimage as BaseProofPreimage;
use midnight_ledger::verify::WellFormedStrictness;
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use rand::{Rng, SeedableRng, rngs::StdRng};
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use std::borrow::Cow;
use std::fs::File;
use std::path::Path;
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
use std::sync::Arc;
use zswap::storage::db::InMemoryDB;

lazy_static! {
    static ref RESOLVER: Resolver = test_resolver("simple-merkle-tree");
}

#[tokio::test]
#[ignore = "run only to regenerate test files"]
#[cfg(all(feature = "transaction-construction", feature = "proving",))]
async fn regenerate() {
    // Piggy-backing off the merkle tree test

    use midnight_ledger::test_utilities::verifier_key;
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
    let mut f = File::create("tests/semantic-preservation-1.tx").unwrap();
    serialize(&tx, &mut f, NetworkId::TestNet).unwrap();
    drop(f);
    ledger_state = ledger_state.assert_apply(&tx);
    let mut f = File::create("tests/semantic-preservation-1.st").unwrap();
    serialize(&ledger_state, &mut f, NetworkId::TestNet).unwrap();
    drop(f);

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

    let mut f = File::create("tests/semantic-preservation-2.tx").unwrap();
    serialize(&tx, &mut f, NetworkId::TestNet).unwrap();
    drop(f);
    ledger_state = ledger_state.assert_apply(&tx);
    let mut f = File::create("tests/semantic-preservation-2.st").unwrap();
    serialize(&ledger_state, &mut f, NetworkId::TestNet).unwrap();
    drop(f);
}

fn fopen<P: AsRef<Path>>(inp: P) -> File {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(inp);
    dbg!(&path);
    File::open(path).unwrap()
}

#[test]
#[ignore = "storage does not currently support backwards compatibility"]
fn test_deploy_well_formed() {
    let ledger_state: LedgerState<InMemoryDB> = LedgerState::new();
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let deploy: Transaction<Proof, InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-1.tx"),
        NetworkId::TestNet,
    )
    .unwrap();
    deploy.well_formed(&ledger_state, strictness).unwrap();
}

#[test]
#[ignore = "storage does not currently support backwards compatibility"]
fn test_deploy_semantics() {
    let ledger_state: LedgerState<InMemoryDB> = LedgerState::new();
    let ledger_state2: LedgerState<InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-1.st"),
        NetworkId::TestNet,
    )
    .unwrap();
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let deploy: Transaction<Proof, InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-1.tx"),
        NetworkId::TestNet,
    )
    .unwrap();
    assert_eq!(ledger_state.assert_apply(&deploy), ledger_state2);
}

#[test]
#[ignore = "storage does not currently support backwards compatibility"]
fn test_call_well_formed() {
    let ledger_state: LedgerState<InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-1.st"),
        NetworkId::TestNet,
    )
    .unwrap();
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let call: Transaction<Proof, InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-2.tx"),
        NetworkId::TestNet,
    )
    .unwrap();
    call.well_formed(&ledger_state, strictness).unwrap();
}

#[test]
#[ignore = "storage does not currently support backwards compatibility"]
fn test_call_semantics() {
    let ledger_state: LedgerState<InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-1.st"),
        NetworkId::TestNet,
    )
    .unwrap();
    let ledger_state2: LedgerState<InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-2.st"),
        NetworkId::TestNet,
    )
    .unwrap();
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let call: Transaction<Proof, InMemoryDB> = deserialize(
        fopen("tests/semantic-preservation-2.tx"),
        NetworkId::TestNet,
    )
    .unwrap();
    assert_eq!(ledger_state.assert_apply(&call), ledger_state2);
}
