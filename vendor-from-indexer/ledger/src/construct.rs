use crate::base_crypto::fab::AlignedValue;
use crate::base_crypto::signatures::Signature;
use crate::error::PartitionFailure;
use crate::serialize::{Serializable, Versioned};
use crate::structure::{
    ContractAction, ContractCall, ContractCalls, ContractDeploy, LedgerParameters,
    MaintenanceUpdate, PROOF_SIZE, ProofPreimage, ProofPreimageVersioned, SingleUpdate,
    StandardTransaction, Transaction,
};
use crate::transient_crypto::commitment::PedersenRandomness;
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::fab::{AlignedValueExt, ValueReprAlignedValue};
use crate::transient_crypto::hash::transient_commit;
use crate::transient_crypto::proofs::{KeyLocation, ProofPreimage as BaseProofPreimage};
use crate::transient_crypto::repr::FieldRepr;
use coin_structure::contract::Address as ContractAddress;
use onchain_runtime::context::{Effects, QueryContext, QueryResults};
use onchain_runtime::error::TranscriptRejected;
use onchain_runtime::ops::Op;
use onchain_runtime::result_mode::ResultModeVerify;
use onchain_runtime::state::{ContractOperation, ContractState, EntryPointBuf};
use onchain_runtime::transcript::Transcript;
use rand::{CryptoRng, Rng};
use std::iter::once;
use zswap::Offer as ZswapOffer;
use zswap::storage::db::DB;

impl<D: DB> From<ContractCalls<ProofPreimage, D>> for Transaction<ProofPreimage, D> {
    fn from(calls: ContractCalls<ProofPreimage, D>) -> Self {
        Self::new(
            ZswapOffer {
                inputs: Vec::new(),
                outputs: Vec::new(),
                transient: Vec::new(),
                deltas: Vec::new(),
            },
            None,
            Some(calls),
        )
    }
}

impl<D: DB> ContractDeploy<D> {
    pub fn new<R: Rng + CryptoRng + ?Sized>(rng: &mut R, initial_state: ContractState<D>) -> Self {
        ContractDeploy {
            initial_state,
            nonce: rng.gen(),
            raw_data: None,
        }
    }
}

impl MaintenanceUpdate {
    pub fn new(address: ContractAddress, updates: Vec<SingleUpdate>, counter: u32) -> Self {
        MaintenanceUpdate {
            address,
            updates,
            counter,
            signatures: Vec::new(),
            raw_data: None,
            raw_data_to_sign: None,
        }
    }

    pub fn add_signature(mut self, idx: u32, signature: Signature) -> Self {
        self.signatures.push((idx, signature));
        self.signatures.sort();
        self
    }
}

impl<D: DB> Transaction<ProofPreimage, D> {
    pub fn new(
        guaranteed_coins: ZswapOffer<BaseProofPreimage>,
        fallible_coins: Option<ZswapOffer<BaseProofPreimage>>,
        contract_calls: Option<ContractCalls<ProofPreimage, D>>,
    ) -> Self {
        let binding_randomness = guaranteed_coins.binding_randomness()
            + fallible_coins
                .as_ref()
                .map(|o| o.binding_randomness())
                .unwrap_or_else(|| PedersenRandomness::from(0))
            + contract_calls
                .as_ref()
                .map(|c| c.binding_randomness())
                .unwrap_or(0.into());
        Transaction::Standard(StandardTransaction {
            guaranteed_coins,
            fallible_coins,
            contract_calls,
            binding_randomness,
            raw_data: None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ContractCallPrototype<D: DB> {
    pub address: ContractAddress,
    pub entry_point: EntryPointBuf,
    pub op: ContractOperation,
    pub guaranteed_public_transcript: Option<Transcript<D>>,
    pub fallible_public_transcript: Option<Transcript<D>>,
    pub private_transcript_outputs: Vec<AlignedValue>,
    pub input: AlignedValue,
    pub output: AlignedValue,
    pub communication_commitment_rand: Fr,
    pub key_location: KeyLocation,
}

pub trait ContractCallExt<D: DB> {
    type Proof;
    fn construct_proof(
        call: &ContractCallPrototype<D>,
        communication_commitment: Fr,
    ) -> ProofPreimageVersioned;
}

impl<D: DB> ContractCallExt<D> for BaseProofPreimage {
    type Proof = BaseProofPreimage;
    fn construct_proof(
        call: &ContractCallPrototype<D>,
        communication_commitment: Fr,
    ) -> ProofPreimageVersioned {
        let inputs = ValueReprAlignedValue(call.input.clone()).field_vec();
        let mut private_transcript = Vec::with_capacity(
            call.private_transcript_outputs
                .iter()
                .map(|o| o.value_only_field_size())
                .sum(),
        );
        for o in call.private_transcript_outputs.iter() {
            o.value_only_field_repr(&mut private_transcript);
        }
        let public_transcript_iter = call
            .guaranteed_public_transcript
            .iter()
            .flat_map(|t| t.program.iter())
            .chain(
                call.fallible_public_transcript
                    .iter()
                    .flat_map(|t| t.program.iter()),
            );
        let mut public_transcript_inputs = Vec::with_capacity(
            public_transcript_iter
                .clone()
                .map(|op| op.field_size())
                .sum(),
        );
        for op in public_transcript_iter.clone() {
            op.field_repr(&mut public_transcript_inputs);
        }
        let mut public_transcript_outputs = Vec::new();
        for op in public_transcript_iter {
            if let Op::Popeq { result, .. } = op {
                result.value_only_field_repr(&mut public_transcript_outputs);
            }
        }

        // This gets populated correctly during proving, ensuring it correctly uses the updated
        // transcripts and costs.
        let binding_input = 0u8.into();

        let proof = BaseProofPreimage {
            inputs,
            private_transcript,
            public_transcript_inputs,
            public_transcript_outputs,
            binding_input,
            communications_commitment: Some((
                communication_commitment,
                call.communication_commitment_rand,
            )),
            key_location: call.key_location.clone(),
        };

        ProofPreimageVersioned::V1(proof)
    }
}

impl<D: DB> ContractCalls<ProofPreimage, D> {
    pub fn new<R: Rng + CryptoRng + ?Sized>(rng: &mut R) -> Self {
        ContractCalls {
            calls: Vec::new(),
            binding_commitment: rng.gen(),
        }
    }

    pub fn add_call<P>(&self, call: ContractCallPrototype<D>) -> Self
    where
        P: ContractCallExt<D, Proof = P>,
    {
        let mut io_repr = Vec::with_capacity(
            call.input.value_only_field_size() + call.output.value_only_field_size(),
        );
        call.input.value_only_field_repr(&mut io_repr);
        call.output.value_only_field_repr(&mut io_repr);
        let communication_commitment =
            transient_commit(&io_repr, call.communication_commitment_rand);

        let proof = P::construct_proof(&call, communication_commitment);
        let call = ContractCall {
            address: call.address,
            entry_point: call.entry_point,
            guaranteed_transcript: call.guaranteed_public_transcript,
            fallible_transcript: call.fallible_public_transcript,
            communication_commitment,
            proof,
            binding_input_data: None,
        };
        // We add the call:
        // - Directly before the first call *claimed* by it
        // - At the end otherwise.
        let mut calls = Vec::new();
        let mut already_inserted = false;
        fn references<D: DB>(
            caller: &ContractCall<ProofPreimage, D>,
            callee: &ContractAction<ProofPreimage, D>,
        ) -> bool {
            match callee {
                ContractAction::Call(call) => caller
                    .guaranteed_transcript
                    .iter()
                    .chain(caller.fallible_transcript.iter())
                    .flat_map(|t| t.effects.claimed_contract_calls.iter())
                    .any(|&(_seq, addr, ep_hash, cc)| {
                        addr == call.address
                            && cc == call.communication_commitment
                            && ep_hash == call.entry_point.ep_hash()
                    }),
                _ => false,
            }
        }
        for c in self.calls.iter().cloned() {
            if !already_inserted && references(&call, &c) {
                calls.push(call.clone().into());
                already_inserted = true;
            }
            calls.push(c);
        }
        if !already_inserted {
            calls.push(call.into());
        }
        ContractCalls {
            calls,
            binding_commitment: self.binding_commitment,
        }
    }

    pub fn add_deploy(&self, deploy: ContractDeploy<D>) -> Self {
        ContractCalls {
            calls: self
                .calls
                .iter()
                .cloned()
                .chain(once(deploy.into()))
                .collect(),
            binding_commitment: self.binding_commitment,
        }
    }

    pub fn add_maintenance_update(&self, upd: MaintenanceUpdate) -> Self {
        ContractCalls {
            calls: self.calls.iter().cloned().chain(once(upd.into())).collect(),
            binding_commitment: self.binding_commitment,
        }
    }

    fn binding_randomness(&self) -> PedersenRandomness {
        self.binding_commitment
    }
}

#[derive(Debug)]
pub struct PreTranscript<'a, D: DB> {
    pub context: &'a QueryContext<D>,
    pub program: &'a [Op<ResultModeVerify, D>],
    pub comm_comm: Option<Fr>,
}

impl<D: DB> PreTranscript<'_, D> {
    fn no_checkpoints(&self) -> usize {
        self.program
            .iter()
            .filter(|op| matches!(op, Op::Ckpt))
            .count()
    }

    // 0-indexed!
    fn run_to_ckpt_no(
        &self,
        mut n: usize,
        params: &LedgerParameters,
    ) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
        n += 1;
        let prog = self
            .program
            .iter()
            .cloned()
            .take_while(|op| {
                if matches!(op, Op::Ckpt) {
                    n -= 1;
                    n != 0
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();
        self.context
            .query(&prog, None, &params.cost_model.runtime_cost_model)
    }

    fn guaranteed_budget(&self, params: &LedgerParameters) -> u128 {
        let est_size = PROOF_SIZE
            + Serializable::serialized_size(&(
                ContractAddress::default(),
                Effects::default(),
                &self.program.iter().collect::<Vec<_>>(),
                &self.comm_comm,
            ));
        est_size as u128 * params.limits.guaranteed_cost_limit_per_byte as u128
    }

    // 1-indexed!
    #[allow(clippy::type_complexity)]
    fn split_at(
        &self,
        mut n: usize,
        params: &LedgerParameters,
    ) -> Result<(Option<Transcript<D>>, Option<Transcript<D>>), TranscriptRejected<D>> {
        let mut prog_guaranteed = Vec::new();
        let mut prog_fallible = Vec::new();
        for op in self.program {
            if n > 0 && matches!(op, Op::Ckpt) {
                n -= 1;
            }
            if n == 0 {
                prog_fallible.push(op.clone());
            } else {
                prog_guaranteed.push(op.clone());
            }
        }
        let guaranteed_res = self.context.query(
            &prog_guaranteed,
            None,
            &params.cost_model.runtime_cost_model,
        )?;
        let mut continuation_context = guaranteed_res.context.clone();
        continuation_context.effects = Effects::default();
        let fallible_res = continuation_context.query(
            &prog_fallible,
            None,
            &params.cost_model.runtime_cost_model,
        )?;
        let mk_transcript = |prog: Vec<Op<ResultModeVerify, D>>,
                             res: QueryResults<ResultModeVerify, D>| {
            if prog.is_empty() {
                None
            } else {
                Some(Transcript {
                    gas: res.gas_heuristic(),
                    effects: res.context.effects.clone(),
                    program: prog,
                    version: <Transcript<D> as Versioned>::VERSION,
                })
            }
        };
        Ok((
            mk_transcript(prog_guaranteed, guaranteed_res),
            mk_transcript(prog_fallible, fallible_res),
        ))
    }
}

trait QueryResultsExt {
    fn gas_heuristic(&self) -> u64;
    fn cost_heuristic(&self, params: &LedgerParameters) -> u128;
}

impl<D: DB> QueryResultsExt for QueryResults<ResultModeVerify, D> {
    fn gas_heuristic(&self) -> u64 {
        self.gas_cost + self.gas_cost / 5
    }

    fn cost_heuristic(&self, params: &LedgerParameters) -> u128 {
        self.gas_heuristic() as u128 * params.ticket_cost_multiplier as u128
            / params.ticket_cost_divisor as u128
    }
}

pub fn communication_commitment(input: AlignedValue, output: AlignedValue, rand: Fr) -> Fr {
    transient_commit(&AlignedValue::concat([&input, &output]), rand)
}

pub type TranscriptPair<D> = (Option<Transcript<D>>, Option<Transcript<D>>);

pub fn partition_transcripts<D: DB>(
    calls: &[PreTranscript<'_, D>],
    params: &LedgerParameters,
) -> Result<Vec<TranscriptPair<D>>, PartitionFailure<D>> {
    let n = calls.len();
    // Step 1: Generate a call graph between `calls`. Assert that this is a forest (no cycles,
    //      no multiple parents).

    // Gather full runs to observe all calls
    let no_ckpts = calls
        .iter()
        .map(PreTranscript::no_checkpoints)
        .collect::<Vec<_>>();
    let full_runs = calls
        .iter()
        .zip(no_ckpts.iter())
        .map(|(pt, n)| pt.run_to_ckpt_no(*n, params))
        .collect::<Result<Vec<_>, _>>()?;
    // Graph is a Vec of Vecs, where call_graph[i] = [a, b, c] means that calls[i] calls calls[a],
    // calls[b] and calls[c]
    let call_graph = full_runs
        .iter()
        .map(|qr| {
            let claimed_commitments = qr
                .context
                .effects
                .claimed_contract_calls
                .iter()
                .map(|(_seq, _addr, _ep_hash, cm)| *cm)
                .collect::<Vec<_>>();
            (0..n)
                .filter(|i| {
                    calls[*i]
                        .comm_comm
                        .map(|comm| claimed_commitments.contains(&comm))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    // Identify cycles by starting at each node in turn, and asserting that the set of reachable
    // nodes does not contain itself
    //
    // Note that this also identifies some non-cycles, such as A -> B, A -> C, B -> D, B -> D,
    // but this is fine as these are also not allowed.
    for i in 0..n {
        let mut visited = vec![i];
        let mut reachable = call_graph[i].clone();
        while let Some(next) = reachable.pop() {
            if visited.contains(&next) {
                return Err(PartitionFailure::NonForest);
            }
            visited.push(next);
            reachable.extend(&call_graph[next]);
        }
    }

    // Step 2: Identify root nodes in the DAG.
    // Identify multiple parents at the same time. For each node, count how many other nodes
    // reference it. If that's 0, it's a root node. If it's >1, we don't have a forest.
    let n_callers = (0..n)
        .map(|i| (0..n).filter(|j| call_graph[*j].contains(&i)).count())
        .collect::<Vec<_>>();
    if n_callers.iter().any(|n| *n > 1) {
        return Err(PartitionFailure::NonForest);
    }
    let root_nodes = n_callers
        .iter()
        .enumerate()
        .filter(|(_, n)| **n == 0)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    // Step 3: For each root node, compute the guaranteed section budget of its closure.
    let closures = root_nodes
        .iter()
        .map(|r| {
            let mut visited = vec![];
            let mut frontier = vec![*r];
            while let Some(item) = frontier.pop() {
                visited.push(item);
                frontier.extend(&call_graph[item]);
            }
            visited
        })
        .collect::<Vec<_>>();
    let closure_budgets = closures
        .iter()
        .map(|closure| {
            closure
                .iter()
                .map(|i| calls[*i].guaranteed_budget(params))
                .sum::<u128>()
        })
        .collect::<Vec<_>>();

    // Step 4: Partition the root nodes:
    //      4a: Split root nodes on `ckpt`s.
    //      4b. Run up to `ckpt` and determine calls that are included. Run those entirely.
    //      4c. Determine the latest `ckpt` for which this fits into the previously computed
    //          budget.

    // preliminary_results is a vec that, for each root, contains a pair consisting of:
    // the number of checkpoints to include in the guaranteed section (0 indicating entirely in the
    // fallible section), and a vec of the indices of the callees that are included in the
    // guaranteed section (a subset of the corresponding entry in call_graph).
    let preliminary_results = root_nodes
        .iter()
        .enumerate()
        .map(|(root_n, &root)| {
            // Run through the number of sections to put into the guaranteed transcript.
            // Start with the number of checkpoints + 1 (we have one more section than checkpoints),
            // end at 1. If none pass, 0 make it into the guaranteed section.
            for n in (1..no_ckpts[root] + 2).rev() {
                let partial_res = calls[root].run_to_ckpt_no(n - 1, params)?;
                let claimed = partial_res
                    .context
                    .effects
                    .claimed_contract_calls
                    .iter()
                    .map(|(_seq, _addr, _ep_hash, comm)| *comm)
                    .collect::<Vec<_>>();
                let claimed_idx = calls
                    .iter()
                    .enumerate()
                    .filter(|(_, pt)| {
                        pt.comm_comm
                            .map(|cc| claimed.contains(&cc))
                            .unwrap_or(false)
                    })
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                let mut required_budget = partial_res.cost_heuristic(params);
                let mut frontier = claimed_idx.clone();
                while let Some(next) = frontier.pop() {
                    required_budget += full_runs[next].cost_heuristic(params);
                    frontier.extend(&call_graph[next]);
                }
                if required_budget <= closure_budgets[root_n] {
                    return Ok((n, claimed_idx));
                }
            }
            Ok::<_, PartitionFailure<D>>((0, vec![]))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut sections_in_guaranteed = vec![0; n];
    for (root_n, &root) in root_nodes.iter().enumerate() {
        sections_in_guaranteed[root] = preliminary_results[root_n].0;
        let mut frontier = preliminary_results[root_n].1.clone();
        while let Some(next) = frontier.pop() {
            sections_in_guaranteed[next] = no_ckpts[next] + 1;
            frontier.extend(&call_graph[next]);
        }
    }

    Ok(sections_in_guaranteed
        .into_iter()
        .enumerate()
        .map(|(i, sections)| calls[i].split_at(sections, params))
        .collect::<Result<Vec<_>, _>>()?)
}
