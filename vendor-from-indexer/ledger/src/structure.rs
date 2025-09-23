use crate::base_crypto::hash::{HashOutput, persistent_commit};
use crate::base_crypto::repr::MemWrite;
use crate::base_crypto::signatures::Signature;
use crate::error::GuaranteedSectionLimitExceeded;
use crate::error::MalformedTransaction;
use crate::serialize::{self, Deserializable, Serializable, Version, Versioned};
use crate::storage::arena::ArenaKey;
use crate::storage::storable::Loader;
use crate::storage::storage::Map;
use crate::transient_crypto::commitment::{Pedersen, PedersenRandomness, PureGeneratorPedersen};
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::merkle_tree::MerkleTree;
use crate::transient_crypto::proofs::{
    IrSource, Proof as BaseProof, ProofPreimage as BaseProofPreimage, ProverKey, VerifierKey,
};
use crate::transient_crypto::repr::{FieldRepr, FromFieldRepr};
use crate::utils::{
    CapturingReader, deserialize_and_capture, serialize_or_data, serialize_or_data_mem,
};
use coin_structure::coin::{NATIVE_TOKEN, TokenType};
use coin_structure::contract::Address as ContractAddress;
use derive_where::derive_where;
#[cfg(any(test, feature = "fake"))]
use fake::Dummy;
use introspection_derive::Introspection;
use onchain_runtime::context::QueryContext;
use onchain_runtime::ops::Op;
use onchain_runtime::result_mode::ResultModeVerify;
#[cfg(feature = "verifying")]
use onchain_runtime::state::ContractOperation;
use onchain_runtime::state::{ContractMaintenanceAuthority, ContractState, EntryPointBuf};
use onchain_runtime::transcript::Transcript;
use rand::{CryptoRng, Rng};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug, Formatter};
use std::io::{Error, Read, Write};
#[cfg(feature = "serde")]
use std::marker::PhantomData;
#[cfg(feature = "verifying")]
use zswap::error::MalformedOffer;
use zswap::serialize::check_injected_version;
use zswap::storage::db::{DB, InMemoryDB};
use zswap::storage::{Storable, base_storable};
use zswap::{AuthorizedMint, Offer as ZswapOffer};

#[cfg(not(feature = "serde"))]
pub trait MaybeSerdeSerialize {}

#[cfg(not(feature = "serde"))]
impl<T> MaybeSerdeSerialize for T {}

#[cfg(feature = "serde")]
pub trait MaybeSerdeSerialize: Serialize {}

#[cfg(feature = "serde")]
impl<T: Serialize> MaybeSerdeSerialize for T {}

#[cfg(any(feature = "verifying", feature = "transaction-construction"))]
pub(crate) const MAX_ZSWAP_PART_BITS: usize = 16;

pub trait PedersenStage:
    Into<Pedersen> + Clone + PartialEq + Eq + PartialOrd + Ord + MaybeSerdeSerialize
{
    fn upgrade<R: Rng + CryptoRng>(
        &self,
        rng: &mut R,
        challenge_pre: &[u8],
    ) -> PureGeneratorPedersen;
    #[cfg(feature = "verifying")]
    fn valid(&self, _challenge_pre: &[u8]) -> Result<(), MalformedTransaction>;
}

impl PedersenStage for PureGeneratorPedersen {
    fn upgrade<R: Rng + CryptoRng>(
        &self,
        _rng: &mut R,
        _challenge_pre: &[u8],
    ) -> PureGeneratorPedersen {
        self.clone()
    }

    #[cfg(feature = "verifying")]
    fn valid(&self, challenge_pre: &[u8]) -> Result<(), MalformedTransaction> {
        if PureGeneratorPedersen::valid(self, challenge_pre) {
            Ok(())
        } else {
            Err(MalformedTransaction::InvalidSchnorrProof)
        }
    }
}

impl PedersenStage for PedersenRandomness {
    fn upgrade<R: Rng + CryptoRng>(
        &self,
        rng: &mut R,
        challenge_pre: &[u8],
    ) -> PureGeneratorPedersen {
        PureGeneratorPedersen::new_from(rng, self, challenge_pre)
    }

    #[cfg(feature = "verifying")]
    fn valid(&self, _challenge_pre: &[u8]) -> Result<(), MalformedTransaction> {
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Versioned)]
#[non_exhaustive]
pub enum ProofPreimageVersioned {
    V1(BaseProofPreimage),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ProofVersioned {
    V1(BaseProof),
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serializable, Deserializable, Versioned,
)]
pub struct Proof;

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serializable, Deserializable, Versioned,
)]
pub struct ProofPreimage;

pub trait Proofish<D: DB>: Clone + Ord + Serializable + Deserializable {
    type Pedersen: PedersenStage + Serializable + Deserializable;
    type Proof: Clone + PartialEq + Eq + Serializable + Deserializable;
    type LatestProof: Clone + PartialEq + Eq + Ord + Serializable + Deserializable;
    #[cfg(feature = "verifying")]
    fn zswap_well_formed(
        offer: &zswap::Offer<Self::LatestProof>,
        segment: u16,
    ) -> Result<Pedersen, MalformedOffer>;
    #[cfg(feature = "verifying")]
    fn zswap_mint_well_formed(
        mint: &zswap::AuthorizedMint<Self::LatestProof>,
    ) -> Result<(), MalformedOffer>;
    #[cfg(feature = "verifying")]
    fn proof_verify(
        op: &ContractOperation,
        proof: &Self::Proof,
        pis: Vec<Fr>,
        call: &ContractCall<Self, D>,
    ) -> Result<(), MalformedTransaction>;
}

impl<D: DB> Proofish<D> for Proof {
    type Pedersen = PureGeneratorPedersen;
    type Proof = ProofVersioned;
    type LatestProof = BaseProof;
    #[cfg(feature = "verifying")]
    fn zswap_well_formed(
        offer: &zswap::Offer<Self::LatestProof>,
        segment: u16,
    ) -> Result<Pedersen, MalformedOffer> {
        offer.well_formed(segment)
    }
    #[cfg(feature = "verifying")]
    fn zswap_mint_well_formed(
        mint: &zswap::AuthorizedMint<Self::LatestProof>,
    ) -> Result<(), MalformedOffer> {
        mint.well_formed()
    }
    #[cfg(all(not(feature = "proof-verifying"), feature = "verifying"))]
    fn proof_verify(
        _op: &ContractOperation,
        _proof: &Self::Proof,
        _pis: Vec<Fr>,
        _call: &ContractCall<Self, D>,
    ) -> Result<(), MalformedTransaction> {
        Ok(())
    }
    #[cfg(all(feature = "proof-verifying", feature = "verifying"))]
    fn proof_verify(
        op: &ContractOperation,
        proof: &Self::Proof,
        pis: Vec<Fr>,
        call: &ContractCall<Self, D>,
    ) -> Result<(), MalformedTransaction> {
        use zswap::transient_crypto::proofs::PARAMS_VERIFIER;

        let vk = match &op.v2 {
            Some(vk) => vk,
            None => {
                warn!("missing verifier key");
                return Err(MalformedTransaction::VerifierKeyNotPresent {
                    address: call.address,
                    operation: call.entry_point.clone(),
                });
            }
        };

        if op.v2.is_some() && !matches!(proof, ProofVersioned::V1(_)) {
            return Err(MalformedTransaction::UnsupportedProofVersion {
                op_version: "V2".to_string(),
            });
        }

        match proof {
            ProofVersioned::V1(proof) => vk
                .verify(&PARAMS_VERIFIER, proof, pis.into_iter())
                .map_err(MalformedTransaction::InvalidProof),
        }
    }
}

impl<D: DB> Proofish<D> for ProofPreimage {
    type Pedersen = PedersenRandomness;
    type Proof = ProofPreimageVersioned;
    type LatestProof = BaseProofPreimage;
    #[cfg(feature = "verifying")]
    fn zswap_well_formed(
        offer: &zswap::Offer<Self::LatestProof>,
        segment: u16,
    ) -> Result<Pedersen, MalformedOffer> {
        offer.well_formed(segment)
    }
    #[cfg(feature = "verifying")]
    fn zswap_mint_well_formed(
        _: &zswap::AuthorizedMint<Self::LatestProof>,
    ) -> Result<(), MalformedOffer> {
        Ok(())
    }
    #[cfg(feature = "verifying")]
    fn proof_verify(
        _: &ContractOperation,
        _: &Self::Proof,
        _: Vec<Fr>,
        _: &ContractCall<Self, D>,
    ) -> Result<(), MalformedTransaction> {
        Ok(())
    }
}

impl<D: DB> Proofish<D> for () {
    type Pedersen = PureGeneratorPedersen;
    type Proof = ();
    type LatestProof = ();
    #[cfg(feature = "verifying")]
    fn zswap_well_formed(
        offer: &zswap::Offer<Self::LatestProof>,
        segment: u16,
    ) -> Result<Pedersen, MalformedOffer> {
        offer.well_formed(segment)
    }
    #[cfg(feature = "verifying")]
    fn zswap_mint_well_formed(
        _: &zswap::AuthorizedMint<Self::LatestProof>,
    ) -> Result<(), MalformedOffer> {
        Ok(())
    }
    #[cfg(feature = "verifying")]
    fn proof_verify(
        _: &ContractOperation,
        _: &Self::Proof,
        _: Vec<Fr>,
        _: &ContractCall<Self, D>,
    ) -> Result<(), MalformedTransaction> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serializable, Deserializable, Versioned)]
pub struct OutputInstruction {
    pub amount: u128,
    pub target_key: coin_structure::coin::PublicKey,
}

#[derive(Clone, Debug, PartialEq, Serializable)]
#[non_exhaustive]
// TODO: Getting `Box` to serialize is a pain right now. Revisit later.
#[allow(clippy::large_enum_variant)]
pub enum SystemTransaction {
    OverwriteParameters(LedgerParameters),
    Mint(Vec<OutputInstruction>),
    MintToTreasury {
        amount: u128,
    },
    PayFromTreasury {
        outputs: Vec<OutputInstruction>,
        nonce: HashOutput,
        token_type: TokenType,
    },
}

impl Versioned for SystemTransaction {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
}

impl Deserializable for SystemTransaction {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 1, minor: 0 }) => {
                let variant: u8 = Deserializable::deserialize(reader, recursion_depth)?;
                Ok(match variant {
                    0 => SystemTransaction::OverwriteParameters(Deserializable::deserialize(
                        reader,
                        recursion_depth,
                    )?),
                    1 => SystemTransaction::Mint(Deserializable::deserialize(
                        reader,
                        recursion_depth,
                    )?),
                    2 => SystemTransaction::MintToTreasury {
                        amount: Deserializable::deserialize(reader, recursion_depth)?,
                    },
                    3 => SystemTransaction::PayFromTreasury {
                        outputs: Deserializable::deserialize(reader, recursion_depth)?,
                        nonce: Deserializable::deserialize(reader, recursion_depth)?,
                        token_type: Deserializable::deserialize(reader, recursion_depth)?,
                    },
                    _ => {
                        return Err(Self::deserialization_error(
                            version,
                            format!("unknown variant: {variant}"),
                        ));
                    }
                })
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serializable)]
pub struct TransactionCostModel {
    pub runtime_cost_model: onchain_runtime::cost_model::CostModel,
    pub transaction_cost_per_byte: u64,
    pub verify_cost_constant: u64,
    pub verify_cost_linear: u64,
    pub deploy_cost_per_byte: u64,
    pub mint_cost: u64,
}

pub const DUMMY_TRANSACTION_COST_MODEL: TransactionCostModel = TransactionCostModel {
    runtime_cost_model: onchain_runtime::cost_model::DUMMY_COST_MODEL,
    transaction_cost_per_byte: 1,
    verify_cost_constant: 5_000,
    verify_cost_linear: 50,
    deploy_cost_per_byte: 100,
    mint_cost: 10_000,
};

impl Versioned for TransactionCostModel {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 1 });
}

impl Deserializable for TransactionCostModel {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 1, minor: 1 }) => Ok(TransactionCostModel {
                runtime_cost_model: Deserializable::deserialize(reader, recursion_depth)?,
                transaction_cost_per_byte: Deserializable::deserialize(reader, recursion_depth)?,
                verify_cost_constant: Deserializable::deserialize(reader, recursion_depth)?,
                verify_cost_linear: Deserializable::deserialize(reader, recursion_depth)?,
                deploy_cost_per_byte: Deserializable::deserialize(reader, recursion_depth)?,
                mint_cost: Deserializable::deserialize(reader, recursion_depth)?,
            }),
            Some(Version { major: 1, minor: 0 }) => Ok(TransactionCostModel {
                runtime_cost_model: Deserializable::deserialize(reader, recursion_depth)?,
                transaction_cost_per_byte: Deserializable::deserialize(reader, recursion_depth)?,
                verify_cost_constant: DUMMY_TRANSACTION_COST_MODEL.verify_cost_constant,
                verify_cost_linear: DUMMY_TRANSACTION_COST_MODEL.verify_cost_linear,
                deploy_cost_per_byte: Deserializable::deserialize(reader, recursion_depth)?,
                mint_cost: Deserializable::deserialize(reader, recursion_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serializable)]
pub struct TransactionLimits {
    pub transaction_byte_limit: u64,
    pub guaranteed_cost_limit_per_byte: u64,
}

impl Versioned for TransactionLimits {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
}

impl Deserializable for TransactionLimits {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 1, minor: 0 }) => Ok(TransactionLimits {
                transaction_byte_limit: Deserializable::deserialize(reader, recursion_depth)?,
                guaranteed_cost_limit_per_byte: Deserializable::deserialize(
                    reader,
                    recursion_depth,
                )?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

pub const DUMMY_LIMITS: TransactionLimits = TransactionLimits {
    transaction_byte_limit: 1 << 20, // 1 MiB
    guaranteed_cost_limit_per_byte: 25,
};

#[derive(Clone, Debug, PartialEq, Eq, Serializable)]
pub struct LedgerParameters {
    pub cost_model: TransactionCostModel,
    pub limits: TransactionLimits,
    pub ticket_cost_multiplier: u64,
    pub ticket_cost_divisor: u64,
}

base_storable!(LedgerParameters);

pub const DUMMY_PARAMETERS: LedgerParameters = LedgerParameters {
    cost_model: DUMMY_TRANSACTION_COST_MODEL,
    limits: DUMMY_LIMITS,
    ticket_cost_multiplier: 1_000,
    ticket_cost_divisor: 1_000,
};

impl Versioned for LedgerParameters {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 2 });
}

impl Deserializable for LedgerParameters {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 1, minor: 2 }) => Ok(LedgerParameters {
                cost_model: Deserializable::deserialize(reader, recursion_depth)?,
                limits: Deserializable::deserialize(reader, recursion_depth)?,
                ticket_cost_multiplier: Deserializable::deserialize(reader, recursion_depth)?,
                ticket_cost_divisor: Deserializable::deserialize(reader, recursion_depth)?,
            }),
            Some(Version { major: 1, minor: 1 }) => Ok(LedgerParameters {
                cost_model: Deserializable::deserialize(reader, recursion_depth)?,
                limits: TransactionLimits {
                    transaction_byte_limit: Deserializable::deserialize(reader, recursion_depth)?,
                    ..DUMMY_LIMITS
                },
                ticket_cost_multiplier: Deserializable::deserialize(reader, recursion_depth)?,
                ticket_cost_divisor: Deserializable::deserialize(reader, recursion_depth)?,
            }),
            Some(Version { major: 1, minor: 0 }) => {
                let cost_model: TransactionCostModel =
                    Deserializable::deserialize(reader, recursion_depth)?;
                let transaction_byte_limit: u64 =
                    Deserializable::deserialize(reader, recursion_depth)?;
                let _transaction_proof_no_limit: u16 =
                    Deserializable::deserialize(reader, recursion_depth)?;
                let _transaction_vk_no_limit: u16 =
                    Deserializable::deserialize(reader, recursion_depth)?;
                let ticket_cost_multiplier: u64 =
                    Deserializable::deserialize(reader, recursion_depth)?;
                let ticket_cost_divisor: u64 =
                    Deserializable::deserialize(reader, recursion_depth)?;
                Ok(LedgerParameters {
                    cost_model,
                    limits: TransactionLimits {
                        transaction_byte_limit,
                        ..DUMMY_LIMITS
                    },
                    ticket_cost_multiplier,
                    ticket_cost_divisor,
                })
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

impl Serializable for ProofPreimageVersioned {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        use ProofPreimageVersioned as V;
        match value {
            V::V1(proof) => {
                Serializable::serialize(&0u8, writer)?;
                Serializable::serialize(proof, writer)
            }
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        use ProofPreimageVersioned as V;
        match value {
            V::V1(proof) => 1 + Serializable::serialized_size(proof),
        }
    }
}

impl Deserializable for ProofPreimageVersioned {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        use ProofPreimageVersioned as V;
        let mut disc = vec![0u8; 1];
        reader.read_exact(&mut disc)?;
        match disc[0] {
            0 => Ok(V::V1(Deserializable::deserialize(reader, recursion_depth)?)),
            _ => Err(Self::deserialization_error(
                version,
                format!("Unknown discriminant {}", disc[0]),
            )),
        }
    }
}

impl Serializable for ProofVersioned {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        use ProofVersioned as V;
        match value {
            V::V1(proof) => {
                Serializable::serialize(&0u8, writer)?;
                Serializable::serialize(proof, writer)
            }
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        use ProofVersioned as V;
        match value {
            V::V1(proof) => 1 + Serializable::serialized_size(proof),
        }
    }
}

impl Versioned for ProofVersioned {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 1 });
}

impl Deserializable for ProofVersioned {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        use ProofVersioned as V;
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(V::V1(BaseProof::versioned_deserialize(
                reader,
                version,
                recursion_depth,
            )?)),
            Some(Version { major: 2, minor: 1 }) => {
                let discriminant: u8 = Deserializable::deserialize(reader, recursion_depth)?;
                match discriminant {
                    0 => Ok(V::V1(Deserializable::deserialize(reader, recursion_depth)?)),
                    _ => Err(Self::deserialization_error(
                        version,
                        format!("Unknown ProofVersioned discriminant: {discriminant}"),
                    )),
                }
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[derive(Clone, Debug, Introspection, Versioned)]
pub enum ProvingData {
    V4(ProverKey, VerifierKey, IrSource),
}

impl Serializable for ProvingData {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        use ProvingData as P;
        match value {
            P::V4(pk, vk, ir) => Serializable::serialize(&(pk, vk, ir), writer),
        }
    }
    fn unversioned_serialized_size(value: &Self) -> usize {
        use ProvingData as P;
        match value {
            P::V4(pk, vk, ir) => Serializable::serialized_size(&(pk, vk, ir)),
        }
    }
}

// General serialization approach:
// ProvingData is *not* versioned
// We read the first two bytes to determine version of ProverKey
// Then call `versioned_deserialize` of the right `ProverKey` instance
impl Deserializable for ProvingData {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, Error> {
        let major: u8 = Deserializable::deserialize(reader, recursion_depth)?;
        let minor = Deserializable::deserialize(reader, recursion_depth)?;
        let prover_key_version = Version { major, minor };

        match prover_key_version {
            Version { major: 4, minor: 0 } => {
                let pk = ProverKey::versioned_deserialize(
                    reader,
                    Some(&prover_key_version),
                    recursion_depth,
                )?;
                let vk = VerifierKey::deserialize(reader, recursion_depth)?;
                let ir = <IrSource as Deserializable>::deserialize(reader, recursion_depth)?;
                Ok(ProvingData::V4(pk, vk, ir))
            }
            _ => Err(ProverKey::deserialization_error(
                Some(&prover_key_version),
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
// TODO: Getting `Box` to serialize is a pain right now. Revisit later.
#[allow(clippy::large_enum_variant)]
pub enum Transaction<P: Proofish<D> + Serializable + Deserializable, D: DB> {
    Standard(StandardTransaction<P, D>),
    ClaimMint(ClaimMintTransaction<P, D>),
}

impl<P: Proofish<D> + Serializable + Deserializable + Debug, D: DB> Debug for Transaction<P, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Transaction::Standard(stx) => stx.fmt(f),
            Transaction::ClaimMint(mtx) => mtx.fmt(f),
        }
    }
}

impl<P: Proofish<D>, D: DB> Versioned for Transaction<P, D> {
    const VERSION: Option<Version> = Some(Version { major: 4, minor: 0 });
}

impl<P: Proofish<D> + Serializable + Deserializable, D: DB> Serializable for Transaction<P, D> {
    fn serialize<W: std::io::Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        let Version { major, minor } = Self::VERSION.expect("Transactions are versioned");
        match value {
            Transaction::Standard(stx) => serialize_or_data(
                &(
                    major,
                    minor,
                    0u32,
                    &stx.guaranteed_coins,
                    &stx.fallible_coins,
                    &stx.contract_calls,
                    &stx.binding_randomness,
                ),
                stx.raw_data.as_ref(),
                writer,
            ),
            Transaction::ClaimMint(mtx) => serialize_or_data(
                &(major, minor, 1u32, &mtx.mint),
                mtx.raw_data.as_ref(),
                writer,
            ),
        }
    }
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        match value {
            Transaction::Standard(stx) => serialize_or_data(
                &(
                    0u32,
                    &stx.guaranteed_coins,
                    &stx.fallible_coins,
                    &stx.contract_calls,
                    &stx.binding_randomness,
                ),
                stx.raw_data.as_ref().map(|slice| &slice[2..]),
                writer,
            ),
            Transaction::ClaimMint(mtx) => serialize_or_data(
                &(1u32, &mtx.mint),
                mtx.raw_data.as_ref().map(|slice| &slice[2..]),
                writer,
            ),
        }
    }
    fn unversioned_serialized_size(value: &Self) -> usize {
        match value {
            Transaction::Standard(stx) => stx
                .raw_data
                .as_ref()
                .map(|data| data.len())
                .unwrap_or_else(|| {
                    1 + Serializable::serialized_size(&stx.guaranteed_coins)
                        + Serializable::serialized_size(&stx.fallible_coins)
                        + Serializable::serialized_size(&stx.contract_calls)
                        + Serializable::serialized_size(&stx.binding_randomness)
                }),
            Transaction::ClaimMint(mtx) => mtx
                .raw_data
                .as_ref()
                .map(|data| data.len())
                .unwrap_or_else(|| 1 + Serializable::serialized_size(&mtx.mint)),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct StandardTransaction<P: Proofish<D> + Serializable + Deserializable, D: DB> {
    pub guaranteed_coins: ZswapOffer<P::LatestProof>,
    pub fallible_coins: Option<ZswapOffer<P::LatestProof>>,
    pub contract_calls: Option<ContractCalls<P, D>>,
    pub binding_randomness: PedersenRandomness,
    pub(crate) raw_data: Option<Vec<u8>>,
}

impl<P: Proofish<D> + Serializable + Deserializable + Debug, D: DB> Debug
    for StandardTransaction<P, D>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("StandardTransaction")
            .field("guaranteed_coins", &self.guaranteed_coins)
            .field("fallible_coins", &self.fallible_coins)
            .field("contract_calls", &self.contract_calls)
            .field("binding_randomness", &self.binding_randomness)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ClaimMintTransaction<P: Proofish<D> + Serializable + Deserializable, D: DB> {
    pub mint: zswap::AuthorizedMint<P::LatestProof>,
    pub(crate) raw_data: Option<Vec<u8>>,
}

impl<P: Proofish<D> + Serializable + Deserializable + Debug, D: DB> Debug
    for ClaimMintTransaction<P, D>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ClaimMintTransaction")
            .field(&self.mint)
            .finish()
    }
}

impl<P: Proofish<D> + Serializable + Deserializable, D: DB>
    From<zswap::AuthorizedMint<P::LatestProof>> for ClaimMintTransaction<P, D>
{
    fn from(value: zswap::AuthorizedMint<P::LatestProof>) -> Self {
        ClaimMintTransaction {
            mint: value,
            raw_data: None,
        }
    }
}

impl<P: Proofish<D>, D: DB> Deserializable for Transaction<P, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let mut reader = CapturingReader::with_data(
            reader,
            version.map(|v| vec![v.major, v.minor]).unwrap_or_default(),
        );
        match version {
            Some(Version { major: 4, minor: 0 }) => {
                let discriminant: u32 = Deserializable::deserialize(&mut reader, recursion_depth)?;
                match discriminant {
                    0 => {
                        let guaranteed_coins =
                            Deserializable::deserialize(&mut reader, recursion_depth)?;
                        let fallible_coins =
                            Deserializable::deserialize(&mut reader, recursion_depth)?;
                        let contract_calls =
                            Deserializable::deserialize(&mut reader, recursion_depth)?;
                        let binding_randomness =
                            Deserializable::deserialize(&mut reader, recursion_depth)?;
                        Ok(Transaction::Standard(StandardTransaction {
                            guaranteed_coins,
                            fallible_coins,
                            contract_calls,
                            binding_randomness,
                            raw_data: Some(reader.into_inner().1),
                        }))
                    }
                    1 => {
                        let mint = Deserializable::deserialize(&mut reader, recursion_depth)?;
                        Ok(Transaction::ClaimMint(ClaimMintTransaction {
                            mint,
                            raw_data: Some(reader.into_inner().1),
                        }))
                    }
                    _ => Err(Self::deserialization_error(
                        version,
                        format!("Unknown Transaction discriminant: {discriminant}"),
                    )),
                }
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

impl<P: Proofish<D>, D: DB> Transaction<P, D> {
    pub fn erase_proofs<R: Rng + CryptoRng>(&self, rng: &mut R) -> Transaction<(), D> {
        match self {
            Transaction::Standard(StandardTransaction {
                guaranteed_coins,
                fallible_coins,
                contract_calls,
                binding_randomness,
                ..
            }) => Transaction::Standard(StandardTransaction {
                guaranteed_coins: guaranteed_coins.erase_proofs(),
                fallible_coins: fallible_coins.as_ref().map(ZswapOffer::erase_proofs),
                contract_calls: contract_calls.as_ref().map(|calls| calls.erase_proofs(rng)),
                binding_randomness: *binding_randomness,
                raw_data: None,
            }),
            Transaction::ClaimMint(ClaimMintTransaction { mint, .. }) => {
                Transaction::ClaimMint(ClaimMintTransaction {
                    mint: mint.erase_proof(),
                    raw_data: None,
                })
            }
        }
    }

    #[instrument(skip(self, other))]
    pub fn merge(&self, other: &Self) -> Result<Self, MalformedTransaction> {
        use Transaction as T;
        match (self, other) {
            (T::Standard(stx1), T::Standard(stx2)) => {
                if stx1.contract_calls.is_some() && stx2.contract_calls.is_some() {
                    return Err(MalformedTransaction::MergingContracts);
                }
                let res = Transaction::Standard(StandardTransaction {
                    guaranteed_coins: stx1.guaranteed_coins.merge(&stx2.guaranteed_coins)?,
                    fallible_coins: match (&stx1.fallible_coins, &stx2.fallible_coins) {
                        (Some(a), Some(b)) => Some(a.merge(b)?),
                        (a, b) => a.clone().or(b.clone()),
                    },
                    contract_calls: stx1.contract_calls.clone().or(stx2.contract_calls.clone()),
                    binding_randomness: stx1.binding_randomness + stx2.binding_randomness,
                    raw_data: None,
                });
                debug!("transaction merged");
                Ok(res)
            }
            _ => Err(MalformedTransaction::CantMergeTypes),
        }
    }

    pub fn identifiers(&'_ self) -> impl Iterator<Item = TransactionIdentifier> + '_ {
        let mut res = Vec::new();
        match self {
            Transaction::Standard(stx) => res.extend(
                stx.guaranteed_coins
                    .inputs
                    .iter()
                    .chain(stx.fallible_coins.iter().flat_map(|o| o.inputs.iter()))
                    .map(|i| i.value_commitment)
                    .chain(
                        stx.guaranteed_coins
                            .outputs
                            .iter()
                            .chain(stx.fallible_coins.iter().flat_map(|o| o.outputs.iter()))
                            .map(|o| o.value_commitment),
                    )
                    .chain(
                        stx.guaranteed_coins
                            .transient
                            .iter()
                            .chain(stx.fallible_coins.iter().flat_map(|o| o.transient.iter()))
                            .map(|io| io.value_commitment_input),
                    )
                    .chain(
                        stx.guaranteed_coins
                            .transient
                            .iter()
                            .chain(stx.fallible_coins.iter().flat_map(|o| o.transient.iter()))
                            .map(|io| io.value_commitment_output),
                    )
                    .chain(
                        stx.contract_calls
                            .iter()
                            .map(|cs| cs.binding_commitment.clone().into()),
                    )
                    .map(TransactionIdentifier::Merged),
            ),
            Transaction::ClaimMint(mint) => {
                res.push(TransactionIdentifier::Unique(persistent_commit(
                    &(mint.mint.coin, mint.mint.recipient),
                    HashOutput(*b"midnight:mint_tx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"),
                )));
            }
        }
        res.into_iter()
    }

    pub fn deploys(&'_ self) -> impl Iterator<Item = &'_ ContractDeploy<D>> + '_ {
        match self {
            Transaction::Standard(stx) => stx.contract_calls.as_ref(),
            _ => None,
        }
        .into_iter()
        .flat_map(|cs| cs.calls.iter())
        .filter_map(|cd| match cd {
            ContractAction::Deploy(ref d) => Some(d),
            _ => None,
        })
    }

    pub fn updates(&'_ self) -> impl Iterator<Item = &'_ MaintenanceUpdate> + '_ {
        match self {
            Transaction::Standard(stx) => stx.contract_calls.as_ref(),
            _ => None,
        }
        .into_iter()
        .flat_map(|cs| cs.calls.iter())
        .filter_map(|cd| match cd {
            ContractAction::Maintain(ref upd) => Some(upd),
            _ => None,
        })
    }

    pub fn calls(&'_ self) -> impl Iterator<Item = &'_ ContractCall<P, D>> + '_ {
        match self {
            Transaction::Standard(stx) => stx.contract_calls.as_ref(),
            _ => None,
        }
        .into_iter()
        .flat_map(|cs| cs.calls())
    }

    pub fn has_identifier(&self, ident: &TransactionIdentifier) -> bool {
        self.identifiers().any(|ident2| ident == &ident2)
    }

    pub fn imbalances(&self, guaranteed: bool, fees: Option<u128>) -> Vec<(TokenType, i128)> {
        match self {
            Transaction::Standard(stx) => {
                let mints =
                    stx.contract_calls
                        .iter()
                        .flat_map(|cs| {
                            cs.calls.iter().filter_map(|cd| match cd {
                                ContractAction::Call(call) => if guaranteed {
                                    call.guaranteed_transcript.as_ref()
                                } else {
                                    call.fallible_transcript.as_ref()
                                }
                                .map(|transcript| {
                                    transcript.effects.mints.iter().map(|(sep, val)| {
                                        (call.address.custom_token_type(*sep), *val)
                                    })
                                }),
                                _ => None,
                            })
                        })
                        .flatten();
                let deltas = if guaranteed {
                    stx.guaranteed_coins.deltas.iter()
                } else {
                    stx.fallible_coins
                        .as_ref()
                        .map(|c| c.deltas.iter())
                        .unwrap_or_else(|| [].iter())
                };
                fn saturating_cast_u128_i128(x: u128) -> i128 {
                    if x > i128::MAX as u128 {
                        i128::MAX
                    } else {
                        x as i128
                    }
                }
                zswap::normalize_deltas(
                    deltas
                        .cloned()
                        .chain([(
                            NATIVE_TOKEN,
                            -(saturating_cast_u128_i128(fees.unwrap_or(0))),
                        )])
                        .chain(mints.map(|(ty, val)| (ty, val as i128))),
                )
            }
            Transaction::ClaimMint { .. } => Vec::new(),
        }
    }
}

pub(crate) trait ProofFees {
    fn proof_fees(&self, cost_model: &TransactionCostModel) -> u128;
    fn proof_sizes(&self) -> (usize, usize);
}

impl<P: Serializable> ProofFees for zswap::Offer<P> {
    fn proof_fees(&self, cost_model: &TransactionCostModel) -> u128 {
        let inputs = (self.inputs.len() + self.transient.len()) as u128;
        let outputs = (self.outputs.len() + self.transient.len()) as u128;
        let constant_factors = inputs + outputs;
        let linear_factors = inputs
            .saturating_mul(zswap::INPUT_PIS as u128)
            .saturating_add(outputs.saturating_mul(zswap::OUTPUT_PIS as u128));
        constant_factors
            .saturating_mul(cost_model.verify_cost_constant as u128)
            .saturating_add(linear_factors.saturating_mul(cost_model.verify_cost_linear as u128))
    }

    fn proof_sizes(&self) -> (usize, usize) {
        let count = self.inputs.len() + self.outputs.len() + self.transient.len();
        let size = self
            .inputs
            .iter()
            .map(|i| &i.proof)
            .chain(self.outputs.iter().map(|o| &o.proof))
            .chain(
                self.transient
                    .iter()
                    .flat_map(|t| [&t.proof_input, &t.proof_output].into_iter()),
            )
            .map(|p| Serializable::serialized_size(p))
            .sum::<usize>();
        (count, size)
    }
}

impl<P: Proofish<D>, D: DB> ProofFees for Transaction<P, D> {
    fn proof_fees(&self, cost_model: &TransactionCostModel) -> u128 {
        match self {
            Transaction::Standard(stx) => stx
                .guaranteed_coins
                .proof_fees(cost_model)
                .saturating_add(
                    stx.fallible_coins
                        .as_ref()
                        .map(|c| ProofFees::proof_fees(c, cost_model))
                        .unwrap_or(0),
                )
                .saturating_add(
                    stx.contract_calls
                        .as_ref()
                        .map(|c| ProofFees::proof_fees(c, cost_model))
                        .unwrap_or(0),
                ),
            Transaction::ClaimMint(mint) => mint.mint.proof_fees(cost_model),
        }
    }

    fn proof_sizes(&self) -> (usize, usize) {
        match self {
            Transaction::Standard(stx) => {
                let guaranteed = stx.guaranteed_coins.proof_sizes();
                let fallible = stx
                    .fallible_coins
                    .as_ref()
                    .map(|c| c.proof_sizes())
                    .unwrap_or((0, 0));
                let calls = stx
                    .contract_calls
                    .as_ref()
                    .map(|c| c.proof_sizes())
                    .unwrap_or((0, 0));
                (
                    guaranteed.0 + fallible.0 + calls.0,
                    guaranteed.1 + fallible.1 + calls.1,
                )
            }
            Transaction::ClaimMint(mint) => mint.mint.proof_sizes(),
        }
    }
}

impl<P: Serializable> ProofFees for AuthorizedMint<P> {
    fn proof_fees(&self, cost_model: &TransactionCostModel) -> u128 {
        (cost_model.verify_cost_constant as u128).saturating_add(
            (cost_model.verify_cost_linear as u128)
                .saturating_mul(zswap::AUTHORIZED_MINT_PIS as u128),
        )
    }
    fn proof_sizes(&self) -> (usize, usize) {
        (1, Serializable::serialized_size(&self.proof))
    }
}

impl<P: Proofish<D>, D: DB> ProofFees for ContractCalls<P, D> {
    fn proof_fees(&self, cost_model: &TransactionCostModel) -> u128 {
        self.calls
            .iter()
            .filter_map(|c| match c {
                ContractAction::Call(c) => Some(c),
                _ => None,
            })
            .map(|c| c.proof_fees(cost_model))
            .fold(0, u128::saturating_add)
    }
    fn proof_sizes(&self) -> (usize, usize) {
        self.calls
            .iter()
            .filter_map(|c| match c {
                ContractAction::Call(c) => Some(c),
                _ => None,
            })
            .map(|c| c.proof_sizes())
            .fold((0, 0), |(a, b), (c, d)| (a + c, b + d))
    }
}

impl<P: Proofish<D>, D: DB> ProofFees for ContractCall<P, D> {
    fn proof_fees(&self, cost_model: &TransactionCostModel) -> u128 {
        let pis = 2 + self
            .guaranteed_transcript
            .iter()
            .chain(self.fallible_transcript.iter())
            .flat_map(|t| t.program.iter())
            .map(|op| op.field_size())
            .sum::<usize>();
        (cost_model.verify_cost_constant as u128)
            .saturating_add((cost_model.verify_cost_linear as u128).saturating_mul(pis as u128))
    }
    fn proof_sizes(&self) -> (usize, usize) {
        (1, Serializable::serialized_size(&self.proof))
    }
}

pub(crate) const PROOF_SIZE: usize = 11_318;

impl<P: Proofish<D>, D: DB> Transaction<P, D>
where
    Transaction<P, D>: Serializable,
{
    pub fn fees(&self, params: &LedgerParameters) -> Result<u128, GuaranteedSectionLimitExceeded> {
        let pre_adjustment = self.cost(params)?;
        let adjusted = pre_adjustment
            .saturating_mul(params.ticket_cost_multiplier as u128)
            .saturating_div(params.ticket_cost_divisor as u128);
        Ok(adjusted)
    }

    pub fn cost(&self, params: &LedgerParameters) -> Result<u128, GuaranteedSectionLimitExceeded> {
        Ok(match self {
            Transaction::Standard { .. } => {
                let proof_sizes = self.proof_sizes();
                let orig_size = Transaction::<P, D>::serialized_size(self);
                let size = orig_size
                    .saturating_sub(proof_sizes.1)
                    .saturating_add(proof_sizes.0.saturating_mul(PROOF_SIZE))
                    as u128;
                let guaranteed = self.guaranteed_fees(&params.cost_model, size);
                let fallible = self.fallible_fees(&params.cost_model);
                let guaranteed_cost_limit_per_byte =
                    params.limits.guaranteed_cost_limit_per_byte as u128;
                if guaranteed > size.saturating_mul(guaranteed_cost_limit_per_byte) {
                    return Err(GuaranteedSectionLimitExceeded {
                        real_cost: guaranteed,
                        without_size_cost: guaranteed.saturating_sub(
                            size.saturating_mul(
                                params.cost_model.transaction_cost_per_byte as u128,
                            ),
                        ),
                        bound: size * guaranteed_cost_limit_per_byte,
                    });
                }
                guaranteed + fallible
            }
            Transaction::ClaimMint { .. } => params.cost_model.mint_cost as u128,
        })
    }

    fn guaranteed_fees(&self, cost_model: &TransactionCostModel, size: u128) -> u128 {
        self.transcript_selector_fees(cost_model, |t| t.guaranteed_transcript.as_ref(), false)
            .saturating_add(size.saturating_mul(cost_model.transaction_cost_per_byte as u128))
            .saturating_add(self.proof_fees(cost_model))
    }

    fn fallible_fees(&self, cost_model: &TransactionCostModel) -> u128 {
        self.transcript_selector_fees(cost_model, |t| t.fallible_transcript.as_ref(), true)
    }

    fn transcript_selector_fees<F: Fn(&ContractCall<P, D>) -> Option<&Transcript<D>>>(
        &self,
        cost_model: &TransactionCostModel,
        selector: F,
        count_deploy: bool,
    ) -> u128 {
        match self {
            Transaction::Standard(stx) => stx.contract_calls.as_ref(),
            _ => None,
        }
        .map(|calls| {
            calls
                .calls
                .iter()
                .map(|cd| match (cd, count_deploy) {
                    // TODO: Add constant overhead parts to calls?
                    (ContractAction::Call(call), _) => {
                        selector(call).map(|t| t.gas as u128).unwrap_or(0)
                    }
                    (ContractAction::Deploy(deploy), true) => (cost_model.deploy_cost_per_byte
                        as u128)
                        .saturating_mul(ContractDeploy::serialized_size(deploy) as u128),
                    (ContractAction::Maintain(upd), true) => (cost_model.deploy_cost_per_byte
                        as u128)
                        .saturating_mul(MaintenanceUpdate::serialized_size(upd) as u128),
                    (_, false) => 0,
                })
                .fold(0, u128::saturating_add)
        })
        .unwrap_or(0)
    }
}

impl<D: DB> Transaction<Proof, D> {
    pub fn transaction_hash(&self) -> TransactionHash {
        let mut hasher = Sha256::new();
        match self {
            Transaction::Standard(stx) => {
                serialize_or_data_mem(self, stx.raw_data.as_ref(), &mut hasher);
            }
            Transaction::ClaimMint(mtx) => {
                serialize_or_data_mem(self, mtx.raw_data.as_ref(), &mut hasher);
            }
        }
        TransactionHash(HashOutput(hasher.finalize().into()))
    }
}

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    Versioned,
    Serializable,
    Deserializable,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
pub struct TransactionHash(pub HashOutput);

#[cfg(feature = "serde")]
impl Serialize for TransactionHash {
    fn serialize<S: serde::ser::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut bytes = Vec::new();
        <Self as Serializable>::serialize(self, &mut bytes).map_err(serde::ser::Error::custom)?;
        ser.serialize_bytes(&bytes)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for TransactionHash {
    fn deserialize<D: serde::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_bytes(BorshVisitor(PhantomData))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ContractCallRawData {
    pub(crate) address: Vec<u8>,
    pub(crate) guaranteed_annot: TranscriptAnnotation,
    pub(crate) fallible_annot: TranscriptAnnotation,
}

#[derive(PartialEq, Eq, Versioned)]
#[derive_where(Clone)]
pub struct ContractCall<P: Proofish<D>, D: DB> {
    pub address: ContractAddress,
    pub entry_point: EntryPointBuf,
    // nb: Vector is *not* sorted
    pub guaranteed_transcript: Option<Transcript<D>>,
    pub fallible_transcript: Option<Transcript<D>>,

    pub communication_commitment: Fr,
    pub proof: P::Proof,

    pub(crate) binding_input_data: Option<ContractCallRawData>,
}

impl<P: Serializable + Proofish<D>, D: DB> Serializable for ContractCall<P, D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        serialize_or_data(
            &value.address,
            value.binding_input_data.as_ref().map(|d| &d.address),
            writer,
        )?;
        Serializable::serialize(&value.entry_point, writer)?;
        serialize_or_data(
            &value.guaranteed_transcript,
            value
                .binding_input_data
                .as_ref()
                .map(|d| &d.guaranteed_annot.full_raw),
            writer,
        )?;
        serialize_or_data(
            &value.fallible_transcript,
            value
                .binding_input_data
                .as_ref()
                .map(|d| &d.fallible_annot.full_raw),
            writer,
        )?;
        Serializable::serialize(&value.communication_commitment, writer)?;
        Serializable::serialize(&value.proof, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        value
            .binding_input_data
            .as_ref()
            .map(|d| {
                d.address.len()
                    + d.guaranteed_annot.full_raw.len()
                    + d.fallible_annot.full_raw.len()
            })
            .unwrap_or_else(|| {
                Serializable::serialized_size(&value.address)
                    + Serializable::serialized_size(&value.guaranteed_transcript)
                    + Serializable::serialized_size(&value.fallible_transcript)
            })
            + Serializable::serialized_size(&value.entry_point)
            + Serializable::serialized_size(&value.communication_commitment)
            + Serializable::serialized_size(&value.proof)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TranscriptAnnotation {
    full_raw: Vec<u8>,
    pub(crate) effects_raw: Option<Vec<u8>>,
}

fn read_annotated_transcript<R: Read, D: DB>(
    reader: &mut R,
    recursion_depth: u32,
) -> std::io::Result<(Option<Transcript<D>>, TranscriptAnnotation)> {
    let mut discrim = [0u8];
    let mut reader = CapturingReader::new(reader);
    reader.read_exact(&mut discrim)?;
    match discrim[0] {
        0 => Ok((
            None,
            TranscriptAnnotation {
                full_raw: reader.into_inner().1,
                effects_raw: None,
            },
        )),
        1 => {
            const MAX_VER: Version = Version { major: 2, minor: 2 };
            const _: () = check_injected_version(Some(MAX_VER), Transcript::<InMemoryDB>::VERSION);
            let mut version = [0u8; 2];
            reader.read_exact(&mut version)?;
            let version = Version {
                major: version[0],
                minor: version[1],
            };
            if version.major != MAX_VER.major || version.minor > MAX_VER.minor {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid version for Transcript: {:?}", version),
                ));
            }
            let gas = <u64 as Deserializable>::deserialize(&mut reader, recursion_depth)?;
            let (effects, effects_raw) = deserialize_and_capture(&mut reader, recursion_depth)?;
            let program_len = <u32 as Deserializable>::deserialize(&mut reader, recursion_depth)?;
            let program = (0..program_len)
                .map(|_| {
                    <Op<ResultModeVerify, D>>::versioned_deserialize(
                        &mut reader,
                        Some(&version),
                        recursion_depth,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok((
                Some(Transcript {
                    gas,
                    effects,
                    program,
                    version: Some(version),
                }),
                TranscriptAnnotation {
                    full_raw: reader.into_inner().1,
                    effects_raw: Some(effects_raw),
                },
            ))
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid discriminator in Option: {}", discrim[0]),
        )),
    }
}

impl<P: Deserializable + Proofish<D>, D: DB> Deserializable for ContractCall<P, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let (address, addr_raw) = deserialize_and_capture(reader, recursion_depth)?;
        let entry_point = Deserializable::deserialize(reader, recursion_depth)?;
        let (guaranteed_transcript, guaranteed_annot) =
            read_annotated_transcript(reader, recursion_depth)?;
        let (fallible_transcript, fallible_annot) =
            read_annotated_transcript(reader, recursion_depth)?;
        let communication_commitment = Deserializable::deserialize(reader, recursion_depth)?;
        let proof = Deserializable::deserialize(reader, recursion_depth)?;
        let binding_input_data = ContractCallRawData {
            address: addr_raw,
            guaranteed_annot,
            fallible_annot,
        };
        Ok(ContractCall {
            address,
            entry_point,
            guaranteed_transcript,
            fallible_transcript,
            communication_commitment,
            proof,
            binding_input_data: Some(binding_input_data),
        })
    }
}

impl<P: Proofish<D>, D: DB> Debug for ContractCall<P, D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_map()
            .entry(&Symbol("contract"), &self.address)
            .entry(&Symbol("entry_point"), &self.entry_point)
            .entry(
                &Symbol("guaranteed_transcript"),
                &self.guaranteed_transcript,
            )
            .entry(&Symbol("fallible_transcript"), &self.fallible_transcript)
            .entry(&Symbol("communication_commitment"), &Symbol("<commitment>"))
            .entry(&Symbol("proof"), &Symbol("<proof>"))
            .finish()
    }
}

impl<P: Proofish<D>, D: DB> ContractCall<P, D> {
    pub fn erase_proof(&self) -> ContractCall<(), D> {
        ContractCall {
            address: self.address,
            entry_point: self.entry_point.clone(),
            guaranteed_transcript: self.guaranteed_transcript.clone(),
            fallible_transcript: self.fallible_transcript.clone(),
            communication_commitment: self.communication_commitment,
            proof: (),
            binding_input_data: self.binding_input_data.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Versioned)]
#[derive_where(Clone)]
pub struct ContractDeploy<D: DB> {
    pub initial_state: ContractState<D>,
    pub nonce: HashOutput,
    // Raw serialized data when deserialized
    pub(crate) raw_data: Option<Vec<u8>>,
}

impl<D: DB> Serializable for ContractDeploy<D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        serialize_or_data(
            &(&value.initial_state, &value.nonce),
            value.raw_data.as_ref(),
            writer,
        )
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        value.raw_data.as_ref().map(|d| d.len()).unwrap_or_else(|| {
            Serializable::unversioned_serialized_size(&value.initial_state)
                + Serializable::unversioned_serialized_size(&value.nonce)
        })
    }
}

impl<D: DB> Deserializable for ContractDeploy<D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let mut reader = CapturingReader::with_data(
            reader,
            version.map(|v| vec![v.major, v.minor]).unwrap_or_default(),
        );
        let initial_state = Deserializable::deserialize(&mut reader, recursion_depth)?;
        let nonce = Deserializable::deserialize(&mut reader, recursion_depth)?;
        Ok(ContractDeploy {
            initial_state,
            nonce,
            raw_data: Some(reader.into_inner().1),
        })
    }
}

impl<D: DB> Debug for ContractDeploy<D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("Deploy ")?;
        self.initial_state.fmt(formatter)
    }
}

impl<D: DB> ContractDeploy<D> {
    pub fn address(&self) -> ContractAddress {
        let mut writer = Sha256::new();
        serialize_or_data_mem(&self, self.raw_data.as_ref(), &mut writer);
        ContractAddress(HashOutput(writer.finalize().into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Versioned)]
#[non_exhaustive]
pub enum ContractOperationVersion {
    V2,
}

impl Serializable for ContractOperationVersion {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        use ContractOperationVersion as V;
        match value {
            V::V2 => Serializable::serialize(&1u8, writer),
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        use ContractOperationVersion as V;
        match value {
            V::V2 => 1,
        }
    }
}

impl Deserializable for ContractOperationVersion {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        _recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        use ContractOperationVersion as V;
        let mut disc = vec![0u8; 1];
        reader.read_exact(&mut disc)?;
        match disc[0] {
            0u8 => Err(Self::deserialization_error(
                version,
                format!("Invalid old discriminant {}", disc[0]),
            )),
            1u8 => Ok(V::V2),
            _ => Err(Self::deserialization_error(
                version,
                format!("Unknown discriminant {}", disc[0]),
            )),
        }
    }
}

impl ContractOperationVersion {
    #[cfg(feature = "transaction-semantics")]
    pub(crate) fn has(&self, co: &ContractOperation) -> bool {
        use ContractOperationVersion as V;
        match self {
            V::V2 => co.v2.is_some(),
        }
    }
    #[cfg(feature = "transaction-semantics")]
    pub(crate) fn rm_from(&self, co: &mut ContractOperation) {
        use ContractOperationVersion as V;
        match self {
            V::V2 => co.v2 = None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Versioned)]
#[non_exhaustive]
pub enum ContractOperationVersionedVerifierKey {
    V2(crate::transient_crypto::proofs::VerifierKey),
}

impl ContractOperationVersionedVerifierKey {
    #[cfg(feature = "transaction-semantics")]
    pub(crate) fn as_version(&self) -> ContractOperationVersion {
        use ContractOperationVersion as V;
        use ContractOperationVersionedVerifierKey as VK;
        match self {
            VK::V2(_) => V::V2,
        }
    }

    #[cfg(feature = "transaction-semantics")]
    pub(crate) fn insert_into(&self, co: &mut ContractOperation) {
        use ContractOperationVersionedVerifierKey as VK;
        match self {
            VK::V2(vk) => co.v2 = Some(vk.clone()),
        }
    }
}

impl Serializable for ContractOperationVersionedVerifierKey {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        use ContractOperationVersionedVerifierKey as VK;
        match value {
            VK::V2(vk) => {
                Serializable::serialize(&1u8, writer)?;
                Serializable::serialize(vk, writer)
            }
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        use ContractOperationVersionedVerifierKey as VK;
        match value {
            VK::V2(vk) => 1 + Serializable::serialized_size(vk),
        }
    }
}

impl Deserializable for ContractOperationVersionedVerifierKey {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        use ContractOperationVersionedVerifierKey as VK;
        let mut disc = vec![0u8; 1];
        reader.read_exact(&mut disc)?;
        match disc[0] {
            0u8 => Err(Self::deserialization_error(
                version,
                format!("Invalid old discriminant {}", disc[0]),
            )),
            1u8 => Ok(VK::V2(Deserializable::deserialize(
                reader,
                recursion_depth,
            )?)),
            _ => Err(Self::deserialization_error(
                version,
                format!("Unknown discriminant {}", disc[0]),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Versioned)]
pub enum SingleUpdate {
    /// Replaces the authority for this contract.
    /// Any subsequent updates in this update sequence are still carried out.
    ReplaceAuthority(ContractMaintenanceAuthority),
    /// Removes a verifier key associated with a given version and entry point.
    VerifierKeyRemove(EntryPointBuf, ContractOperationVersion),
    /// Inserts a new verifier key under a given version and entry point.
    /// This operations *does not* replace existing keys, which must first be
    /// explicitly removed.
    VerifierKeyInsert(EntryPointBuf, ContractOperationVersionedVerifierKey),
}

impl Serializable for SingleUpdate {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        match value {
            SingleUpdate::ReplaceAuthority(auth) => {
                Serializable::serialize(&0u8, writer)?;
                Serializable::serialize(auth, writer)
            }
            SingleUpdate::VerifierKeyRemove(entry_point, ver) => {
                Serializable::serialize(&1u8, writer)?;
                Serializable::serialize(entry_point, writer)?;
                Serializable::serialize(ver, writer)
            }
            SingleUpdate::VerifierKeyInsert(entry_point, vervk) => {
                Serializable::serialize(&2u8, writer)?;
                Serializable::serialize(entry_point, writer)?;
                Serializable::serialize(vervk, writer)
            }
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        1 + match value {
            SingleUpdate::ReplaceAuthority(auth) => Serializable::serialized_size(auth),
            SingleUpdate::VerifierKeyRemove(entry_point, ver) => {
                Serializable::serialized_size(entry_point) + Serializable::serialized_size(ver)
            }
            SingleUpdate::VerifierKeyInsert(entry_point, vervk) => {
                Serializable::serialized_size(entry_point) + Serializable::serialized_size(vervk)
            }
        }
    }
}

impl Deserializable for SingleUpdate {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let mut disc = vec![0u8; 1];
        reader.read_exact(&mut disc)?;
        match disc[0] {
            0u8 => Ok(SingleUpdate::ReplaceAuthority(Deserializable::deserialize(
                reader,
                recursion_depth,
            )?)),
            1u8 => Ok(SingleUpdate::VerifierKeyRemove(
                Deserializable::deserialize(reader, recursion_depth)?,
                Deserializable::deserialize(reader, recursion_depth)?,
            )),
            2u8 => Ok(SingleUpdate::VerifierKeyInsert(
                Deserializable::deserialize(reader, recursion_depth)?,
                Deserializable::deserialize(reader, recursion_depth)?,
            )),
            _ => Err(Self::deserialization_error(
                version,
                format!("Unknown discriminant {}", disc[0]),
            )),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MaintenanceUpdate {
    pub address: ContractAddress,
    pub updates: Vec<SingleUpdate>,
    pub counter: u32,
    pub signatures: Vec<(u32, Signature)>,
    pub(crate) raw_data_to_sign: Option<Vec<u8>>,
    pub(crate) raw_data: Option<Vec<u8>>,
}

impl Debug for MaintenanceUpdate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaintenanceUpdate")
            .field("address", &self.address)
            .field("updates", &self.updates)
            .field("counter", &self.counter)
            .field("signatures", &self.signatures)
            .finish()
    }
}

impl Serializable for MaintenanceUpdate {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        serialize_or_data(
            &(
                &value.address,
                &value.updates,
                &value.counter,
                &value.signatures,
            ),
            value.raw_data.as_ref().map(|data| &data[2..]),
            writer,
        )
    }

    fn serialize<W: std::io::Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        let Some(Version { major, minor }) = MaintenanceUpdate::VERSION else {
            unreachable!()
        };
        serialize_or_data(
            &(
                (major, minor),
                &value.address,
                &value.updates,
                &value.counter,
                &value.signatures,
            ),
            value.raw_data.as_ref(),
            writer,
        )
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        value.raw_data.as_ref().map(|d| d.len()).unwrap_or_else(|| {
            Serializable::serialized_size(&value.address)
                + Serializable::serialized_size(&value.updates)
                + Serializable::serialized_size(&value.counter)
                + Serializable::serialized_size(&value.signatures)
        })
    }
}

impl MaintenanceUpdate {
    pub fn data_to_sign(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(b"midnight:contract-update:");
        serialize_or_data_mem(
            &(&self.address, &self.updates, &self.counter),
            self.raw_data_to_sign.as_ref(),
            &mut data,
        );
        data
    }
}

impl Versioned for MaintenanceUpdate {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
}

impl Deserializable for MaintenanceUpdate {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let mut reader = CapturingReader::with_data(
            reader,
            version.map(|v| vec![v.major, v.minor]).unwrap_or_default(),
        );
        match version {
            Some(Version { major: 1, minor: 0 }) => {
                let address = Deserializable::deserialize(&mut reader, recursion_depth)?;
                let updates = Deserializable::deserialize(&mut reader, recursion_depth)?;
                let counter = Deserializable::deserialize(&mut reader, recursion_depth)?;
                let raw_data_to_sign = if version.is_some() {
                    reader.data[2..].to_vec()
                } else {
                    reader.data.clone()
                };
                let signatures = Deserializable::deserialize(&mut reader, recursion_depth)?;
                Ok(MaintenanceUpdate {
                    address,
                    updates,
                    counter,
                    signatures,
                    raw_data: Some(reader.into_inner().1),
                    raw_data_to_sign: Some(raw_data_to_sign),
                })
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version".to_owned(),
            )),
        }
    }
}

#[derive(PartialEq, Eq, Versioned)]
#[derive_where(Clone; P)]
pub enum ContractAction<P: Proofish<D>, D: DB> {
    Call(Box<ContractCall<P, D>>),
    Deploy(ContractDeploy<D>),
    Maintain(MaintenanceUpdate),
}

impl<P: Serializable + Proofish<D>, D: DB> Serializable for ContractAction<P, D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        match value {
            Self::Call(c) => {
                <u8 as Serializable>::serialize(&0, writer)?;
                <ContractCall<P, D> as Serializable>::serialize(c, writer)?;
            }
            Self::Deploy(c) => {
                <u8 as Serializable>::serialize(&1, writer)?;
                <ContractDeploy<D> as Serializable>::serialize(c, writer)?;
            }
            Self::Maintain(c) => {
                <u8 as Serializable>::serialize(&2, writer)?;
                <MaintenanceUpdate as Serializable>::serialize(c, writer)?;
            }
        }

        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        match value {
            Self::Call(c) => 1 + ContractCall::<P, D>::unversioned_serialized_size(c),
            Self::Deploy(c) => 1 + ContractDeploy::unversioned_serialized_size(c),
            Self::Maintain(c) => 1 + MaintenanceUpdate::unversioned_serialized_size(c),
        }
    }
}

impl<P: Deserializable + Proofish<D>, D: DB> Deserializable for ContractAction<P, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let disc = <u8 as Deserializable>::deserialize(reader, recursion_depth)?;
        match disc {
            0 => Ok(Self::Call(Box::new(ContractCall::<P, D>::deserialize(
                reader,
                recursion_depth,
            )?))),
            1 => Ok(Self::Deploy(ContractDeploy::deserialize(
                reader,
                recursion_depth,
            )?)),
            2 => Ok(Self::Maintain(MaintenanceUpdate::deserialize(
                reader,
                recursion_depth,
            )?)),
            _ => Err(Self::deserialization_error(
                None.as_ref(),
                format!("Unknown discriminant: {disc}"),
            )),
        }
    }
}

impl<P: Proofish<D>, D: DB> From<ContractCall<P, D>> for ContractAction<P, D> {
    fn from(call: ContractCall<P, D>) -> Self {
        ContractAction::Call(Box::new(call))
    }
}

impl<P: Proofish<D>, D: DB> From<ContractDeploy<D>> for ContractAction<P, D> {
    fn from(deploy: ContractDeploy<D>) -> Self {
        ContractAction::Deploy(deploy)
    }
}

impl<P: Proofish<D>, D: DB> From<MaintenanceUpdate> for ContractAction<P, D> {
    fn from(upd: MaintenanceUpdate) -> Self {
        ContractAction::Maintain(upd)
    }
}

impl<P: Proofish<D>, D: DB> Debug for ContractAction<P, D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            ContractAction::Call(call) => call.fmt(formatter),
            ContractAction::Deploy(deploy) => deploy.fmt(formatter),
            ContractAction::Maintain(upd) => upd.fmt(formatter),
        }
    }
}

impl<P: Proofish<D>, D: DB> ContractAction<P, D> {
    pub fn erase_proof(&self) -> ContractAction<(), D> {
        match self {
            ContractAction::Call(call) => ContractAction::Call(Box::new(call.erase_proof())),
            ContractAction::Deploy(deploy) => ContractAction::Deploy(deploy.clone()),
            ContractAction::Maintain(upd) => ContractAction::Maintain(upd.clone()),
        }
    }
}

#[derive(PartialEq, Eq, Versioned)]
#[derive_where(Clone)]
pub struct ContractCalls<P: Proofish<D>, D: DB> {
    pub calls: Vec<ContractAction<P, D>>,
    // NOTE: The challenge of this commitment should include, in sequence:
    //  - For each call:
    //    - The call's `contract` field
    //    - The call's `entry_point` field
    //    - The call's `transcript` field
    //  - All deploys
    pub binding_commitment: P::Pedersen,
}

impl<P: Proofish<D>, D: DB> Serializable for ContractCalls<P, D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        Serializable::serialize(&value.calls, writer)?;
        Serializable::serialize(&value.binding_commitment, writer)?;
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::serialized_size(&value.calls)
            + Serializable::serialized_size(&value.binding_commitment)
    }
}

impl<P: Proofish<D>, D: DB> Deserializable for ContractCalls<P, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        Ok(Self {
            calls: Deserializable::deserialize(reader, recursion_depth)?,
            binding_commitment: Deserializable::deserialize(reader, recursion_depth)?,
        })
    }
}

impl<P: Proofish<D>, D: DB> Debug for ContractCalls<P, D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.calls.fmt(formatter)
    }
}

impl<P: Proofish<D>, D: DB> ContractCalls<P, D> {
    pub fn erase_proofs<R: Rng + CryptoRng>(&self, rng: &mut R) -> ContractCalls<(), D> {
        ContractCalls {
            calls: self.calls.iter().map(ContractAction::erase_proof).collect(),
            binding_commitment: self
                .binding_commitment
                .upgrade(rng, &Self::challenge_pre_for(&self.calls[..])[..]),
        }
    }

    pub(crate) fn challenge_pre_for(calls: &[ContractAction<P, D>]) -> Vec<u8> {
        let mut data = Vec::new();
        for cd in calls.iter() {
            match cd {
                ContractAction::Call(call) => {
                    data.push(0u8);
                    serialize_or_data_mem(
                        &call.address,
                        call.binding_input_data.as_ref().map(|d| &d.address),
                        &mut data,
                    );
                    serialize_or_data_mem(&call.entry_point, None::<&[u8]>, &mut data);
                    serialize_or_data_mem(
                        &call.guaranteed_transcript,
                        call.binding_input_data
                            .as_ref()
                            .map(|d| &d.guaranteed_annot.full_raw),
                        &mut data,
                    );
                    serialize_or_data_mem(
                        &call.fallible_transcript,
                        call.binding_input_data
                            .as_ref()
                            .map(|d| &d.fallible_annot.full_raw),
                        &mut data,
                    );
                }
                ContractAction::Deploy(deploy) => {
                    data.push(1u8);
                    serialize_or_data_mem(&deploy, deploy.raw_data.as_ref(), &mut data);
                }
                ContractAction::Maintain(upd) => {
                    data.push(2u8);
                    serialize_or_data_mem(&upd, upd.raw_data.as_ref(), &mut data);
                }
            }
        }
        data
    }

    pub fn calls(&'_ self) -> impl Iterator<Item = &'_ ContractCall<P, D>> + '_ {
        self.calls.iter().filter_map(|cd| match cd {
            ContractAction::Call(call) => Some(call.as_ref()),
            _ => None,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Versioned, Serializable, Deserializable)]
pub enum TransactionIdentifier {
    Merged(Pedersen),
    Unique(HashOutput),
}

#[cfg(feature = "serde")]
impl Serialize for TransactionIdentifier {
    fn serialize<S: serde::ser::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut bytes = Vec::new();
        <Self as Serializable>::serialize(self, &mut bytes).map_err(serde::ser::Error::custom)?;
        ser.serialize_bytes(&bytes)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for TransactionIdentifier {
    fn deserialize<D: serde::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_bytes(BorshVisitor(PhantomData))
    }
}

#[derive(Debug, Storable)]
#[derive_where(Clone, PartialEq)]
#[storable(db = D)]
pub struct LedgerState<D: DB> {
    pub parameters: LedgerParameters,
    pub unminted_native_token_supply: u128,
    pub unclaimed_mints: Map<coin_structure::coin::PublicKey, Map<TokenType, u128, D>, D>,
    pub treasury: Map<TokenType, u128, D>,
    pub zswap: zswap::ledger::State<D>,
    pub contract: Map<ContractAddress, ContractState<D>, D>,
}

impl<D: DB> Versioned for LedgerState<D> {
    const VERSION: Option<Version> = Some(Version { major: 4, minor: 0 });
}

impl<D: DB> Serializable for LedgerState<D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        Serializable::serialize(&value.parameters, writer)?;
        Serializable::serialize(&value.unminted_native_token_supply, writer)?;
        Serializable::serialize(&value.unclaimed_mints, writer)?;
        Serializable::serialize(&value.treasury, writer)?;
        Serializable::serialize(&value.zswap, writer)?;
        Serializable::serialize(&value.contract, writer)?;
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::serialized_size(&value.parameters)
            + Serializable::serialized_size(&value.unminted_native_token_supply)
            + Serializable::serialized_size(&value.unclaimed_mints)
            + Serializable::serialized_size(&value.treasury)
            + Serializable::serialized_size(&value.zswap)
            + Serializable::serialized_size(&value.contract)
    }
}

impl<D: DB> Deserializable for LedgerState<D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 4, minor: 0 }) => Ok(Self {
                parameters: LedgerParameters::deserialize(reader, recursion_depth)?,
                unminted_native_token_supply: Deserializable::deserialize(reader, recursion_depth)?,
                unclaimed_mints: Deserializable::deserialize(reader, recursion_depth)?,
                treasury: Deserializable::deserialize(reader, recursion_depth)?,
                zswap: zswap::ledger::State::deserialize(reader, recursion_depth)?,
                contract: Deserializable::deserialize(reader, recursion_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

/// The maximum mintable supply of NIGHT atomic units. 24 billion NIGHT
/// with an atomic unit at 10^-6.
#[allow(clippy::inconsistent_digit_grouping)]
pub const MAX_SUPPLY: u128 = 24_000_000_000___000_000;

impl<D: DB> Default for LedgerState<D>
where
    MerkleTree<Option<ContractAddress>>: 'static,
{
    fn default() -> Self {
        LedgerState {
            parameters: DUMMY_PARAMETERS,
            unminted_native_token_supply: MAX_SUPPLY,
            unclaimed_mints: Map::new(),
            treasury: Map::new(),
            zswap: zswap::ledger::State::new(),
            contract: Map::new(),
        }
    }
}

impl<D: DB> LedgerState<D>
where
    MerkleTree<Option<ContractAddress>>: 'static,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn index(&self, address: ContractAddress) -> Option<ContractState<D>> {
        self.contract.get(&address).cloned()
    }

    pub fn update_index(&self, address: ContractAddress, context: QueryContext<D>) -> Self {
        let contract = self.contract.get(&address).cloned().unwrap_or_default();
        let contract = ContractState {
            data: context.state,
            ..contract
        };
        LedgerState {
            contract: self.contract.insert(address, contract),
            ..self.clone()
        }
    }
}

struct Symbol(&'static str);

impl Debug for Symbol {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[cfg(feature = "serde")]
struct BorshVisitor<T>(PhantomData<T>);

#[cfg(feature = "serde")]
impl<T: Deserializable> serde::de::Visitor<'_> for BorshVisitor<T> {
    type Value = T;
    fn expecting(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "Borsh-serialized {}", std::any::type_name::<T>())
    }
    fn visit_bytes<E: serde::de::Error>(self, mut v: &[u8]) -> Result<Self::Value, E> {
        Deserializable::deserialize(&mut v, 0).map_err(serde::de::Error::custom)
    }
}

impl TransactionCostModel {
    pub const fn input_fee_overhead(&self) -> u128 {
        const MAX_BYTES: u128 = 6296;
        self.transaction_cost_per_byte as u128 * MAX_BYTES
            + self.verify_cost_constant as u128
            + self.verify_cost_linear as u128 * zswap::INPUT_PIS as u128
    }

    pub const fn output_fee_overhead(&self) -> u128 {
        const MAX_BYTES: u128 = 6264;
        self.transaction_cost_per_byte as u128 * MAX_BYTES
            + self.verify_cost_constant as u128
            + self.verify_cost_linear as u128 * zswap::OUTPUT_PIS as u128
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(
        feature = "proving",
        feature = "transaction-construction",
        feature = "test-utilities"
    ))]
    use crate::test_utilities::PUBLIC_PARAMS;

    #[test]
    fn test_state_serialized() {
        let state = LedgerState::<InMemoryDB>::new();
        let mut ser = Vec::new();
        serialize::serialize(&state, &mut ser, serialize::NetworkId::Undeployed).unwrap();
        serialize::deserialize::<LedgerState<InMemoryDB>, _>(
            &ser[..],
            serialize::NetworkId::Undeployed,
        )
        .unwrap();
    }

    #[test]
    #[cfg(all(
        feature = "proving",
        feature = "transaction-construction",
        feature = "test-utilities"
    ))]
    fn test_fee_overheads() {
        use futures::executor::block_on;
        use rand::Rng;
        use rand::rngs::OsRng;
        use zswap::keys::SecretKeys;

        let nonce = OsRng.gen();
        let mut local = zswap::local::State::<InMemoryDB>::new();
        let sks = SecretKeys::from_rng_seed(&mut OsRng);
        let coin = coin_structure::coin::Info {
            nonce,
            type_: NATIVE_TOKEN,
            value: 42,
        };
        let out = block_on(
            zswap::Output::new(&mut OsRng, &coin, 0, &sks.coin_public_key(), None)
                .unwrap()
                .prove(OsRng, &*PUBLIC_PARAMS, &*PUBLIC_PARAMS),
        )
        .unwrap();
        assert_eq!(
            DUMMY_TRANSACTION_COST_MODEL.output_fee_overhead(),
            dbg!(zswap::Output::<BaseProof>::serialized_size(&out)) as u128
                * DUMMY_TRANSACTION_COST_MODEL.transaction_cost_per_byte as u128
                + zswap::OUTPUT_PIS as u128
                    * DUMMY_TRANSACTION_COST_MODEL.verify_cost_linear as u128
                + DUMMY_TRANSACTION_COST_MODEL.verify_cost_constant as u128
        );
        local = local.watch_for(&sks.coin_public_key(), &coin);
        local = local.apply(
            &sks,
            &zswap::Offer {
                inputs: vec![],
                outputs: vec![out],
                transient: vec![],
                deltas: vec![(NATIVE_TOKEN, -42)],
            },
        );
        let inp = block_on(
            local
                .spend(&mut OsRng, &sks, &local.coins.iter().next().unwrap().1, 0)
                .unwrap()
                .1
                .prove(OsRng, &*PUBLIC_PARAMS, &*PUBLIC_PARAMS),
        )
        .unwrap();
        let real_cost = dbg!(zswap::Input::<BaseProof>::serialized_size(&inp) as u128)
            * DUMMY_TRANSACTION_COST_MODEL.transaction_cost_per_byte as u128
            + zswap::INPUT_PIS as u128 * DUMMY_TRANSACTION_COST_MODEL.verify_cost_linear as u128
            + DUMMY_TRANSACTION_COST_MODEL.verify_cost_constant as u128;
        let input_overhead = DUMMY_TRANSACTION_COST_MODEL.input_fee_overhead();
        // There can be slight variation due to randomness
        assert!(dbg!(input_overhead) >= dbg!(real_cost));
        assert!(input_overhead - 50 <= real_cost);
    }
}
