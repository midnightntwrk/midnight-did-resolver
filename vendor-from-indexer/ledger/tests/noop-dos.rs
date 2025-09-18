#![cfg(feature = "proving")]

use lazy_static::lazy_static;
use midnight_ledger::base_crypto::fab::AlignedValue;
use midnight_ledger::base_crypto::rng::SplittableRng;
use midnight_ledger::construct::{ContractCallPrototype, PreTranscript, partition_transcripts};
use midnight_ledger::serialize::Serializable;
use midnight_ledger::storage::storage::{Array, HashMap};
use midnight_ledger::structure::{
    ContractAction, ContractCalls, ContractDeploy, DUMMY_PARAMETERS, LedgerState, ProvingData,
    StandardTransaction, Transaction,
};
use midnight_ledger::test_utilities::{Resolver, test_resolver, tx_prove};
use midnight_ledger::transient_crypto::proofs::{
    KeyLocation, ProofPreimage as BaseProofPreimage, VerifierKey,
};
use midnight_ledger::verify::WellFormedStrictness;
use onchain_runtime::context::QueryContext;
use onchain_runtime::ops::{Key, Op, key};
use onchain_runtime::program_fragments::*;
use onchain_runtime::result_mode::{ResultModeGather, ResultModeVerify};
use onchain_runtime::state::{ContractOperation, ContractState, StateValue, stval};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, Instant};
use zswap::storage::db::{DB, InMemoryDB};

lazy_static! {
    static ref RESOLVER: Resolver = test_resolver("fallible");
}

async fn verifier_key(name: &'static str) -> Option<VerifierKey> {
    match RESOLVER
        .resolve(&KeyLocation(Cow::Borrowed(name)))
        .await
        .ok()??
    {
        ProvingData::V4(_, vk, _) => Some(vk),
    }
}

fn program_with_results<D: DB>(
    prog: &[Op<ResultModeGather, D>],
    results: &[AlignedValue],
) -> Vec<Op<ResultModeVerify, D>> {
    let mut res_iter = results.iter();
    let res = prog
        .iter()
        .map(|op| op.clone().translate(|()| res_iter.next().unwrap().clone()))
        .filter(|op| match op {
            Op::Idx { path, .. } => !path.is_empty(),
            Op::Ins { n, .. } => *n != 0,
            _ => true,
        })
        .collect::<Vec<_>>();
    res
}

#[tokio::test]
async fn noop_dos() {
    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();

    // Part 1: Deploy
    println!(":: Part 1: Deploy");
    let count_op = ContractOperation::new(verifier_key("count").await);
    let contract = ContractState {
        operations: HashMap::new().insert(b"count"[..].into(), count_op.clone()),
        data: stval!([(0u64), (false), (0u64)]),
        maintenance_authority: Default::default(),
    };
    let (tx, addr) = {
        let deploy = ContractDeploy::new(&mut rng, contract.clone());
        let addr = deploy.address();
        let tx = tx_prove(
            rng.split(),
            &Transaction::from(ContractCalls::new(&mut rng).add_deploy(deploy)),
            &RESOLVER,
        )
        .await
        .unwrap();
        (tx, addr)
    };
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    tx.well_formed(&ledger_state, strictness).unwrap();
    ledger_state = ledger_state.assert_apply(&tx);

    // Part 2: First application
    println!(":: Part 2: First count");
    let guaranteed_public_transcript = partition_transcripts(
        &[PreTranscript {
            context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
            program: &program_with_results::<InMemoryDB>(
                &Counter_increment!([key!(0u8)], false, 1u64),
                &[],
            ),
            comm_comm: None,
        }],
        &DUMMY_PARAMETERS,
    )
    .unwrap()[0]
        .0
        .clone()
        .unwrap();
    let fallible_public_transcript = partition_transcripts(
        &[PreTranscript {
            context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
            program: &program_with_results(
                &[
                    &kernel_checkpoint!((), ())[..],
                    &Cell_read!([key!(1u8)], false, bool),
                    &Cell_write!([key!(1u8)], false, bool, true),
                    &Counter_increment!([key!(2u8)], false, 1u64),
                ]
                .into_iter()
                .flat_map(|x| x.iter())
                .cloned()
                .collect::<Vec<_>>(),
                &[false.into()],
            ),
            comm_comm: None,
        }],
        &DUMMY_PARAMETERS,
    )
    .unwrap()[0]
        .0
        .clone()
        .unwrap();
    let mut tx = {
        let call = ContractCallPrototype {
            address: addr,
            entry_point: b"count"[..].into(),
            op: count_op.clone(),
            input: ().into(),
            output: ().into(),
            guaranteed_public_transcript: Some(guaranteed_public_transcript),
            fallible_public_transcript: Some(fallible_public_transcript),
            private_transcript_outputs: vec![],
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("count")),
        };
        tx_prove(
            rng.split(),
            &Transaction::from(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call)),
            &RESOLVER,
        )
        .await
        .unwrap()
    };
    let mut noop_dos: Vec<Op<ResultModeVerify, InMemoryDB>> = Vec::new();
    for _ in 0..1_000 {
        noop_dos.push(Op::Noop { n: 0x1ffff });
        noop_dos.push(Op::Dup { n: 1 });
        noop_dos.push(Op::Pop);
    }
    match &mut tx {
        Transaction::Standard(StandardTransaction { contract_calls, .. }) => {
            match &mut contract_calls.as_mut().unwrap().calls[0] {
                ContractAction::Call(call) => {
                    call.guaranteed_transcript = Some(
                        partition_transcripts(
                            &[PreTranscript {
                                context: &QueryContext::new(
                                    ledger_state.index(addr).unwrap().data,
                                    addr,
                                ),
                                program: &noop_dos,
                                comm_comm: None,
                            }],
                            &DUMMY_PARAMETERS,
                        )
                        .unwrap()[0]
                            .0
                            .clone()
                            .unwrap(),
                    );
                    dbg!(call);
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
    dbg!(Serializable::serialized_size(&tx));
    let t0 = Instant::now();
    dbg!(tx.well_formed(&ledger_state, strictness)).ok();
    let t1 = Instant::now();
    assert!(t1 - t0 < Duration::from_millis(100));
}
