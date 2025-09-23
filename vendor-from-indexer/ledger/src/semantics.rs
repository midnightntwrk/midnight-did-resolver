use crate::base_crypto::hash::HashOutput;
use crate::error::{SystemTransactionError, TransactionInvalid};
use crate::storage::db::DB;
use crate::storage::storage::Map;
use crate::structure::{
    ContractAction, ContractCalls, LedgerState, OutputInstruction, Proofish, SingleUpdate,
    SystemTransaction, Transaction,
};
use crate::transient_crypto::merkle_tree::MerkleTree;
use coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN, Nonce, TokenType};
use coin_structure::contract::Address as ContractAddress;
use onchain_runtime::context::{BlockContext, QueryContext};
use onchain_runtime::cost_model::DUMMY_COST_MODEL;
use onchain_runtime::state::ContractOperation;
use std::ops::Deref;
use zswap::keys::SecretKeys;
use zswap::local::State as ZswapLocalState;
use zswap::{AuthorizedMint, Offer};

pub(crate) fn whitelist_matches(
    whitelist: &Option<Map<ContractAddress, ()>>,
    addr: &ContractAddress,
) -> bool {
    match whitelist {
        Some(wl) => wl.contains_key(addr),
        None => true,
    }
}

#[derive(Debug, Default)]
pub struct TransactionContext<D: DB> {
    pub ref_state: LedgerState<D>,
    pub block_context: BlockContext,
    pub whitelist: Option<Map<ContractAddress, ()>>,
}

#[derive(Debug)]
pub enum TransactionResult<D: DB> {
    Success,
    PartialSuccess(TransactionInvalid<D>),
    Failure(TransactionInvalid<D>),
}

impl<D: DB> From<TransactionResult<D>> for ErasedTransactionResult {
    fn from(res: TransactionResult<D>) -> ErasedTransactionResult {
        match res {
            TransactionResult::Success => ErasedTransactionResult::Success,
            TransactionResult::PartialSuccess(_) => ErasedTransactionResult::PartialSuccess,
            TransactionResult::Failure(_) => ErasedTransactionResult::Failure,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErasedTransactionResult {
    Success,
    PartialSuccess,
    Failure,
}

pub trait ZswapLocalStateExt<D: DB> {
    fn apply_system_tx(&self, secret_keys: &SecretKeys, tx: &SystemTransaction) -> Self;
    fn apply_tx<P: Proofish<D>>(
        &self,
        secret_keys: &SecretKeys,
        tx: &Transaction<P, D>,
        res: ErasedTransactionResult,
    ) -> Self;
}

impl<D: DB> ZswapLocalStateExt<D> for ZswapLocalState<D> {
    fn apply_system_tx(&self, secret_keys: &SecretKeys, tx: &SystemTransaction) -> Self {
        match tx {
            SystemTransaction::PayFromTreasury {
                outputs,
                nonce,
                token_type,
            } => {
                let mut res = self.clone();
                for OutputInstruction { amount, target_key } in outputs {
                    let coin = CoinInfo {
                        value: *amount,
                        type_: *token_type,
                        nonce: Nonce(*nonce),
                    };
                    res = self.apply_mint(
                        secret_keys,
                        &AuthorizedMint {
                            coin,
                            recipient: *target_key,
                            proof: (),
                        },
                    );
                }
                res
            }
            _ => self.clone(),
        }
    }

    fn apply_tx<P: Proofish<D>>(
        &self,
        secret_keys: &SecretKeys,
        tx: &Transaction<P, D>,
        res: ErasedTransactionResult,
    ) -> Self {
        match (tx, res) {
            (_, ErasedTransactionResult::Failure) => self.clone(),
            (Transaction::Standard(stx), ErasedTransactionResult::PartialSuccess) => {
                self.apply(secret_keys, &stx.guaranteed_coins)
            }
            (Transaction::Standard(stx), ErasedTransactionResult::Success) => {
                let post_guaranteed = self.apply(secret_keys, &stx.guaranteed_coins);
                if let Some(fallible) = &stx.fallible_coins {
                    post_guaranteed.apply(secret_keys, fallible)
                } else {
                    post_guaranteed
                }
            }
            (Transaction::ClaimMint(mint), ErasedTransactionResult::Success) => {
                self.apply_mint(secret_keys, &mint.mint)
            }
            (Transaction::ClaimMint(mint), ErasedTransactionResult::PartialSuccess) => {
                // NOTE: Can only be reached through incorrect usage! Mint's can't partially
                // succeed
                error!(
                    ?mint.mint,
                    "processing partial success of mint, that isn't possible!"
                );
                self.clone()
            }
        }
    }
}

impl<D: DB> LedgerState<D>
where
    MerkleTree<Option<ContractAddress>>: 'static,
{
    #[instrument(skip(self))]
    fn native_issue_unbalanced(
        &self,
        target: coin_structure::coin::PublicKey,
        token_type: TokenType,
        nonce: HashOutput,
        value: u128,
    ) -> Result<Self, SystemTransactionError> {
        let cm = coin_structure::coin::Info {
            value,
            nonce: coin_structure::coin::Nonce(nonce),
            type_: token_type,
        }
        .commitment(&coin_structure::transfer::Recipient::User(target));
        if self.zswap.coin_coms_set.contains_key(&cm) {
            Err(SystemTransactionError::CommitmentAlreadyPresent(cm))
        } else {
            let zswap = zswap::ledger::State {
                coin_coms: self
                    .zswap
                    .coin_coms
                    .update(self.zswap.first_free, &cm, None),
                coin_coms_set: self.zswap.coin_coms_set.insert(cm, ()),
                first_free: self.zswap.first_free + 1,
                ..self.zswap.clone()
            };
            Ok(LedgerState {
                zswap,
                ..self.clone()
            })
        }
    }

    #[instrument(skip(self, tx))]
    pub fn apply_system_tx(&self, tx: &SystemTransaction) -> Result<Self, SystemTransactionError> {
        match tx {
            SystemTransaction::OverwriteParameters(new_params) => Ok(LedgerState {
                parameters: new_params.clone(),
                ..self.clone()
            }),
            SystemTransaction::Mint(outputs) => {
                let total = outputs
                    .iter()
                    .map(|o| o.amount)
                    .try_fold(0u128, |acc, a| acc.checked_add(a));
                let total = match total {
                    Some(v) if v <= self.unminted_native_token_supply => v,
                    _ => {
                        error!(?total, ?outputs, supply = ?self.unminted_native_token_supply, "[priviledged] mint rejected due to insufficient unminted supply");
                        return Err(SystemTransactionError::IllegalMint {
                            amount: total,
                            supply: self.unminted_native_token_supply,
                        });
                    }
                };
                info!(?outputs, supply_before = ?self.unminted_native_token_supply, "[priviledged] native token mint");
                let unminted_native_token_supply = self.unminted_native_token_supply - total;
                let mut unclaimed_mints = self.unclaimed_mints.clone();
                for OutputInstruction { amount, target_key } in outputs {
                    let at_key = unclaimed_mints
                        .get(target_key)
                        .cloned()
                        .unwrap_or_else(Map::new);
                    let curr_value = at_key.get(&NATIVE_TOKEN).cloned().unwrap_or(0);
                    let new_key = at_key.insert(NATIVE_TOKEN, curr_value.saturating_add(*amount));
                    unclaimed_mints = unclaimed_mints.insert(*target_key, new_key);
                }
                let state = LedgerState {
                    unminted_native_token_supply,
                    unclaimed_mints,
                    ..self.clone()
                };
                Ok(state)
            }
            SystemTransaction::MintToTreasury { amount } => {
                if *amount > self.unminted_native_token_supply {
                    error!(?amount, supply = ?self.unminted_native_token_supply, "[priviledged] mint to treasury rejected due to insufficient unminted supply");
                    return Err(SystemTransactionError::IllegalMint {
                        amount: Some(*amount),
                        supply: self.unminted_native_token_supply,
                    });
                }
                info!(?amount, supply_before = ?self.unminted_native_token_supply, "[priviledged] native token mint to treasury");
                let mut treasury = self.treasury.clone();
                let native_token = treasury
                    .get(&NATIVE_TOKEN)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(*amount);
                treasury = treasury.insert(NATIVE_TOKEN, native_token);
                Ok(LedgerState {
                    unminted_native_token_supply: self.unminted_native_token_supply - *amount,
                    treasury,
                    ..self.clone()
                })
            }
            SystemTransaction::PayFromTreasury {
                outputs,
                token_type,
                nonce,
            } => {
                let mut treasury = self.treasury.clone();
                let tt_amount = treasury.get(token_type).copied().unwrap_or(0);
                let req_total = outputs
                    .iter()
                    .map(|o| o.amount)
                    .try_fold(0u128, |acc, a| acc.checked_add(a));
                let req_total = match req_total {
                    Some(v) if v <= tt_amount => v,
                    _ => {
                        error!(?req_total, ?token_type, ?outputs, supply = ?tt_amount, "[priviledged] treasury payout rejected due to insufficient funds");
                        return Err(SystemTransactionError::InsufficientTreasuryFunds {
                            requested: req_total,
                            actual: tt_amount,
                            token_type: *token_type,
                        });
                    }
                };
                info!(
                    ?req_total,
                    ?token_type,
                    ?outputs,
                    supply_before = tt_amount,
                    "[priviledged] authorized treasury payout"
                );
                treasury = treasury.insert(*token_type, tt_amount - req_total);
                let mut state = LedgerState {
                    treasury,
                    ..self.clone()
                };
                for output in outputs {
                    state = state.native_issue_unbalanced(
                        output.target_key,
                        *token_type,
                        *nonce,
                        output.amount,
                    )?;
                }
                Ok(state)
            }
        }
    }

    #[instrument(skip(self, tx, offer, calls, context))]
    fn apply_section<P: Proofish<D>>(
        &self,
        tx: &Transaction<P, D>,
        offer: Option<&Offer<P::LatestProof>>,
        calls: Option<&ContractCalls<P, D>>,
        // Three cases:
        // - Some(Some(fallible)) - We're applying the *guaranteed* section, and
        //                          there is a fallible Zswap offer present
        // - Some(None)           - We're applying the *guaranteed* section, and
        //                          there is no fallible Zswap offer present
        // - None                 - We're applying the fallible section
        //
        // This is used to check the fallible offer during the guaranteed
        // section, to prevent a merge from maliciously invalidating the fallible section
        guaranteed: Option<Option<&Offer<P::LatestProof>>>,
        context: &TransactionContext<D>,
    ) -> Result<Self, TransactionInvalid<D>> {
        let mut state: LedgerState<D> = self.clone();
        let indicies = if let Some(offer) = offer {
            let (zswap2, indicies) = state.zswap.try_apply(offer, context.whitelist.clone())?;
            state.zswap = zswap2;
            indicies
        } else {
            Map::new()
        };
        if let Some(Some(offer)) = guaranteed {
            // Check that the fallible zswap section *can* be applied, but discard the result
            // for now. This is to prevent merging from causing undue failures.
            state.zswap.try_apply(offer, context.whitelist.clone())?;
        }
        for call in calls.iter().flat_map(|calls| calls.calls.iter()) {
            match call {
                ContractAction::Call(call) => {
                    if !whitelist_matches(&context.whitelist, &call.address) {
                        continue;
                    } else if let Some(cstate) = state.index(call.address) {
                        let mut qcontext = QueryContext::new(cstate.data, call.address);
                        qcontext.com_indicies = indicies.clone();
                        qcontext.block = context.block_context.clone();
                        let transcript = if guaranteed.is_some() {
                            call.guaranteed_transcript.as_ref()
                        } else {
                            call.fallible_transcript.as_ref()
                        };
                        if let Some(transcript) = transcript {
                            let qcontext2 =
                                qcontext.run_transcript(transcript, &DUMMY_COST_MODEL)?; //TODO: use real cost model
                            if qcontext2.effects != transcript.effects {
                                return Err(TransactionInvalid::EffectsMismatch {
                                    declared: Box::new(transcript.effects.clone()),
                                    actual: Box::new(qcontext2.effects),
                                });
                            }
                            state = state.update_index(call.address, qcontext2);
                        }
                    } else {
                        warn!(?call.address, "contract not present");
                        return Err(TransactionInvalid::ContractNotPresent(call.address));
                    }
                }
                ContractAction::Deploy(deploy) => {
                    let addr = deploy.address();
                    if !whitelist_matches(&context.whitelist, &addr) || guaranteed.is_some() {
                        continue;
                    } else {
                        if state.contract.contains_key(&addr) {
                            return Err(TransactionInvalid::ContractAlreadyDeployed(addr));
                        }
                        state.contract = state.contract.insert(addr, deploy.initial_state.clone());
                    }
                }
                ContractAction::Maintain(upd) => {
                    let addr = upd.address;
                    if !whitelist_matches(&context.whitelist, &addr) || guaranteed.is_some() {
                        continue;
                    } else {
                        let mut cstate = match state.contract.get(&addr) {
                            Some(st) => st.clone(),
                            None => return Err(TransactionInvalid::ContractNotPresent(addr)),
                        };
                        if cstate.maintenance_authority.counter != upd.counter {
                            return Err(TransactionInvalid::ReplayCounterMismatch(addr));
                        }
                        cstate.maintenance_authority.counter =
                            cstate.maintenance_authority.counter.saturating_add(1);
                        for op in upd.updates.iter() {
                            match op {
                                SingleUpdate::ReplaceAuthority(auth) => {
                                    cstate.maintenance_authority = auth.clone()
                                }
                                SingleUpdate::VerifierKeyRemove(ep, ver) => {
                                    let mut op = match cstate.operations.get(ep) {
                                        Some(op) => op.deref().clone(),
                                        None => {
                                            return Err(TransactionInvalid::VerifierKeyNotFound(
                                                ep.clone(),
                                                ver.clone(),
                                            ));
                                        }
                                    };
                                    ver.rm_from(&mut op);
                                    if op == ContractOperation::new(None) {
                                        cstate.operations = cstate.operations.remove(ep);
                                    } else {
                                        cstate.operations =
                                            cstate.operations.insert(ep.clone(), op);
                                    }
                                }
                                SingleUpdate::VerifierKeyInsert(ep, vk) => {
                                    let mut op = match cstate.operations.get(ep) {
                                        Some(op) => (*op).clone(),
                                        None => ContractOperation::new(None),
                                    };
                                    if vk.as_version().has(&op) {
                                        return Err(TransactionInvalid::VerifierKeyAlreadyPresent(
                                            ep.clone(),
                                            vk.as_version(),
                                        ));
                                    }
                                    vk.insert_into(&mut op);
                                    cstate.operations = cstate.operations.insert(ep.clone(), op);
                                }
                            }
                        }
                        state.contract = state.contract.insert(addr, cstate);
                    }
                }
            }
        }
        // Increase the treasury balance by this section's imbalance, *not*
        // counting fees. That means that fees also get included here.
        state.treasury = tx.imbalances(guaranteed.is_some(), None).into_iter().fold(state.treasury.clone(), |treasury, (tt, val)| {
            if val > 0 {
                let val_after = treasury.get(&tt).copied().unwrap_or(0).saturating_add_signed(val);
                treasury.insert(tt, val_after)
            } else {
                error!(?tt, ?val, "Ignoring negative imbalance; this is a value preservation bug if occurring outside testing");
                treasury
            }
        });
        trace!("transaction phase successfully applied");
        Ok(state)
    }

    pub fn batch_apply_independant<P: Proofish<D>>(
        &self,
        txs: &[Transaction<P, D>],
        context: &TransactionContext<D>,
    ) -> Vec<TransactionResult<D>> {
        txs.iter().map(|tx| self.apply(tx, context).1).collect()
    }

    pub fn batch_apply_all_or_nothing<P: Proofish<D>>(
        &self,
        txs: &[Transaction<P, D>],
        context: &TransactionContext<D>,
    ) -> Result<(Self, Vec<TransactionResult<D>>), TransactionInvalid<D>> {
        let mut state = self.clone();
        let mut res = Vec::with_capacity(txs.len());
        for tx in txs {
            let (state2, txres) = state.apply(tx, context);
            if let TransactionResult::Failure(err) = txres {
                return Err(err);
            } else {
                res.push(txres);
            }
            state = state2;
        }
        Ok((state, res))
    }

    #[allow(clippy::type_complexity)]
    pub fn batch_apply_until_first_failure<'a, P: Proofish<D>>(
        &self,
        txs: &'a [Transaction<P, D>],
        context: &TransactionContext<D>,
    ) -> Result<
        (Self, Vec<TransactionResult<D>>),
        (
            // The state before the first failure
            Self,
            // The results up to the failure
            Vec<TransactionResult<D>>,
            // The failure itself
            TransactionInvalid<D>,
            // The remaining transactions, including the failing one
            &'a [Transaction<P, D>],
        ),
    > {
        let mut state = self.clone();
        let mut res = Vec::with_capacity(txs.len());
        for (i, tx) in txs.iter().enumerate() {
            let (state2, txres) = state.apply(tx, context);
            if let TransactionResult::Failure(err) = txres {
                return Err((state, res, err, &txs[i..]));
            } else {
                res.push(txres);
            }
            state = state2;
        }
        Ok((state, res))
    }

    #[instrument(skip(self, tx, context))]
    pub fn apply<P: Proofish<D>>(
        &self,
        tx: &Transaction<P, D>,
        context: &TransactionContext<D>,
    ) -> (Self, TransactionResult<D>) {
        match tx {
            Transaction::Standard(stx) => {
                let state = match self.apply_section(
                    tx,
                    Some(&stx.guaranteed_coins),
                    stx.contract_calls.as_ref(),
                    Some(stx.fallible_coins.as_ref()),
                    context,
                ) {
                    Ok(state) => state,
                    Err(err) => return (self.clone(), TransactionResult::Failure(err)),
                };
                let final_state = match state.apply_section(
                    tx,
                    stx.fallible_coins.as_ref(),
                    stx.contract_calls.as_ref(),
                    None,
                    context,
                ) {
                    Ok(state) => state,
                    Err(err) => return (state, TransactionResult::PartialSuccess(err)),
                };
                (final_state, TransactionResult::Success)
            }
            Transaction::ClaimMint(mint) => {
                let recipient_unclaimed = self
                    .unclaimed_mints
                    .get(&mint.mint.recipient)
                    .cloned()
                    .unwrap_or_else(Map::new);
                let claimable = recipient_unclaimed
                    .get(&mint.mint.coin.type_)
                    .copied()
                    .unwrap_or(0);
                if mint.mint.coin.value > claimable {
                    return (
                        self.clone(),
                        TransactionResult::Failure(TransactionInvalid::InsufficientClaimable {
                            requested: mint.mint.coin.value,
                            token_type: mint.mint.coin.type_,
                            claimable,
                            claimant: mint.mint.recipient,
                        }),
                    );
                }
                let remaining = claimable - mint.mint.coin.value;
                let new_recipient_unclaimed = if remaining == 0 {
                    recipient_unclaimed.remove(&mint.mint.coin.type_)
                } else {
                    recipient_unclaimed.insert(mint.mint.coin.type_, remaining)
                };
                let unclaimed_mints = if new_recipient_unclaimed.is_empty() {
                    self.unclaimed_mints.remove(&mint.mint.recipient)
                } else {
                    self.unclaimed_mints
                        .insert(mint.mint.recipient, new_recipient_unclaimed)
                };
                let zswap = match self
                    .zswap
                    .apply_mint(&mint.mint, context.whitelist.is_some())
                {
                    Ok((zswap, _, _)) => zswap,
                    Err(err) => {
                        return (
                            self.clone(),
                            TransactionResult::Failure(TransactionInvalid::Zswap(err)),
                        );
                    }
                };
                let treasury = self.treasury.insert(
                    NATIVE_TOKEN,
                    self.treasury
                        .get(&NATIVE_TOKEN)
                        .copied()
                        .unwrap_or(0)
                        .saturating_add(self.parameters.cost_model.mint_cost as u128),
                );
                (
                    LedgerState {
                        zswap,
                        unclaimed_mints,
                        treasury,
                        ..self.clone()
                    },
                    TransactionResult::Success,
                )
            }
        }
    }
}
