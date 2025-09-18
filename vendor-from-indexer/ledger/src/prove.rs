use crate::base_crypto::rng::SplittableRng;
use crate::error::TransactionProvingError;
use crate::serialize::Serializable;
use crate::structure::{
    ClaimMintTransaction, ContractAction, ContractCall, ContractCalls, Proof, ProofPreimage,
    ProofPreimageVersioned, ProofVersioned, ProvingData, StandardTransaction, Transaction,
};
use crate::transient_crypto::commitment::{Pedersen, PureGeneratorPedersen};
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::proofs::{
    IrSource, KeyLocation, ParamsProverProvider, ProverKey, VerifierKey,
};
use futures::future::join_all;
use onchain_runtime::cost_model::DUMMY_COST_MODEL;
use onchain_runtime::ops::Op;
use onchain_runtime::transcript::Transcript;
use rand::CryptoRng;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::io;
use std::pin::Pin;
use zswap::prove::ZswapResolver;
use zswap::storage::db::DB;
use zswap::transient_crypto::proofs::Resolver as ResolverT;

pub type ExternalResolver = Box<
    dyn Fn(
            KeyLocation,
        ) -> Pin<Box<dyn Future<Output = io::Result<Option<ProvingData>>> + Send + Sync>>
        + Send
        + Sync,
>;

pub struct Resolver {
    pub zswap_resolver: ZswapResolver,
    #[allow(clippy::type_complexity)]
    pub external_resolver: ExternalResolver,
}

impl Resolver {
    #[allow(clippy::type_complexity)]
    pub fn new(zswap_resolver: ZswapResolver, external_resolver: ExternalResolver) -> Self {
        Resolver {
            zswap_resolver,
            external_resolver,
        }
    }
    pub async fn resolve(&self, key: &KeyLocation) -> io::Result<Option<ProvingData>> {
        Ok(
            if let Some((pk, vk, ir)) = self.zswap_resolver.resolve_key(key.clone()).await? {
                Some(ProvingData::V4(pk, vk, ir))
            } else {
                (self.external_resolver)(key.clone()).await?
            },
        )
    }
}

impl crate::transient_crypto::proofs::ParamsProverProvider for Resolver {
    async fn get_params(&self, k: u8) -> io::Result<zswap::transient_crypto::proofs::ParamsProver> {
        self.zswap_resolver.get_params(k).await
    }
}

impl crate::transient_crypto::proofs::Resolver for Resolver {
    async fn resolve_key(
        &self,
        key: KeyLocation,
    ) -> io::Result<Option<(ProverKey, VerifierKey, IrSource)>> {
        match self.resolve(&key).await? {
            Some(ProvingData::V4(pk, vk, ir)) => Ok(Some((pk, vk, ir))),
            None => Ok(None),
        }
    }
}

impl<D: DB> Transaction<ProofPreimage, D> {
    #[instrument(skip(self, rng, pp, resolver))]
    pub async fn prove<'a>(
        &'a self,
        mut rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &Resolver,
    ) -> Result<Transaction<Proof, D>, TransactionProvingError<D>> {
        match self {
            Transaction::Standard(stx) => {
                let (guaranteed_coins, fallible_coins, contract_calls) = futures::join!(
                    stx.guaranteed_coins.prove(rng.split(), pp, resolver,),
                    futures::future::OptionFuture::from(
                        stx.fallible_coins
                            .as_ref()
                            .map(|o| { o.prove(rng.split(), pp, resolver) })
                    ),
                    futures::future::OptionFuture::from(
                        stx.contract_calls
                            .as_ref()
                            .map(|cc| cc.prove(rng.split(), pp, resolver))
                    )
                );
                Ok(Transaction::Standard(StandardTransaction {
                    guaranteed_coins: guaranteed_coins?,
                    fallible_coins: fallible_coins.transpose()?,
                    contract_calls: contract_calls.transpose()?,
                    binding_randomness: stx.binding_randomness,
                    raw_data: None,
                }))
            }
            Transaction::ClaimMint(mint) => Ok(Transaction::ClaimMint(ClaimMintTransaction {
                mint: mint.mint.prove(rng, pp, resolver).await?,
                raw_data: None,
            })),
        }
    }
}

impl<D: DB> ContractCalls<ProofPreimage, D> {
    async fn prove(
        &self,
        mut rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &Resolver,
    ) -> Result<ContractCalls<Proof, D>, TransactionProvingError<D>> {
        let calls = join_all(
            self.calls
                .iter()
                .map(|call| call.prove(rng.split(), pp, resolver, self.binding_commitment.into())),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
        let pedersen_randomness = self.binding_commitment;
        let binding_commitment = PureGeneratorPedersen::new_from(
            &mut rng,
            &pedersen_randomness,
            &ContractCalls::challenge_pre_for(&calls),
        );
        Ok(ContractCalls {
            calls,
            binding_commitment,
        })
    }
}

impl<D: DB> ContractAction<ProofPreimage, D> {
    async fn prove(
        &self,
        rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &Resolver,
        binding_commitment: Pedersen,
    ) -> Result<ContractAction<Proof, D>, TransactionProvingError<D>> {
        use ContractAction::*;
        Ok(match self {
            Call(call) => Call(Box::new(
                call.prove(rng, pp, resolver, binding_commitment).await?,
            )),
            Deploy(deploy) => Deploy(deploy.clone()),
            Maintain(upd) => Maintain(upd.clone()),
        })
    }
}

impl<D: DB> ContractCall<ProofPreimage, D> {
    async fn prove(
        &self,
        rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &Resolver,
        binding_commitment: Pedersen,
    ) -> Result<ContractCall<Proof, D>, TransactionProvingError<D>> {
        let cost_model = DUMMY_COST_MODEL; //TODO: replace with real cost model
        let active_calls = match &self.proof {
            ProofPreimageVersioned::V1(proof) => proof.check(
                &resolver
                    .resolve_key(proof.key_location.clone())
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| {
                        TransactionProvingError::MissingKeyset(proof.key_location.clone())
                    })?
                    .2,
            )?,
        };
        let mut remaining_active_calls = &active_calls[..];

        // Process the transcript programs, inserting noops in inactive segments
        let mut guaranteed_prog = Vec::new();
        let mut fallible_prog = Vec::new();
        for (old_transcript, transcript) in [
            self.guaranteed_transcript
                .as_ref()
                .map(|t| (t.program.iter(), &mut guaranteed_prog)),
            self.fallible_transcript
                .as_ref()
                .map(|t| (t.program.iter(), &mut fallible_prog)),
        ]
        .into_iter()
        .flatten()
        {
            for op in old_transcript {
                while let Some(Some(skip)) = remaining_active_calls.first() {
                    transcript.push(Op::Noop { n: *skip as u32 });
                    remaining_active_calls = &remaining_active_calls[1..];
                }
                transcript.push(op.clone());
                remaining_active_calls = &remaining_active_calls[1..];
            }
            while let Some(Some(skip)) = remaining_active_calls.first() {
                transcript.push(Op::Noop { n: *skip as u32 });
                remaining_active_calls = &remaining_active_calls[1..];
            }
        }
        // Combine adjacent noops, and count their cost.
        let mut guaranteed_noop_gas_cost: u64 = 0;
        let mut fallible_noop_gas_cost: u64 = 0;
        for (prog, gas_cost) in [
            (&mut guaranteed_prog, &mut guaranteed_noop_gas_cost),
            (&mut fallible_prog, &mut fallible_noop_gas_cost),
        ]
        .into_iter()
        {
            // Marks the current write head
            let mut i = 0;
            // The current chain of noops
            let mut n = 0;
            // Marks the current read head
            for j in 0..prog.len() {
                match prog[j].clone() {
                    Op::Noop { n: n2 } => n += n2,
                    op => {
                        if n != 0 {
                            prog[i] = Op::Noop { n };
                            *gas_cost +=
                                cost_model.noop_constant + cost_model.noop_linear * n as u64;
                            i += 1;
                            n = 0;
                        }
                        prog[i] = op;
                        i += 1;
                    }
                }
            }
            if n != 0 {
                prog[i] = Op::Noop { n };
                *gas_cost += cost_model.noop_constant + cost_model.noop_linear * n as u64;
                i += 1;
            }
            prog.truncate(i);
        }
        let guaranteed_transcript = self.guaranteed_transcript.as_ref().map(|t| Transcript {
            gas: t.gas + guaranteed_noop_gas_cost,
            effects: t.effects.clone(),
            program: guaranteed_prog,
            version: t.version.clone(),
        });
        let fallible_transcript = self.fallible_transcript.as_ref().map(|t| Transcript {
            gas: t.gas + fallible_noop_gas_cost,
            effects: t.effects.clone(),
            program: fallible_prog,
            version: t.version.clone(),
        });
        let mut hasher = Sha256::new();
        Serializable::serialize(
            &(
                &self.address,
                &self.entry_point,
                guaranteed_transcript.as_ref().map(|t| &t.gas).unwrap_or(&0),
                guaranteed_transcript
                    .as_ref()
                    .map(|t| &t.effects)
                    .unwrap_or(&Default::default()),
                fallible_transcript.as_ref().map(|t| &t.gas),
                fallible_transcript.as_ref().map(|t| &t.effects),
                guaranteed_transcript
                    .as_ref()
                    .map(|t| t.program.iter().len() as u64)
                    .unwrap_or(0),
                binding_commitment,
            ),
            &mut hasher,
        )
        .expect("In-memory serialization should succeed.");

        let proof = match &self.proof {
            ProofPreimageVersioned::V1(proof) => {
                let mut pre_proof = proof.clone();
                pre_proof.binding_input =
                    Fr::from_le_bytes(&hasher.finalize()[..31]).expect("31 bytes should fit in Fr");
                ProofVersioned::V1(pre_proof.prove(rng, pp, resolver).await?.0)
            }
        };

        // Assemble the final call
        Ok(ContractCall {
            address: self.address,
            entry_point: self.entry_point.clone(),
            guaranteed_transcript,
            fallible_transcript,
            communication_commitment: self.communication_commitment,
            proof,
            binding_input_data: None,
        })
    }
}
