use crate::error::MalformedTransaction;
use crate::serialize::Serializable;
use crate::structure::{
    ContractAction, ContractCall, ContractCalls, ContractDeploy, LedgerState, MAX_ZSWAP_PART_BITS,
    MaintenanceUpdate, PedersenStage, Proofish, SingleUpdate, Transaction,
};
use crate::transient_crypto::commitment::{Pedersen, PedersenRandomness};
use crate::transient_crypto::curve::{EmbeddedFr, EmbeddedGroupAffine, Fr};
use crate::transient_crypto::repr::FieldRepr;
use crate::utils::serialize_or_data_mem;
use coin_structure::contract::Address as ContractAddress;
use onchain_runtime::ops::Op;
use onchain_runtime::state::{ContractOperation, ContractState, EntryPoint};
use onchain_runtime::transcript::Transcript;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Add;
use std::ops::Deref;
use zswap::storage::db::DB;

pub trait ContractStateExt {
    fn well_formed(&self, address: ContractAddress) -> Result<(), MalformedTransaction>;
}

impl<D: DB> ContractStateExt for ContractState<D> {
    #[instrument(skip(self))]
    fn well_formed(&self, address: ContractAddress) -> Result<(), MalformedTransaction> {
        for a in self.operations.iter() {
            a.1.well_formed(address, a.0.as_ref())?;
        }
        if self.maintenance_authority.counter != 0 {
            return Err(MalformedTransaction::NotNormalized);
        }
        trace!("well formed");
        Ok(())
    }
}

pub trait ContractOperationExt {
    fn well_formed(
        &self,
        address: ContractAddress,
        operation: EntryPoint,
    ) -> Result<(), MalformedTransaction>;
}

impl ContractOperationExt for ContractOperation {
    #[instrument(skip(self))]
    fn well_formed(
        &self,
        address: ContractAddress,
        operation: EntryPoint,
    ) -> Result<(), MalformedTransaction> {
        match &self.v2 {
            Some(_) => Ok(()),
            None => {
                if cfg!(feature = "test-utilities") {
                    warn!("no verifier key set, ignoring in test mode");
                    Ok(())
                } else {
                    warn!("no verifier key set");
                    Err(MalformedTransaction::VerifierKeyNotSet {
                        address,
                        operation: operation.into(),
                    })
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct WellFormedStrictness {
    pub enforce_balancing: bool,
    pub verify_native_proofs: bool,
    pub verify_contract_proofs: bool,
    pub verify_signatures: bool,
    pub enforce_limits: bool,
}

impl Default for WellFormedStrictness {
    fn default() -> Self {
        WellFormedStrictness {
            enforce_balancing: true,
            verify_native_proofs: true,
            verify_contract_proofs: true,
            verify_signatures: true,
            enforce_limits: true,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum ContractLifetime {
    Guaranteed,
    Fallible,
    Both,
}

impl<P: Proofish<D>, D: DB> ContractCall<P, D> {
    fn lifetime(&self) -> ContractLifetime {
        match (
            self.guaranteed_transcript.is_some(),
            self.fallible_transcript.is_some(),
        ) {
            (true, false) | (false, false) => ContractLifetime::Guaranteed,
            (false, true) => ContractLifetime::Fallible,
            (true, true) => ContractLifetime::Both,
        }
    }
}

impl<P: Proofish<D>, D: DB> Transaction<P, D>
where
    Transaction<P, D>: Serializable,
{
    // All checks that can be done without a state.
    #[instrument(skip(self, ref_state))]
    /// Checks if a transaction is well-formed, performing all checks possible
    /// with a moderately stale reference state.
    ///
    /// `enforce_balancing` being set to [None] permits imbalanced transactions,
    /// while [Some]([usize]) informs the balance check of the serialized
    /// transaction size to use for transaction size cost calculation.
    pub fn well_formed(
        &self,
        ref_state: &LedgerState<D>,
        strictness: WellFormedStrictness,
    ) -> Result<(), MalformedTransaction> {
        if strictness.enforce_limits
            && Transaction::serialized_size(self) as u64
                > ref_state.parameters.limits.transaction_byte_limit
        {
            return Err(MalformedTransaction::TransactionTooLarge {
                tx_size: Transaction::serialized_size(self),
                limit: ref_state.parameters.limits.transaction_byte_limit,
            });
        }
        match self {
            Transaction::Standard(stx) => {
                let fees = self.fees(&ref_state.parameters)?;
                let guaranteed_zswap_com = self.well_formed_segment(strictness, true, fees)?;
                let fallible_zswap_com = self.well_formed_segment(strictness, false, 0)?;
                if let Some(calls) = &stx.contract_calls {
                    calls.well_formed(ref_state, strictness)?;
                }
                let com = stx
                    .contract_calls
                    .iter()
                    .map(|call| Into::<Pedersen>::into(call.binding_commitment.clone()))
                    .fold(guaranteed_zswap_com + fallible_zswap_com, Add::add);
                let com_unit = Pedersen(EmbeddedGroupAffine::identity());
                let com_rc = Pedersen::commit(
                    &Fr::from(0u64),
                    &EmbeddedFr::from(0u64),
                    &stx.binding_randomness,
                );
                if com - com_rc != com_unit {
                    warn!("transaction malformed: invalid binding commitment");
                    return Err(MalformedTransaction::BindingCommitmentOpeningInvalid);
                }
                debug!("transaction well-formed");
                Ok(())
            }
            Transaction::ClaimMint(mtx) => {
                P::zswap_mint_well_formed(&mtx.mint).map_err(MalformedTransaction::Zswap)
            }
        }
    }

    #[allow(unstable_name_collisions)] // is_sorted will behave identically
    #[instrument(skip(self))]
    fn well_formed_segment(
        &self,
        strictness: WellFormedStrictness,
        guaranteed: bool,
        fees: u128,
    ) -> Result<Pedersen, MalformedTransaction> {
        let segment = if guaranteed { 0 } else { 1 };
        match self {
            Transaction::Standard(stx) => {
                let coins = if guaranteed {
                    Some(&stx.guaranteed_coins)
                } else {
                    stx.fallible_coins.as_ref()
                };
                let n_parts = match coins {
                    Some(offer) => offer.inputs.len() + offer.outputs.len() + offer.transient.len(),
                    None => 0,
                };
                if n_parts >= (1 << MAX_ZSWAP_PART_BITS) {
                    return Err(MalformedTransaction::TooManyZswapEntries);
                };
                let zswap_com = coins
                    .map(|coins| {
                        if strictness.verify_native_proofs {
                            P::zswap_well_formed(coins, segment)
                        } else {
                            <() as Proofish<D>>::zswap_well_formed(&coins.erase_proofs(), segment)
                        }
                    })
                    .transpose()?
                    .unwrap_or_else(|| Pedersen::from(PedersenRandomness::from(0)));
                if strictness.enforce_balancing {
                    for (ty, val) in self.imbalances(guaranteed, Some(fees)).iter() {
                        if *val < 0 {
                            warn!("transaction imbalanced");
                            return Err(MalformedTransaction::Unbalanced(*ty, *val));
                        }
                    }
                }
                let mut nullifiers_remaining: BTreeSet<_> = coins
                    .iter()
                    .flat_map(|o| o.inputs.iter())
                    .filter_map(|inp| inp.contract_address.map(|address| (inp.nullifier, address)))
                    .chain(
                        coins
                            .iter()
                            .flat_map(|o| o.transient.iter())
                            .filter_map(|io| {
                                io.contract_address.map(|address| (io.nullifier, address))
                            }),
                    )
                    .collect();
                let mut comms_remaining: BTreeSet<_> = coins
                    .iter()
                    .flat_map(|o| o.outputs.iter())
                    .filter_map(|out| out.contract_address.map(|address| (out.coin_com, address)))
                    .chain(
                        coins
                            .iter()
                            .flat_map(|o| o.transient.iter())
                            .filter_map(|io| {
                                io.contract_address.map(|address| (io.coin_com, address))
                            }),
                    )
                    .collect();
                let mut outputs_unclaimed: BTreeSet<_> = coins
                    .iter()
                    .flat_map(|o| o.outputs.iter())
                    .map(|out| out.coin_com)
                    .chain(
                        coins
                            .iter()
                            .flat_map(|o| o.transient.iter())
                            .map(|io| io.coin_com),
                    )
                    .collect();
                for cs in stx.contract_calls.iter() {
                    let mut unclaimed_contracts: BTreeMap<_, _> = cs
                        .calls()
                        .enumerate()
                        .map(|(seq, call)| {
                            (
                                (
                                    call.address,
                                    call.entry_point.ep_hash(),
                                    call.communication_commitment,
                                ),
                                (seq, call.lifetime()),
                            )
                        })
                        .collect();
                    for (idx, call) in cs.calls().enumerate() {
                        let maybe_transcript = if guaranteed {
                            &call.guaranteed_transcript
                        } else {
                            &call.fallible_transcript
                        };
                        let transcript = match maybe_transcript.as_ref() {
                            Some(t) => t,
                            None => continue,
                        };
                        for &nul in transcript.effects.claimed_nullifiers.iter() {
                            if !nullifiers_remaining.remove(&(nul, call.address)) {
                                warn!(?nul, "transaction malformed: coin improperly nullified");
                                return Err(MalformedTransaction::ClaimNullifierFailed(nul));
                            }
                        }
                        for &receive in transcript.effects.claimed_receives.iter() {
                            if !comms_remaining.remove(&(receive, call.address)) {
                                warn!(?receive, "transaction malformed: coin improperly received");
                                return Err(MalformedTransaction::ClaimReceiveFailed(receive));
                            }
                        }
                        for &spend in transcript.effects.claimed_spends.iter() {
                            if !outputs_unclaimed.remove(&spend) {
                                warn!(?spend, "transaction malformed: coin improperly spent");
                                return Err(MalformedTransaction::ClaimSpendFailed(spend));
                            }
                        }
                        let mut claims = transcript
                            .effects
                            .claimed_contract_calls
                            .iter()
                            .collect::<Vec<_>>();
                        claims.sort();
                        // We iterate over claims by their sequence number.
                        // Then, we assert that the indices of the corresponding entry in the
                        // overall calls are strictly monotonically increasing, and all greater
                        // than the current calls index.
                        let mut last_idx = idx;
                        for &(_seq, address, entry_point, comm) in claims.into_iter() {
                            match (
                                unclaimed_contracts.remove(&(address, entry_point, comm)),
                                guaranteed,
                            ) {
                                // We can call guarnateed-only contracts from a guarnateed section,
                                // and fallible only contracts from a fallible section. A contract
                                // with both *cannot* be a callee.
                                (Some((idx, ContractLifetime::Guaranteed)), true)
                                | (Some((idx, ContractLifetime::Fallible)), false) => {
                                    if idx <= last_idx {
                                        return Err(MalformedTransaction::ClaimCallFailed {
                                            address,
                                            entry_point,
                                            comm,
                                        });
                                    } else {
                                        last_idx = idx;
                                    }
                                }
                                // In any other case, or if the callee doesn't exist, reject the
                                // call.
                                _ => {
                                    return Err(MalformedTransaction::ClaimCallFailed {
                                        address,
                                        entry_point,
                                        comm,
                                    });
                                }
                            }
                        }
                    }
                }
                if let Some(&(com, _)) = comms_remaining.iter().next() {
                    warn!(?com, "transaction malformed: coin not received");
                    return Err(MalformedTransaction::UnclaimedCoinCom(com));
                }
                if let Some(&(nul, _)) = nullifiers_remaining.iter().next() {
                    warn!(?nul, "transaction malformed: spend not authorized");
                    return Err(MalformedTransaction::UnclaimedNullifier(nul));
                }
                Ok(zswap_com)
            }
            // Do not call if it's not a standard tx!
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "verifying")]
impl<D: DB> ContractDeploy<D> {
    pub(crate) fn well_formed(&self) -> Result<(), MalformedTransaction> {
        self.initial_state.well_formed(self.address())
    }
}

impl<P: Proofish<D>, D: DB> ContractAction<P, D> {
    fn well_formed(
        &self,
        ref_state: &LedgerState<D>,
        strictness: WellFormedStrictness,
        parent: &ContractCalls<P, D>,
    ) -> Result<(), MalformedTransaction> {
        match self {
            ContractAction::Call(call) => call.well_formed(ref_state, strictness, parent),
            ContractAction::Deploy(deploy) => deploy.well_formed(),
            ContractAction::Maintain(upd) => upd.well_formed(ref_state, strictness),
        }
    }
}

impl MaintenanceUpdate {
    pub(crate) fn well_formed<D: DB>(
        &self,
        ref_state: &LedgerState<D>,
        strictness: WellFormedStrictness,
    ) -> Result<(), MalformedTransaction> {
        let cstate = ref_state
            .index(self.address)
            .ok_or_else(|| MalformedTransaction::ContractNotPresent(self.address))?;
        let authority = cstate.maintenance_authority;
        let data = self.data_to_sign();
        // Ensure ordering, and that no party signed two votes
        if !self.signatures.windows(2).all(|xs| xs[0].0 < xs[1].0) {
            return Err(MalformedTransaction::NotNormalized);
        }
        // Ensure that any new committee uses an incremented counter
        if self
            .updates
            .iter()
            .filter_map(|up| match up {
                SingleUpdate::ReplaceAuthority(new_auth) => Some(new_auth),
                _ => None,
            })
            .any(|auth| auth.counter != self.counter.saturating_add(1))
        {
            return Err(MalformedTransaction::NotNormalized);
        }
        for (idx, sig) in self.signatures.iter() {
            let vk = authority.committee.get(*idx as usize).ok_or(
                MalformedTransaction::KeyNotInCommittee {
                    address: self.address,
                    key_id: *idx as usize,
                },
            )?;
            if strictness.verify_signatures && !vk.verify(&data, sig) {
                return Err(MalformedTransaction::InvalidCommitteeSignature {
                    address: self.address,
                    key_id: *idx as usize,
                });
            }
        }
        if self.signatures.len() < authority.threshold as usize {
            return Err(MalformedTransaction::ThresholdMissed {
                address: self.address,
                signatures: self.signatures.len(),
                threshold: authority.threshold as usize,
            });
        }
        Ok(())
    }
}

impl<P: Proofish<D>, D: DB> ContractCalls<P, D> {
    fn well_formed(
        &self,
        ref_state: &LedgerState<D>,
        strictness: WellFormedStrictness,
    ) -> Result<(), MalformedTransaction> {
        self.calls
            .iter()
            .try_for_each(|c| c.well_formed(ref_state, strictness, self))?;
        self.binding_commitment
            .valid(&Self::challenge_pre_for(&self.calls))?;
        Ok(())
    }
}

impl<P: Proofish<D>, D: DB> ContractCall<P, D> {
    pub(crate) fn well_formed(
        &self,
        ref_state: &LedgerState<D>,
        strictness: WellFormedStrictness,
        parent: &ContractCalls<P, D>,
    ) -> Result<(), MalformedTransaction> {
        if let Some(fallible) = &self.fallible_transcript {
            if fallible.program.first() != Some(&Op::Ckpt) && self.guaranteed_transcript.is_some() {
                return Err(MalformedTransaction::FallibleWithoutCheckpoint);
            }
        }
        for transcript in [
            self.guaranteed_transcript.as_ref(),
            self.fallible_transcript.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            for window in transcript.program.windows(2) {
                if let (Op::Noop { .. }, Op::Noop { .. }) = (&window[0], &window[1]) {
                    return Err(MalformedTransaction::NotNormalized);
                }
            }
        }

        if strictness.verify_contract_proofs {
            let cstate = ref_state
                .index(self.address)
                .ok_or_else(|| MalformedTransaction::ContractNotPresent(self.address))?;
            let op = cstate.operations.get(&self.entry_point).ok_or_else(|| {
                MalformedTransaction::VerifierKeyNotPresent {
                    address: self.address,
                    operation: self.entry_point.clone(),
                }
            })?;
            if op.v2.is_some() {
                if self.guaranteed_transcript.is_some()
                    && !matches!(&self.guaranteed_transcript, Some(Transcript { version: Some(version), ..}) if version.major == 2 && version.minor <= 2)
                {
                    return Err(MalformedTransaction::GuaranteedTranscriptVersion {
                        op_version: "V2".to_string(),
                    });
                }
                if self.fallible_transcript.is_some()
                    && !matches!(&self.fallible_transcript, Some(Transcript { version: Some(version), ..}) if version.major == 2 && version.minor <= 2)
                {
                    return Err(MalformedTransaction::FallibleTranscriptVersion {
                        op_version: "V2".to_string(),
                    });
                }
            }
            P::proof_verify(op.deref(), &self.proof, self.public_inputs(parent), self)?;
            trace!("call valid");
        }
        Ok(())
    }

    // NOTE: The proof should receive the following inputs for binding purposes:
    //  - The contract address
    //  - The contract entry point
    //  - Both transcript parts's declared gas costs, and effects
    //  - The count of instructions in the guaranteed transcript
    //  - The parent ContractCalls's `binding_commitment.commitment`.
    // These need to be *truely* binding (see note at the bottom of this file!)
    // In addition, it should receive
    //  - The communication commitment
    //  - The transcript
    pub(crate) fn public_inputs(&self, parent: &ContractCalls<P, D>) -> Vec<Fr> {
        let mut binding_input = Vec::new();
        let raw_data = self.binding_input_data.as_ref();
        serialize_or_data_mem(
            &self.address,
            raw_data.map(|d| &d.address),
            &mut binding_input,
        );
        serialize_or_data_mem(&self.entry_point, None::<&[u8]>, &mut binding_input);
        serialize_or_data_mem(
            &self
                .guaranteed_transcript
                .as_ref()
                .map(|t| &t.gas)
                .unwrap_or(&0),
            None::<&[u8]>,
            &mut binding_input,
        );
        if let Some(t) = self.guaranteed_transcript.as_ref() {
            serialize_or_data_mem(
                &t.effects,
                raw_data.and_then(|d| d.guaranteed_annot.effects_raw.as_ref()),
                &mut binding_input,
            );
        } else {
            // Backwards-compatible with `Effects::default` serialization, as this may be unstable.
            binding_input.extend(vec![0u8; 20]);
        }
        if let Some(t) = self.fallible_transcript.as_ref() {
            serialize_or_data_mem(&Some(t.gas), None::<&[u8]>, &mut binding_input);
            // Discrimant for `Some`.
            binding_input.push(1);
            serialize_or_data_mem(
                &t.effects,
                raw_data.and_then(|d| d.fallible_annot.effects_raw.as_ref()),
                &mut binding_input,
            );
        } else {
            // (None, None)
            binding_input.extend(&[0, 0]);
        }
        let len = self
            .guaranteed_transcript
            .as_ref()
            .map(|t| t.program.len() as u64)
            .unwrap_or_default();
        serialize_or_data_mem(&len, None::<&[u8]>, &mut binding_input);
        serialize_or_data_mem(
            &Into::<Pedersen>::into(parent.binding_commitment.clone()),
            None::<&[u8]>,
            &mut binding_input,
        );
        let mut hasher = Sha256::new();
        hasher.update(&binding_input[..]);
        let binding_input = Fr::from_le_bytes(&hasher.finalize()[..31])
            .expect("Trimmed persistent hash should fall in Fr");
        let mut res = vec![binding_input];
        res.push(self.communication_commitment);
        if let Some(guaranteed) = self.guaranteed_transcript.as_ref() {
            for op in guaranteed.program.iter() {
                op.field_repr(&mut res);
            }
        }
        if let Some(fallible) = self.fallible_transcript.as_ref() {
            for op in fallible.program.iter() {
                op.field_repr(&mut res);
            }
        }
        res
    }
}
