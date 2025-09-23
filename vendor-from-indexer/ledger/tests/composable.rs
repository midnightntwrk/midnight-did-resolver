use lazy_static::lazy_static;
use midnight_ledger::base_crypto::fab::AlignedValue;
use midnight_ledger::base_crypto::hash::{HashOutput, persistent_commit};
use midnight_ledger::base_crypto::rng::SplittableRng;
use midnight_ledger::coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN};
use midnight_ledger::coin_structure::transfer::{Recipient, SenderEvidence};
use midnight_ledger::construct::{ContractCallPrototype, PreTranscript, partition_transcripts};
use midnight_ledger::error::{MalformedTransaction, TransactionInvalid};
use midnight_ledger::semantics::TransactionResult;
use midnight_ledger::storage::storage::{Array, HashMap};
#[cfg(feature = "proving")]
use midnight_ledger::structure::ProvingData;
use midnight_ledger::structure::{
    ContractCalls, ContractDeploy, DUMMY_PARAMETERS, LedgerState, Transaction,
};
#[cfg(feature = "proving")]
use midnight_ledger::test_utilities::PUBLIC_PARAMS;
use midnight_ledger::test_utilities::{Resolver, test_resolver, tx_prove, verifier_key};
use midnight_ledger::transient_crypto::fab::ValueReprAlignedValue;
use midnight_ledger::transient_crypto::hash::transient_commit;
use midnight_ledger::transient_crypto::proofs::{KeyLocation, ProofPreimage as BaseProofPreimage};
use midnight_ledger::verify::WellFormedStrictness;
use midnight_ledger::zswap::{Offer, Output, Transient};
use onchain_runtime::context::QueryContext;
use onchain_runtime::error::TranscriptRejected;
use onchain_runtime::ops::{Key, Op, key};
use onchain_runtime::program_fragments::*;
use onchain_runtime::result_mode::{ResultModeGather, ResultModeVerify};
use onchain_runtime::state::{ContractOperation, ContractState, StateValue, stval};
use onchain_runtime::vm_error::OnchainProgramError;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::borrow::Cow;
use std::sync::Arc;
use zswap::storage::db::{DB, InMemoryDB};

lazy_static! {
    static ref RESOLVER_INNER: Resolver = test_resolver("composable-inner");
    static ref RESOLVER_OUTER: Resolver = test_resolver("composable-outer");
    static ref RESOLVER_RELAY: Resolver = test_resolver("composable-relay");
    static ref RESOLVER_BURN: Resolver = test_resolver("composable-burn");
}

#[cfg(feature = "proving")]
async fn resolve_any(key: KeyLocation) -> std::io::Result<Option<ProvingData>> {
    let resolvers = [
        &*RESOLVER_INNER,
        &*RESOLVER_OUTER,
        &*RESOLVER_RELAY,
        &*RESOLVER_BURN,
    ];
    for resolver in resolvers.into_iter() {
        if let Some(res) = resolver.resolve(&key).await? {
            return Ok(Some(res));
        }
    }
    Ok(None)
}

#[cfg(feature = "proving")]
lazy_static! {
    static ref RESOLVER: Resolver = Resolver::new(
        PUBLIC_PARAMS.clone(),
        Box::new(|inp| Box::pin(resolve_any(inp))),
    );
}
//    .into_iter()
//    .flatten()
//    .map(|(k, v)| (*k, v.clone()))
//    .collect();
#[cfg(not(feature = "proving"))]
lazy_static! {
    static ref RESOLVER: Resolver = ();
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
async fn composable() {
    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();

    // Part 1: Deploy inner
    println!(":: Part 1: Deploy inner");
    let get_op = ContractOperation::new(verifier_key(&RESOLVER, "get").await);
    let set_op = ContractOperation::new(verifier_key(&RESOLVER, "set").await);
    let auth_sk: HashOutput = rng.gen();
    let auth_pk = persistent_commit(b"mdn:ex:ci", auth_sk);
    let contract = ContractState {
        operations: HashMap::new()
            .insert(b"get"[..].into(), get_op.clone())
            .insert(b"set"[..].into(), set_op.clone()),
        data: stval!([(auth_pk), (b"hello".to_vec())]),
        maintenance_authority: Default::default(),
    };
    let (tx, addr_inner) = {
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

    // Part 2: Deploy outer
    println!(":: Part 2: Deploy outer");
    let ep_hash = persistent_commit(
        b"get",
        HashOutput(*b"midnight:entry-point\0\0\0\0\0\0\0\0\0\0\0\0"),
    );
    let update_op = ContractOperation::new(verifier_key(&RESOLVER, "update").await);
    let contract = ContractState {
        operations: HashMap::new().insert(b"update"[..].into(), update_op.clone()),
        data: stval!([(b"".to_vec()), (addr_inner), (ep_hash)]),
        maintenance_authority: Default::default(),
    };
    let (tx, addr_outer) = {
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

    // Part 3: (Golden) run update
    println!(":: Part 3: (Golden) run update");
    let tx = {
        let cc_rand = rng.gen();
        let cc = transient_commit(&ValueReprAlignedValue(b"hello".to_vec().into()), cc_rand);
        let transcripts = partition_transcripts(
            &[
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_inner).unwrap().data,
                        addr_inner,
                    ),
                    program: &program_with_results(
                        &Cell_read!([key!(1u8)], false, Vec<u8>),
                        &[b"hello".to_vec().into()],
                    ),
                    comm_comm: Some(cc),
                },
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_outer).unwrap().data,
                        addr_outer,
                    ),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, HashOutput)[..],
                            &Cell_read!([key!(2u8)], false, HashOutput),
                            &kernel_claim_contract_call!(
                                (),
                                (),
                                AlignedValue::from(addr_inner.0),
                                AlignedValue::from(ep_hash),
                                AlignedValue::from(cc)
                            ),
                            &Cell_write!([key!(0u8)], false, Vec<u8>, b"hello".to_vec()),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[addr_inner.into(), ep_hash.into()],
                    ),
                    comm_comm: None,
                },
            ],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        let call_inner = ContractCallPrototype {
            address: addr_inner,
            entry_point: b"get"[..].into(),
            op: get_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![],
            input: ().into(),
            output: b"hello".to_vec().into(),
            communication_commitment_rand: cc_rand,
            key_location: KeyLocation(Cow::Borrowed("get")),
        };
        let call_outer = ContractCallPrototype {
            address: addr_outer,
            entry_point: b"update"[..].into(),
            op: update_op.clone(),
            guaranteed_public_transcript: transcripts[1].0.clone(),
            fallible_public_transcript: transcripts[1].1.clone(),
            private_transcript_outputs: vec![b"hello".to_vec().into(), cc_rand.into()],
            input: ().into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("update")),
        };
        let pre_tx = Transaction::from(
            ContractCalls::new(&mut rng)
                .add_call::<BaseProofPreimage>(call_inner)
                .add_call::<BaseProofPreimage>(call_outer),
        );
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        tx.well_formed(&ledger_state, strictness).unwrap();
        tx
    };
    ledger_state = ledger_state.assert_apply(&tx);
    assert_eq!(
        &ledger_state.index(addr_outer).unwrap().data,
        &stval!([(b"hello".to_vec()), (addr_inner), (ep_hash)])
    );

    // Part 4: Rejected (call missing)
    println!(":: Part 4: Rejected (call missing)");
    {
        let cc_rand = rng.gen();
        let cc = transient_commit(
            &ValueReprAlignedValue(AlignedValue::from(b"malicious".to_vec())),
            cc_rand,
        );
        let transcripts = partition_transcripts(
            &[PreTranscript {
                context: &QueryContext::new(
                    ledger_state.index(addr_outer).unwrap().data,
                    addr_outer,
                ),
                program: &program_with_results(
                    &[
                        &Cell_read!([key!(1u8)], false, HashOutput)[..],
                        &Cell_read!([key!(2u8)], false, HashOutput),
                        &kernel_claim_contract_call!(
                            (),
                            (),
                            AlignedValue::from(addr_inner.0),
                            AlignedValue::from(ep_hash),
                            AlignedValue::from(cc)
                        ),
                        &Cell_write!([key!(0u8)], false, Vec<u8>, b"malicious".to_vec()),
                    ]
                    .into_iter()
                    .flat_map(|x| x.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
                    &[addr_inner.into(), ep_hash.into()],
                ),
                comm_comm: None,
            }],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        let call_outer = ContractCallPrototype {
            address: addr_outer,
            entry_point: b"update"[..].into(),
            op: update_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![b"malicious".to_vec().into(), cc_rand.into()],
            input: ().into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("update")),
        };
        let pre_tx = Transaction::from(
            ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call_outer),
        );
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        assert!(matches!(
            tx.well_formed(&ledger_state, strictness),
            Err(MalformedTransaction::ClaimCallFailed { .. })
        ));
    }

    // Part 5: Rejected (call mismatch)
    println!(":: Part 5: Rejected (call mismatch)");
    {
        let cc_rand = rng.gen();
        let cc = transient_commit(
            &ValueReprAlignedValue(AlignedValue::from(b"malicious".to_vec())),
            cc_rand,
        );
        let transcripts = partition_transcripts(
            &[
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_inner).unwrap().data,
                        addr_inner,
                    ),
                    program: &program_with_results(
                        &Cell_read!([key!(1u8)], false, Vec<u8>),
                        &[b"hello".to_vec().into()],
                    ),
                    comm_comm: Some(cc),
                },
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_outer).unwrap().data,
                        addr_outer,
                    ),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, HashOutput)[..],
                            &Cell_read!([key!(2u8)], false, HashOutput),
                            &kernel_claim_contract_call!(
                                (),
                                (),
                                AlignedValue::from(addr_inner.0),
                                AlignedValue::from(ep_hash),
                                AlignedValue::from(cc)
                            ),
                            &Cell_write!([key!(0u8)], false, Vec<u8>, b"malicious".to_vec()),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[addr_inner.into(), ep_hash.into()],
                    ),
                    comm_comm: None,
                },
            ],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        let call_inner = ContractCallPrototype {
            address: addr_inner,
            entry_point: b"get"[..].into(),
            op: get_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![],
            input: ().into(),
            output: b"hello".to_vec().into(),
            communication_commitment_rand: cc_rand,
            key_location: KeyLocation(Cow::Borrowed("get")),
        };
        let call_outer = ContractCallPrototype {
            address: addr_outer,
            entry_point: b"update"[..].into(),
            op: update_op.clone(),
            guaranteed_public_transcript: transcripts[1].0.clone(),
            fallible_public_transcript: transcripts[1].1.clone(),
            private_transcript_outputs: vec![b"malicious".to_vec().into(), cc_rand.into()],
            input: ().into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("update")),
        };
        let pre_tx = Transaction::from(
            ContractCalls::new(&mut rng)
                .add_call::<BaseProofPreimage>(call_inner)
                .add_call::<BaseProofPreimage>(call_outer),
        );
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        assert!(matches!(
            tx.well_formed(&ledger_state, strictness),
            Err(MalformedTransaction::ClaimCallFailed { .. })
        ));
    }

    // Part 6: Rejected (read mismatch)
    println!(":: Part 6: Rejected (read mismatch)");
    let tx = {
        let cc_rand = rng.gen();
        let cc = transient_commit(
            &ValueReprAlignedValue(b"malicious".to_vec().into()),
            cc_rand,
        );
        let transcripts = partition_transcripts(
            &[
                PreTranscript {
                    context: &QueryContext::new(
                        stval!([(auth_pk), (b"malicious".to_vec())]),
                        addr_inner,
                    ),
                    program: &program_with_results(
                        &Cell_read!([key!(1u8)], false, Vec<u8>),
                        &[b"malicious".to_vec().into()],
                    ),
                    comm_comm: Some(cc),
                },
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_outer).unwrap().data,
                        addr_outer,
                    ),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, HashOutput)[..],
                            &Cell_read!([key!(2u8)], false, HashOutput),
                            &kernel_claim_contract_call!(
                                (),
                                (),
                                AlignedValue::from(addr_inner.0),
                                AlignedValue::from(ep_hash),
                                AlignedValue::from(cc)
                            ),
                            &Cell_write!([key!(0u8)], false, Vec<u8>, b"malicious".to_vec()),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[addr_inner.into(), ep_hash.into()],
                    ),
                    comm_comm: None,
                },
            ],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        let call_inner = ContractCallPrototype {
            address: addr_inner,
            entry_point: b"get"[..].into(),
            op: get_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![],
            input: ().into(),
            output: b"malicious".to_vec().into(),
            communication_commitment_rand: cc_rand,
            key_location: KeyLocation(Cow::Borrowed("get")),
        };
        let call_outer = ContractCallPrototype {
            address: addr_outer,
            entry_point: b"update"[..].into(),
            op: update_op.clone(),
            guaranteed_public_transcript: transcripts[1].0.clone(),
            fallible_public_transcript: transcripts[1].1.clone(),
            private_transcript_outputs: vec![b"malicious".to_vec().into(), cc_rand.into()],
            input: ().into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("update")),
        };
        let pre_tx = Transaction::from(
            ContractCalls::new(&mut rng)
                .add_call::<BaseProofPreimage>(call_inner)
                .add_call::<BaseProofPreimage>(call_outer),
        );
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        tx.well_formed(&ledger_state, strictness).unwrap();
        tx
    };
    assert!(matches!(
        ledger_state.apply(&tx, &Default::default()).1,
        TransactionResult::Failure(TransactionInvalid::Transcript(
            TranscriptRejected::Execution(OnchainProgramError::ReadMismatch { .. })
        ))
    ));

    // Part 7: Rejected (read mismatch & fallibility hacking)
    println!(":: Part 7: Rejected (read mismatch & fallibility hacking)");
    {
        let cc_rand = rng.gen();
        let cc = transient_commit(
            &ValueReprAlignedValue(b"malicious".to_vec().into()),
            cc_rand,
        );
        let transcripts = partition_transcripts(
            &[
                PreTranscript {
                    context: &QueryContext::new(
                        stval!([(auth_pk), (b"malicious".to_vec())]),
                        addr_inner,
                    ),
                    program: &program_with_results(
                        &Cell_read!([key!(1u8)], false, Vec<u8>),
                        &[b"malicious".to_vec().into()],
                    ),
                    comm_comm: Some(cc),
                },
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_outer).unwrap().data,
                        addr_outer,
                    ),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, HashOutput)[..],
                            &Cell_read!([key!(2u8)], false, HashOutput),
                            &kernel_claim_contract_call!(
                                (),
                                (),
                                AlignedValue::from(addr_inner.0),
                                AlignedValue::from(ep_hash),
                                AlignedValue::from(cc)
                            ),
                            &Cell_write!([key!(0u8)], false, Vec<u8>, b"malicious".to_vec()),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[addr_inner.into(), ep_hash.into()],
                    ),
                    comm_comm: None,
                },
            ],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        // Manually move things to the fallible section.
        let call_inner = ContractCallPrototype {
            address: addr_inner,
            entry_point: b"get"[..].into(),
            op: get_op.clone(),
            guaranteed_public_transcript: None,
            fallible_public_transcript: transcripts[0].0.clone(),
            private_transcript_outputs: vec![],
            input: ().into(),
            output: b"malicious".to_vec().into(),
            communication_commitment_rand: cc_rand,
            key_location: KeyLocation(Cow::Borrowed("get")),
        };
        dbg!(&transcripts);
        let call_outer = ContractCallPrototype {
            address: addr_outer,
            entry_point: b"update"[..].into(),
            op: update_op.clone(),
            guaranteed_public_transcript: transcripts[1].0.clone(),
            fallible_public_transcript: transcripts[1].1.clone(),
            private_transcript_outputs: vec![b"malicious".to_vec().into(), cc_rand.into()],
            input: ().into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("update")),
        };
        let pre_tx = Transaction::from(
            ContractCalls::new(&mut rng)
                .add_call::<BaseProofPreimage>(call_inner)
                .add_call::<BaseProofPreimage>(call_outer),
        );
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::ClaimCallFailed { .. })
        ));
    }

    // Part 8: Contract merges aren't possible
    println!(":: Part 8: Contract merges aren't possible");
    {
        let cc_rand = rng.gen();
        let cc = transient_commit(
            &ValueReprAlignedValue(b"malicious".to_vec().into()),
            cc_rand,
        );
        let transcripts = partition_transcripts(
            &[
                PreTranscript {
                    context: &QueryContext::new(
                        stval!([(auth_pk), (b"malicious".to_vec())]),
                        addr_inner,
                    ),
                    program: &program_with_results(
                        &Cell_read!([key!(1u8)], false, Vec<u8>),
                        &[b"malicious".to_vec().into()],
                    ),
                    comm_comm: Some(cc),
                },
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_outer).unwrap().data,
                        addr_outer,
                    ),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, HashOutput)[..],
                            &Cell_read!([key!(2u8)], false, HashOutput),
                            &kernel_claim_contract_call!(
                                (),
                                (),
                                AlignedValue::from(addr_inner.0),
                                AlignedValue::from(ep_hash),
                                AlignedValue::from(cc)
                            ),
                            &Cell_write!([key!(0u8)], false, Vec<u8>, b"malicious".to_vec()),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[addr_inner.into(), ep_hash.into()],
                    ),
                    comm_comm: None,
                },
            ],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        let call_inner = ContractCallPrototype {
            address: addr_inner,
            entry_point: b"get"[..].into(),
            op: get_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![],
            input: ().into(),
            output: b"malicious".to_vec().into(),
            communication_commitment_rand: cc_rand,
            key_location: KeyLocation(Cow::Borrowed("get")),
        };
        let call_outer = ContractCallPrototype {
            address: addr_outer,
            entry_point: b"update"[..].into(),
            op: update_op.clone(),
            guaranteed_public_transcript: transcripts[1].0.clone(),
            fallible_public_transcript: transcripts[1].1.clone(),
            private_transcript_outputs: vec![b"malicious".to_vec().into(), cc_rand.into()],
            input: ().into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("update")),
        };
        let pre_tx1 = Transaction::from(
            ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call_inner),
        );
        let tx1 = tx_prove(rng.split(), &pre_tx1, &RESOLVER).await.unwrap();
        let pre_tx2 = Transaction::from(
            ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call_outer),
        );
        let tx2 = tx_prove(rng.split(), &pre_tx2, &RESOLVER).await.unwrap();
        assert!(matches!(
            tx1.merge(&tx2),
            Err(MalformedTransaction::MergingContracts)
        ));
    }
}

#[tokio::test]
async fn composable_funded() {
    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();

    // Part 1: Deploy burn
    println!(":: Part 1: Deploy burn");
    let burn_op = ContractOperation::new(verifier_key(&RESOLVER, "burn").await);
    let contract = ContractState {
        operations: HashMap::new().insert(b"burn"[..].into(), burn_op.clone()),
        data: stval!([]),
        maintenance_authority: Default::default(),
    };
    let (tx, addr_burn) = {
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

    // Part 2: Deploy relay
    println!(":: Part 2: Deploy relay");
    let ep_hash = persistent_commit(
        b"burn",
        HashOutput(*b"midnight:entry-point\0\0\0\0\0\0\0\0\0\0\0\0"),
    );
    let send_to_burn_op = ContractOperation::new(verifier_key(&RESOLVER, "send_to_burn").await);
    let contract = ContractState {
        operations: HashMap::new().insert(b"send_to_burn"[..].into(), send_to_burn_op.clone()),
        data: stval!([(addr_burn), (ep_hash)]),
        maintenance_authority: Default::default(),
    };
    let (tx, addr_relay) = {
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

    // Part 3: Burn
    println!(":: Part 3: Burn");
    let tx = {
        let coin_in = CoinInfo {
            nonce: rng.gen(),
            value: 10_001,
            type_: NATIVE_TOKEN,
        };
        let coin_xfer = coin_in.evolve_from(b"midnight:kernel:nonce_evolve", 10_000, NATIVE_TOKEN);
        let coin_change = coin_in.evolve_from(b"midnight:kernel:nonce_evolve/2", 1, NATIVE_TOKEN);
        let recipient_burn = Recipient::Contract(addr_burn);
        let recipient_relay = Recipient::Contract(addr_relay);
        let sender_relay = SenderEvidence::Contract(addr_relay);
        let cc_rand = rng.gen();
        let cc = transient_commit(&ValueReprAlignedValue(coin_xfer.into()), cc_rand);
        let transcripts = partition_transcripts(
            &[
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_burn).unwrap().data,
                        addr_burn,
                    ),
                    program: &program_with_results(
                        &[
                            &kernel_self!((), ())[..],
                            &kernel_claim_zswap_coin_receive!(
                                (),
                                (),
                                coin_xfer.commitment(&recipient_burn)
                            ),
                        ]
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect::<Vec<_>>()[..],
                        &[addr_burn.into()],
                    ),
                    comm_comm: Some(cc),
                },
                PreTranscript {
                    context: &QueryContext::new(
                        ledger_state.index(addr_relay).unwrap().data,
                        addr_relay,
                    ),
                    program: &program_with_results(
                        &[
                            &kernel_self!((), ())[..],
                            &kernel_claim_zswap_coin_receive!(
                                (),
                                (),
                                coin_in.commitment(&recipient_relay)
                            ),
                            &Cell_read!([key!(0u8)], false, HashOutput),
                            &kernel_self!((), ()),
                            &kernel_claim_zswap_nullifier!(
                                (),
                                (),
                                coin_in.nullifier(&sender_relay)
                            ),
                            &kernel_claim_zswap_coin_spend!(
                                (),
                                (),
                                coin_xfer.commitment(&recipient_burn)
                            ),
                            &kernel_claim_zswap_coin_spend!(
                                (),
                                (),
                                coin_change.commitment(&recipient_relay)
                            ),
                            &kernel_claim_zswap_coin_receive!(
                                (),
                                (),
                                coin_change.commitment(&recipient_relay)
                            ),
                            &Cell_read!([key!(0u8)], false, HashOutput),
                            &Cell_read!([key!(1u8)], false, HashOutput),
                            &kernel_claim_contract_call!(
                                (),
                                (),
                                AlignedValue::from(addr_burn),
                                AlignedValue::from(ep_hash),
                                AlignedValue::from(cc)
                            ),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[
                            addr_relay.into(),
                            addr_burn.into(),
                            addr_relay.into(),
                            addr_burn.into(),
                            ep_hash.into(),
                        ],
                    ),
                    comm_comm: None,
                },
            ],
            &DUMMY_PARAMETERS,
        )
        .unwrap();
        dbg!(&ledger_state);
        let call_burn = ContractCallPrototype {
            address: addr_burn,
            entry_point: b"burn"[..].into(),
            op: burn_op.clone(),
            guaranteed_public_transcript: transcripts[0].0.clone(),
            fallible_public_transcript: transcripts[0].1.clone(),
            private_transcript_outputs: vec![],
            input: coin_xfer.into(),
            output: ().into(),
            communication_commitment_rand: cc_rand,
            key_location: KeyLocation(Cow::Borrowed("burn")),
        };

        let call_relay = ContractCallPrototype {
            address: addr_relay,
            entry_point: b"send_to_burn"[..].into(),
            op: send_to_burn_op.clone(),
            guaranteed_public_transcript: transcripts[1].0.clone(),
            fallible_public_transcript: transcripts[1].1.clone(),
            private_transcript_outputs: vec![().into(), cc_rand.into()],
            input: coin_in.into(),
            output: ().into(),
            communication_commitment_rand: rng.gen(),
            key_location: KeyLocation(Cow::Borrowed("send_to_burn")),
        };
        let coin_in_out = Output::new_contract_owned(&mut rng, &coin_in, 0, addr_relay).unwrap();
        let mut offer = Offer {
            inputs: vec![],
            outputs: vec![
                Output::new_contract_owned(&mut rng, &coin_xfer, 0, addr_burn).unwrap(),
                Output::new_contract_owned(&mut rng, &coin_change, 0, addr_relay).unwrap(),
            ],
            transient: vec![
                Transient::new_from_contract_owned_output(
                    &mut rng,
                    &coin_in.qualify(0),
                    0,
                    coin_in_out,
                )
                .unwrap(),
            ],
            deltas: vec![(NATIVE_TOKEN, -10_001)],
        };
        offer.normalize();
        let pre_tx = Transaction::new(
            offer,
            None,
            Some(
                ContractCalls::new(&mut rng)
                    .add_call::<BaseProofPreimage>(call_burn)
                    .add_call::<BaseProofPreimage>(call_relay),
            ),
        );
        dbg!(&pre_tx);
        let tx = tx_prove(rng.split(), &pre_tx, &RESOLVER).await.unwrap();
        tx.well_formed(&ledger_state, strictness).unwrap();
        tx
    };
    ledger_state.assert_apply(&tx);
}
