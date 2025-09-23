#![deny(warnings)]

use coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN, QualifiedInfo as QualifiedCoinInfo};
use coin_structure::contract::Address as ContractAddress;
use coin_structure::transfer::{Recipient, SenderEvidence};
use futures::FutureExt;
use lazy_static::lazy_static;
use midnight_ledger::base_crypto::fab::{AlignedValue, Value};
use midnight_ledger::base_crypto::hash::{HashOutput, persistent_commit};
use midnight_ledger::base_crypto::rng::SplittableRng;
use midnight_ledger::construct::{ContractCallPrototype, PreTranscript, partition_transcripts};
use midnight_ledger::semantics::{ErasedTransactionResult::Success, ZswapLocalStateExt};
use midnight_ledger::serialize;
use midnight_ledger::storage::db::{DB, InMemoryDB};
use midnight_ledger::storage::storage::{Array, HashMap};
use midnight_ledger::structure::{
    ContractCalls, ContractDeploy, DUMMY_PARAMETERS, LedgerState, ProofPreimage, Transaction,
};
use midnight_ledger::test_utilities::tx_prove;
use midnight_ledger::test_utilities::{Resolver, verifier_key};
use midnight_ledger::test_utilities::{Tx, balance_tx, test_resolver};
use midnight_ledger::transient_crypto::curve::Fr;
use midnight_ledger::transient_crypto::fab::ValueReprAlignedValue;
use midnight_ledger::transient_crypto::merkle_tree::{MerkleTree, leaf_hash};
use midnight_ledger::transient_crypto::proofs::{KeyLocation, ProofPreimage as BaseProofPreimage};
use midnight_ledger::verify::WellFormedStrictness;
use onchain_runtime::context::QueryContext;
use onchain_runtime::ops::{Key, Op, key};
use onchain_runtime::program_fragments::*;
use onchain_runtime::result_mode::{ResultModeGather, ResultModeVerify};
use onchain_runtime::state::{ContractOperation, ContractState, StateValue, stval};
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, SeedableRng};
use std::borrow::Cow;
use std::fs::File;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use zswap::keys::SecretKeys;
use zswap::local::State as ZswapLocalState;
use zswap::transient_crypto::proofs::PARAMS_VERIFIER;
use zswap::verify::{OUTPUT_VK, SIGN_VK, SPEND_VK};
use zswap::{
    Input as ZswapInput, Offer as ZswapOffer, Output as ZswapOutput, Transient as ZswapTransient,
};

lazy_static! {
    static ref RESOLVER: Resolver = test_resolver("micro-dao");
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

fn context_with_offer<D: DB>(
    ledger: &LedgerState<D>,
    addr: ContractAddress,
    offer: Option<&ZswapOffer<BaseProofPreimage>>,
) -> QueryContext<D> {
    let mut res = QueryContext::new(ledger.index(addr).unwrap().data, addr);
    if let Some(offer) = offer {
        let (_, indicies) = ledger.zswap.try_apply(offer, None).unwrap();
        res.com_indicies = indicies;
    }
    res
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum TestMode {
    Full,
    Capture,
    Replay,
}

impl TestMode {
    async fn replay_or<T: Future<Output = Tx<D>>, F: FnOnce() -> T, D: DB>(
        self,
        file: impl AsRef<Path>,
        f: F,
    ) -> Tx<D> {
        if TestMode::Full == self {
            return f().await;
        }
        if TestMode::Capture == self {
            // Do the capture, and then immediately test it
            f().await;
        }
        let f = File::open(file.as_ref()).unwrap();
        serialize::deserialize(f, serialize::NetworkId::Undeployed).unwrap()
    }

    async fn capture_prove<D: DB>(
        self,
        file: impl AsRef<Path>,
        rng: impl CryptoRng + SplittableRng,
        tx: Transaction<ProofPreimage, D>,
    ) -> Tx<D> {
        let tx = tx_prove(rng, &tx, &RESOLVER).await.unwrap();
        if TestMode::Capture == self {
            let f = File::create(file.as_ref()).unwrap();
            serialize::serialize(&tx, f, serialize::NetworkId::Undeployed).unwrap();
        }
        tx
    }
}

#[tokio::test]
async fn micro_dao() {
    micro_dao_inner(TestMode::Full).await
}

#[tokio::test]
#[ignore = "run for specifically profiling node behaviour"]
async fn micro_dao_capture() {
    micro_dao_inner(TestMode::Capture).await
}

#[tokio::test]
#[ignore = "run for specifically profiling node behaviour"]
async fn micro_dao_replay() {
    micro_dao_inner(TestMode::Replay).await
}

#[allow(unused_assignments, clippy::redundant_clone)]
async fn micro_dao_inner(mode: TestMode) {
    let mut rng = StdRng::seed_from_u64(0x42);
    //rayon::ThreadPoolBuilder::new().use_current_thread().num_threads(1).build_global().unwrap();
    lazy_static::initialize(&PARAMS_VERIFIER);
    SPEND_VK.init().ok();
    OUTPUT_VK.init().ok();
    SIGN_VK.init().ok();
    let org_sk: HashOutput = rng.gen();
    let sep = b"lares:udao:pk";
    let org_pk = persistent_commit(sep, org_sk);
    let advance_op = ContractOperation::new(verifier_key(&RESOLVER, "advance").await);
    let buy_in_op = ContractOperation::new(verifier_key(&RESOLVER, "buy_in").await);
    let cash_out_op = ContractOperation::new(verifier_key(&RESOLVER, "cash_out").await);
    let set_topic_op = ContractOperation::new(verifier_key(&RESOLVER, "set_topic").await);
    let vote_commit_op = ContractOperation::new(verifier_key(&RESOLVER, "vote_commit").await);
    let vote_reveal_op = ContractOperation::new(verifier_key(&RESOLVER, "vote_reveal").await);

    dbg!(cfg!(feature = "proving"));
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();
    let mut zswap_state = ZswapLocalState::<InMemoryDB>::new();
    let zswap_sk = SecretKeys::from_rng_seed(&mut rng);
    // Part 0: Premint
    println!(":: Part 0: Premint");
    const MINT_AMOUNT: u128 = 5000000000;
    let mut unbalanced_strictness = WellFormedStrictness::default();
    unbalanced_strictness.enforce_balancing = false;
    let balanced_strictness = WellFormedStrictness::default();
    let tx = mode
        .replay_or("tx0", || {
            let coin = CoinInfo::new(&mut rng, MINT_AMOUNT, NATIVE_TOKEN);
            let out =
                ZswapOutput::new(&mut rng, &coin, 0, &zswap_sk.coin_public_key(), None).unwrap();
            zswap_state = zswap_state.watch_for(&zswap_sk.coin_public_key(), &coin);
            let offer = ZswapOffer {
                inputs: Vec::new(),
                outputs: vec![out],
                transient: Vec::new(),
                deltas: vec![(NATIVE_TOKEN, -(MINT_AMOUNT as i128))],
            };
            mode.capture_prove("tx0", rng.split(), Transaction::new(offer, None, None))
        })
        .await;
    tx.well_formed(&ledger_state, unbalanced_strictness)
        .unwrap();
    ledger_state = ledger_state.assert_apply(&tx);
    if mode != TestMode::Replay {
        zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
    }
    let funds_before: u128 = zswap_state
        .coins
        .iter()
        .map(|a| a.1)
        .filter(|c| c.type_ == NATIVE_TOKEN)
        .map(|c| c.value)
        .sum();

    // Part 1: Deploy
    println!(":: Part 1: Deploy");
    let contract: ContractState<InMemoryDB> = ContractState {
        operations: HashMap::new()
            .insert(b"advance"[..].into(), advance_op.clone())
            .insert(b"buy_in"[..].into(), buy_in_op.clone())
            .insert(b"cash_out"[..].into(), cash_out_op.clone())
            .insert(b"set_topic"[..].into(), set_topic_op.clone())
            .insert(b"vote_commit"[..].into(), vote_commit_op.clone())
            .insert(b"vote_reveal"[..].into(), vote_reveal_op.clone()),
        data: stval!([
            (org_pk),
            (0u8),
            (Option::<Vec<u8>>::None),
            (Option::<[u8; 32]>::None),
            (0u64),
            (0u64),
            (0u64),
            [{MT(10) {}}, (0u64)],
            [{MT(10) {}}, (0u64), { MerkleTree::<()>::blank(10).root() => null }],
            {},
            {},
            (QualifiedCoinInfo::default()),
            (false)
        ]),
        maintenance_authority: Default::default(),
    };
    let tx = mode
        .replay_or("tx1", || {
            let deploy = ContractDeploy::new(&mut rng, contract.clone());
            let tx = Transaction::from(ContractCalls::new(&mut rng).add_deploy(deploy));
            tx.well_formed(&ledger_state, unbalanced_strictness)
                .unwrap();

            let tx = balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER);
            tx.then(|tx| mode.capture_prove("tx1", rng.split(), tx.unwrap()))
        })
        .await;
    let addr = tx.deploys().next().unwrap().address();
    tx.well_formed(&ledger_state, balanced_strictness).unwrap();
    ledger_state = ledger_state.assert_apply(&tx);
    if mode != TestMode::Replay {
        zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
    }

    println!(":: Part 2: Setting topic");
    let tx = mode
        .replay_or("tx2", || {
            let transcripts = partition_transcripts(
                &[PreTranscript {
                    context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(0u8)], false, [u8; 32])[..],
                            &Cell_read!([key!(1u8)], false, u8),
                            &Cell_write!(
                                [key!(2u8)],
                                false,
                                Option<Vec<u8>>,
                                Some(b"test topic".to_vec())
                            ),
                            &Cell_write!(
                                [key!(3u8)],
                                false,
                                Option<[u8; 32]>,
                                Some(zswap_sk.coin_public_key().0.0)
                            ),
                            &Cell_write!([key!(1u8)], true, u8, 1u8),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[org_pk.into(), 0u8.into()],
                    ),
                    comm_comm: None,
                }],
                &DUMMY_PARAMETERS,
            )
            .unwrap();
            let call = ContractCallPrototype {
                address: addr,
                entry_point: b"set_topic"[..].into(),
                op: set_topic_op.clone(),
                input: (b"test topic".to_vec(), zswap_sk.coin_public_key()).into(),
                output: ().into(),
                guaranteed_public_transcript: transcripts[0].0.clone(),
                fallible_public_transcript: transcripts[0].1.clone(),
                private_transcript_outputs: vec![org_sk.into()],
                communication_commitment_rand: rng.gen(),
                key_location: KeyLocation(Cow::Borrowed("set_topic")),
            };
            let tx =
                Transaction::from(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call));
            tx.well_formed(&ledger_state, unbalanced_strictness)
                .unwrap();
            //gen_static_serialize_file(&tx).unwrap();
            balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER)
                .then(|tx| mode.capture_prove("tx2", rng.split(), tx.unwrap()))
        })
        .await;
    //dbg!(&tx);
    tx.well_formed(&ledger_state, balanced_strictness).unwrap();
    ledger_state = ledger_state.assert_apply(&tx);
    //gen_static_serialize_file(&ledger_state).unwrap();
    if mode != TestMode::Replay {
        zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
    }

    // Part 3: Buy-in
    println!(":: Part 3: Buy-in");
    let part_sks: [HashOutput; 2] = rng.gen();
    let part_pks: [HashOutput; 2] = [
        persistent_commit(sep, part_sks[0]),
        persistent_commit(sep, part_sks[1]),
    ];
    let part_names: [&'static str; 2] = ["red", "blue"];
    for ((sk, pk), name) in part_sks.iter().zip(part_pks.iter()).zip(part_names.iter()) {
        println!("  :: {}", name);
        let tx = mode
            .replay_or(format!("tx3-{name}"), || {
                let coin = CoinInfo::new(&mut rng, 100000, NATIVE_TOKEN);
                let out = ZswapOutput::new_contract_owned(&mut rng, &coin, 0, addr).unwrap();
                let coin_com = coin.commitment(&Recipient::Contract(addr));
                let pot_has_coin = *name != "red";
                let mut public_transcript: Vec<Op<ResultModeGather, InMemoryDB>> = [
                    &kernel_self!((), ())[..],
                    &kernel_claim_zswap_coin_receive!((), (), coin_com),
                    &Cell_read!([key!(12u8)], false, bool),
                ]
                .into_iter()
                .flat_map(|x| x.iter())
                .cloned()
                .collect();
                let mut public_transcript_results: Vec<AlignedValue> =
                    vec![addr.into(), pot_has_coin.into()];

                let offer = if pot_has_coin {
                    let state = ledger_state.contract.get(&addr).unwrap();
                    let pot_cell = if let StateValue::Array(arr) = &state.data {
                        &arr[11]
                    } else {
                        unreachable!()
                    };
                    let pot = if let StateValue::Cell(pot) = pot_cell {
                        QualifiedCoinInfo::try_from(&**AsRef::<Value>::as_ref(pot)).unwrap()
                    } else {
                        unreachable!()
                    };
                    let pot_nul = CoinInfo::from(&pot).nullifier(&SenderEvidence::Contract(addr));
                    let coin_nul = coin.nullifier(&SenderEvidence::Contract(addr));
                    let pot_in = ZswapInput::new_contract_owned(
                        &mut rng,
                        &pot,
                        0,
                        addr,
                        &ledger_state.zswap.coin_coms,
                    )
                    .unwrap();
                    let transient = ZswapTransient::new_from_contract_owned_output(
                        &mut rng,
                        &coin.qualify(0),
                        0,
                        out,
                    )
                    .unwrap();
                    let new_coin = CoinInfo::from(&pot).evolve_from(
                        b"midnight:kernel:nonce_evolve",
                        pot.value + coin.value,
                        pot.type_,
                    );
                    let out =
                        ZswapOutput::new_contract_owned(&mut rng, &new_coin, 0, addr).unwrap();
                    let coin_com = new_coin.commitment(&Recipient::Contract(addr));

                    public_transcript_results.extend([pot.into(), addr.into(), addr.into()]);
                    public_transcript.extend(
                        [
                            &Cell_read!([key!(11u8)], false, QualifiedCoinInfo)[..],
                            &kernel_self!((), ()),
                            &kernel_claim_zswap_nullifier!((), (), pot_nul),
                            &kernel_claim_zswap_nullifier!((), (), coin_nul),
                            &kernel_claim_zswap_coin_spend!((), (), coin_com),
                            &kernel_claim_zswap_coin_receive!((), (), coin_com),
                            &kernel_self!((), ()),
                            &Cell_write_coin!(
                                [key!(11u8)],
                                true,
                                QualifiedCoinInfo,
                                new_coin,
                                Recipient::Contract(addr)
                            ),
                        ]
                        .into_iter()
                        .flatten()
                        .cloned(),
                    );
                    ZswapOffer {
                        inputs: vec![pot_in],
                        outputs: vec![out],
                        transient: vec![transient],
                        deltas: vec![(NATIVE_TOKEN, -100000)],
                    }
                } else {
                    public_transcript_results.extend([addr.into()]);
                    public_transcript.extend(
                        [
                            &kernel_self!((), ()),
                            &Cell_write_coin!(
                                [key!(11u8)],
                                true,
                                QualifiedCoinInfo,
                                coin.clone(),
                                Recipient::Contract(addr)
                            )[..],
                            &Cell_write!([key!(12u8)], true, bool, true),
                        ]
                        .into_iter()
                        .flatten()
                        .cloned(),
                    );
                    ZswapOffer {
                        inputs: Vec::new(),
                        outputs: vec![out],
                        transient: Vec::new(),
                        deltas: vec![(NATIVE_TOKEN, -100000)],
                    }
                };
                public_transcript.extend(
                    HistoricMerkleTree_insert!([key!(8u8)], false, 10, [u8; 32], pk)
                        .iter()
                        .cloned(),
                );
                let transcripts = partition_transcripts(
                    &[PreTranscript {
                        context: &context_with_offer(&ledger_state, addr, Some(&offer)),
                        program: &program_with_results(
                            &public_transcript,
                            &public_transcript_results,
                        ),
                        comm_comm: None,
                    }],
                    &DUMMY_PARAMETERS,
                )
                .unwrap();
                let call = ContractCallPrototype {
                    address: addr,
                    entry_point: b"buy_in"[..].into(),
                    op: buy_in_op.clone(),
                    input: coin.into(),
                    output: ().into(),
                    guaranteed_public_transcript: transcripts[0].0.clone(),
                    fallible_public_transcript: transcripts[0].1.clone(),
                    private_transcript_outputs: vec![(*sk).into()],
                    communication_commitment_rand: rng.gen(),
                    key_location: KeyLocation(Cow::Borrowed("buy_in")),
                };
                let tx = Transaction::new(
                    offer,
                    None,
                    Some(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call)),
                );
                tx.well_formed(&ledger_state, unbalanced_strictness)
                    .unwrap();

                balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER)
                    .then(|tx| mode.capture_prove(format!("tx3-{name}"), rng.split(), tx.unwrap()))
            })
            .await;
        //dbg!(&tx);
        tx.well_formed(&ledger_state, balanced_strictness).unwrap();
        ledger_state = ledger_state.assert_apply(&tx);
        if mode != TestMode::Replay {
            zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
        }
    }

    // Part 4: Vote commitment
    println!(":: Part 4: Vote commitment");
    let part_votes: [bool; 2] = [true, false];
    for (((sk, pk), vote), name) in part_sks
        .iter()
        .zip(part_pks.iter())
        .zip(part_votes.iter())
        .zip(part_names.iter())
    {
        println!("  :: {}", name);
        let tx = mode
            .replay_or(format!("tx4-{name}"), || {
                let contract = ledger_state.index(addr).unwrap();
                let eligible_voters = if let StateValue::Array(arr) = &contract.data {
                    &arr[8]
                } else {
                    unreachable!()
                };
                let mtree_val = if let StateValue::Array(arr) = eligible_voters {
                    &arr[0]
                } else {
                    unreachable!()
                };
                let path = if let StateValue::BoundedMerkleTree(tree) = mtree_val {
                    tree.find_path_for_leaf(*pk).unwrap()
                } else {
                    unreachable!()
                };
                let nul = persistent_commit(b"\0\0\0\0\0\0\0\0udao:cn\0", *sk);
                let cm = persistent_commit(
                    if *vote {
                        b"\0\0\0\0\0\0\0\0yes\0\0\0\0\0"
                    } else {
                        b"\0\0\0\0\0\0\0\0no\0\0\0\0\0\0"
                    },
                    *sk,
                );
                let private_transcript_outputs = vec![
                    AlignedValue::from(Fr::from(0u64)),
                    AlignedValue::from(*sk),
                    AlignedValue::from(true),
                    AlignedValue::from(path.clone()),
                ];
                let transcripts = partition_transcripts(
                    &[PreTranscript {
                        context: &context_with_offer(&ledger_state, addr, None),
                        program: &program_with_results(
                            &[
                                &Cell_read!(&[key!(1u8)], false, u8)[..],
                                &Counter_read!(&[key!(6u8)], false),
                                &Set_member!(&[key!(9u8)], false, [u8; 32], nul.0),
                                &HistoricMerkleTree_check_root!(
                                    &[key!(8u8)],
                                    false,
                                    10,
                                    [u8; 32],
                                    path.root()
                                ),
                                &Counter_read!(&[key!(6u8)], false),
                                &MerkleTree_insert!(&[key!(7u8)], false, 10, [u8; 32], cm.0),
                                &Set_insert!(&[key!(9u8)], false, [u8; 32], nul.0),
                            ]
                            .into_iter()
                            .flat_map(|x| x.iter())
                            .cloned()
                            .collect::<Vec<_>>(),
                            &[
                                1u8.into(),
                                0u64.into(),
                                false.into(),
                                true.into(),
                                0u64.into(),
                            ],
                        ),
                        comm_comm: None,
                    }],
                    &DUMMY_PARAMETERS,
                )
                .unwrap();
                let call = ContractCallPrototype {
                    address: addr,
                    entry_point: b"vote_commit"[..].into(),
                    op: vote_commit_op.clone(),
                    input: (*vote).into(),
                    output: ().into(),
                    guaranteed_public_transcript: transcripts[0].0.clone(),
                    fallible_public_transcript: transcripts[0].1.clone(),
                    private_transcript_outputs,
                    communication_commitment_rand: rng.gen(),
                    key_location: KeyLocation(Cow::Borrowed("vote_commit")),
                };
                let tx = Transaction::from(
                    ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call),
                );
                tx.well_formed(&ledger_state, unbalanced_strictness)
                    .unwrap();

                balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER)
                    .then(|tx| mode.capture_prove(format!("tx4-{name}"), rng.split(), tx.unwrap()))
            })
            .await;
        //dbg!(&tx);
        tx.well_formed(&ledger_state, balanced_strictness).unwrap();
        ledger_state = ledger_state.assert_apply(&tx);
        if mode != TestMode::Replay {
            zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
        }
    }

    // Part 5: advance to reveal phase
    println!(":: Part 5: Advance to reveal phase");
    let tx = mode
        .replay_or("tx5", || {
            let transcripts = partition_transcripts(
                &[PreTranscript {
                    context: &context_with_offer(&ledger_state, addr, None),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, u8)[..],
                            &Cell_read!([key!(0u8)], false, [u8; 32]),
                            &Cell_read!([key!(1u8)], false, u8),
                            &Cell_write!([key!(1u8)], false, u8, 2u8),
                            &Cell_read!([key!(1u8)], false, u8),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[1u8.into(), org_pk.into(), 1u8.into(), 2u8.into()],
                    ),
                    comm_comm: None,
                }],
                &DUMMY_PARAMETERS,
            )
            .unwrap();
            let call = ContractCallPrototype {
                address: addr,
                entry_point: b"advance"[..].into(),
                op: advance_op.clone(),
                input: ().into(),
                output: ().into(),
                guaranteed_public_transcript: transcripts[0].0.clone(),
                fallible_public_transcript: transcripts[0].1.clone(),
                private_transcript_outputs: vec![org_sk.into()],
                communication_commitment_rand: rng.gen(),
                key_location: KeyLocation(Cow::Borrowed("advance")),
            };
            let tx =
                Transaction::from(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call));
            tx.well_formed(&ledger_state, unbalanced_strictness)
                .unwrap();

            balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER)
                .then(|tx| mode.capture_prove("tx5", rng.split(), tx.unwrap()))
        })
        .await;
    //dbg!(&tx);
    tx.well_formed(&ledger_state, balanced_strictness).unwrap();
    ledger_state = ledger_state.assert_apply(&tx);
    if mode != TestMode::Replay {
        zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
    }

    // Part 6: Vote revealing
    println!(":: Part 6: Vote revealing");
    for ((sk, vote), name) in part_sks
        .iter()
        .zip(part_votes.iter())
        .zip(part_names.iter())
    {
        println!("  :: {}", name);
        let tx = mode
            .replay_or(format!("tx6-{name}"), || {
                let cm = persistent_commit(
                    if *vote {
                        b"\0\0\0\0\0\0\0\0yes\0\0\0\0\0"
                    } else {
                        b"\0\0\0\0\0\0\0\0no\0\0\0\0\0\0"
                    },
                    *sk,
                );
                let contract = ledger_state.index(addr).unwrap();
                let committed_votes = if let StateValue::Array(arr) = &contract.data {
                    &arr[7]
                } else {
                    unreachable!()
                };
                let mtree_value = if let StateValue::Array(arr) = committed_votes {
                    &arr[0]
                } else {
                    unreachable!()
                };
                let path = if let StateValue::BoundedMerkleTree(tree) = mtree_value {
                    tree.find_path_for_leaf(cm).unwrap()
                } else {
                    unreachable!()
                };
                let nul = persistent_commit(b"\0\0\0\0\0\0\0\0udao:rn\0", *sk);
                let private_transcript_outputs = vec![
                    AlignedValue::from(Fr::from(1u64)),
                    AlignedValue::from(*sk),
                    AlignedValue::from(true),
                    AlignedValue::from(*vote),
                    AlignedValue::from(true),
                    AlignedValue::from(path.clone()),
                ];
                let transcripts = partition_transcripts(
                    &[PreTranscript {
                        context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
                        program: &program_with_results(
                            &[
                                &Cell_read!([key!(1u8)], false, u8)[..],
                                &Counter_read!([key!(6u8)], false),
                                &Set_member!([key!(10u8)], false, [u8; 32], nul.0),
                                &Counter_read!([key!(6u8)], false),
                                &MerkleTree_check_root!(
                                    [key!(7u8)],
                                    false,
                                    10,
                                    [u8; 32],
                                    path.root()
                                ),
                                &Counter_increment!(
                                    [key!(if *vote { 4u8 } else { 5u8 })],
                                    false,
                                    1u64
                                ),
                                &Set_insert!([key!(10u8)], false, [u8; 32], nul.0),
                            ]
                            .into_iter()
                            .flat_map(|x| x.iter())
                            .cloned()
                            .collect::<Vec<_>>(),
                            &[
                                2u8.into(),
                                0u64.into(),
                                false.into(),
                                0u64.into(),
                                true.into(),
                            ],
                        ),
                        comm_comm: None,
                    }],
                    &DUMMY_PARAMETERS,
                )
                .unwrap();
                let call = ContractCallPrototype {
                    address: addr,
                    entry_point: b"vote_reveal"[..].into(),
                    op: vote_reveal_op.clone(),
                    input: ().into(),
                    output: ().into(),
                    guaranteed_public_transcript: transcripts[0].0.clone(),
                    fallible_public_transcript: transcripts[0].1.clone(),
                    private_transcript_outputs,
                    communication_commitment_rand: rng.gen(),
                    key_location: KeyLocation(Cow::Borrowed("vote_reveal")),
                };
                let tx = Transaction::from(
                    ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call),
                );
                tx.well_formed(&ledger_state, unbalanced_strictness)
                    .unwrap();

                balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER)
                    .then(|tx| mode.capture_prove(format!("tx6-{name}"), rng.split(), tx.unwrap()))
            })
            .await;
        //dbg!(&tx);
        tx.well_formed(&ledger_state, balanced_strictness).unwrap();
        ledger_state = ledger_state.assert_apply(&tx);
        if mode != TestMode::Replay {
            zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
        }
    }

    // Part 7: advance to final phase
    println!(":: Part 7: Advance to final phase");
    let tx = mode
        .replay_or("tx7", || {
            let transcripts = partition_transcripts(
                &[PreTranscript {
                    context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, u8)[..],
                            &Cell_read!([key!(0u8)], false, [u8; 32]),
                            &Cell_read!([key!(1u8)], false, u8),
                            &Cell_write!([key!(1u8)], false, u8, 3u8),
                            &Cell_read!([key!(1u8)], false, u8),
                            &Counter_read!([key!(5u8)], false),
                            &Counter_less_than!([key!(4u8)], false, 1u64),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[
                            2u8.into(),
                            org_pk.into(),
                            2u8.into(),
                            3u8.into(),
                            1u64.into(),
                            false.into(),
                        ],
                    ),
                    comm_comm: None,
                }],
                &DUMMY_PARAMETERS,
            )
            .unwrap();
            let call = ContractCallPrototype {
                address: addr,
                entry_point: b"advance"[..].into(),
                op: advance_op.clone(),
                input: ().into(),
                output: ().into(),
                guaranteed_public_transcript: transcripts[0].0.clone(),
                fallible_public_transcript: transcripts[0].1.clone(),
                private_transcript_outputs: vec![org_sk.into()],
                communication_commitment_rand: rng.gen(),
                key_location: KeyLocation(Cow::Borrowed("advance")),
            };
            let tx =
                Transaction::from(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call));
            tx.well_formed(&ledger_state, unbalanced_strictness)
                .unwrap();

            balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER)
                .then(|tx| mode.capture_prove("tx7", rng.split(), tx.unwrap()))
        })
        .await;
    //dbg!(&tx);
    tx.well_formed(&ledger_state, balanced_strictness).unwrap();
    ledger_state = ledger_state.assert_apply(&tx);
    if mode != TestMode::Replay {
        zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
    }

    // Part 8: cash out
    println!(":: Part 8: Cash Out");
    let tx = mode
        .replay_or("tx8", || {
            let contract = ledger_state.contract.get(&addr).unwrap();
            let pot_val = if let StateValue::Array(arr) = &contract.data {
                &arr[11]
            } else {
                unreachable!()
            };
            let pot = if let StateValue::Cell(pot) = pot_val {
                QualifiedCoinInfo::try_from(&**AsRef::<Value>::as_ref(pot)).unwrap()
            } else {
                unreachable!()
            };
            let new_coin = CoinInfo::from(&pot).evolve_from(
                b"midnight:kernel:nonce_evolve",
                pot.value,
                pot.type_,
            );
            let nul = CoinInfo::from(&pot).nullifier(&SenderEvidence::Contract(addr));
            let coin_com = new_coin.commitment(&Recipient::User(zswap_sk.coin_public_key()));
            let beneficiary = Some(zswap_sk.coin_public_key());
            let transcripts = partition_transcripts(
                &[PreTranscript {
                    context: &QueryContext::new(ledger_state.index(addr).unwrap().data, addr),
                    program: &program_with_results(
                        &[
                            &Cell_read!([key!(1u8)], false, u8)[..],
                            &Cell_read!([key!(3u8)], false, Option<[u8; 32]>),
                            &Cell_read!([key!(3u8)], false, Option<[u8; 32]>),
                            &Counter_read!([key!(4u8)], false),
                            &Counter_less_than!([key!(5u8)], false, 2u64),
                            &Cell_read!([key!(11u8)], false, QualifiedCoinInfo),
                            &Cell_read!([key!(11u8)], false, QualifiedCoinInfo),
                            &kernel_self!((), ()),
                            &kernel_claim_zswap_nullifier!((), (), nul),
                            &kernel_claim_zswap_coin_spend!((), (), coin_com),
                            &Cell_write!([key!(1u8)], false, u8, 0u8),
                            &Cell_write!(
                                [key!(2u8)],
                                false,
                                Option<Vec<u8>>,
                                Option::<Vec<u8>>::None
                            ),
                            &Counter_reset_to_default!([key!(4u8)], false),
                            &Counter_reset_to_default!([key!(5u8)], false),
                            &Cell_write!(
                                [key!(3u8)],
                                false,
                                Option<[u8; 32]>,
                                Option::<[u8; 32]>::None
                            ),
                            &MerkleTree_reset_to_default!([key!(7u8)], false, 10, [u8; 32]),
                            &Set_reset_to_default!([key!(9u8)], false, [u8; 32]),
                            &Set_reset_to_default!([key!(10u8)], false, [u8; 32]),
                            &Cell_write!(
                                [key!(11u8)],
                                false,
                                QualifiedCoinInfo,
                                QualifiedCoinInfo::default()
                            ),
                            &Cell_write!([key!(12u8)], false, bool, false),
                            &Counter_increment!([key!(6u8)], false, 1u64),
                        ]
                        .into_iter()
                        .flat_map(|x| x.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                        &[
                            3u8.into(),
                            beneficiary.into(),
                            beneficiary.into(),
                            1u64.into(),
                            true.into(),
                            pot.into(),
                            pot.into(),
                            addr.into(),
                        ],
                    ),
                    comm_comm: None,
                }],
                &DUMMY_PARAMETERS,
            )
            .unwrap();
            let call = ContractCallPrototype {
                address: addr,
                entry_point: b"cash_out"[..].into(),
                op: cash_out_op.clone(),
                input: ().into(),
                output: new_coin.into(),
                guaranteed_public_transcript: transcripts[0].0.clone(),
                fallible_public_transcript: transcripts[0].1.clone(),
                private_transcript_outputs: vec![zswap_sk.coin_public_key().into()],
                communication_commitment_rand: rng.gen(),
                key_location: KeyLocation(Cow::Borrowed("cash_out")),
            };
            let offer = ZswapOffer {
                inputs: vec![
                    ZswapInput::new_contract_owned(
                        &mut rng,
                        &pot,
                        0,
                        addr,
                        &ledger_state.zswap.coin_coms,
                    )
                    .unwrap(),
                ],
                outputs: vec![
                    ZswapOutput::new(&mut rng, &new_coin, 0, &zswap_sk.coin_public_key(), None)
                        .unwrap(),
                ],
                transient: vec![],
                deltas: vec![],
            };
            zswap_state = zswap_state.watch_for(&zswap_sk.coin_public_key(), &new_coin);
            let tx = Transaction::new(
                offer,
                None,
                Some(ContractCalls::new(&mut rng).add_call::<BaseProofPreimage>(call)),
            );
            tx.well_formed(&ledger_state, unbalanced_strictness)
                .unwrap();
            balance_tx(rng.split(), tx, &mut zswap_state, &zswap_sk, &RESOLVER)
                .then(|tx| mode.capture_prove("tx8", rng.split(), tx.unwrap()))
        })
        .await;
    //dbg!(&tx);
    tx.well_formed(&ledger_state, balanced_strictness).unwrap();
    ledger_state = ledger_state.assert_apply(&tx);
    if mode != TestMode::Replay {
        zswap_state = zswap_state.apply_tx(&zswap_sk, &tx, Success);
    }
    let funds_after: u128 = zswap_state
        .coins
        .iter()
        .map(|a| a.1)
        .filter(|c| c.type_ == NATIVE_TOKEN)
        .map(|c| c.value)
        .sum();
    println!(
        "We started with {} tokens, and ended with {}. {} lost to fees, and hopefully not the contract.",
        funds_before,
        funds_after,
        funds_before - funds_after
    );
}
