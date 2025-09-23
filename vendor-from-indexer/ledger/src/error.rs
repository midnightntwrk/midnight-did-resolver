use crate::base_crypto::fab::{Alignment, Value};
use crate::base_crypto::hash::HashOutput;
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::proofs::{KeyLocation, ProvingError, VerifyingError};
use coin_structure::coin::{self, Commitment, TokenType};
use coin_structure::contract::Address as ContractAddress;
use onchain_runtime::context::Effects;
use onchain_runtime::error::TranscriptRejected;
use onchain_runtime::state::EntryPointBuf;
use onchain_runtime::transcript::Transcript;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use zswap::storage::db::DB;

use crate::structure::ContractOperationVersion;

#[derive(Debug)]
pub enum SystemTransactionError {
    IllegalMint {
        amount: Option<u128>,
        supply: u128,
    },
    InsufficientTreasuryFunds {
        requested: Option<u128>,
        actual: u128,
        token_type: TokenType,
    },
    CommitmentAlreadyPresent(Commitment),
}

impl Display for SystemTransactionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            SystemTransactionError::IllegalMint {
                amount: Some(amount),
                supply,
            } => write!(
                f,
                "illegal mint of {amount} native tokens, exceeding remaining supply of {supply}"
            ),
            SystemTransactionError::IllegalMint {
                amount: None,
                supply,
            } => write!(
                f,
                "illegal mint of > 2^128 native tokens, exceeding remaining supply of {supply}"
            ),
            SystemTransactionError::InsufficientTreasuryFunds {
                requested: Some(requested),
                actual,
                token_type,
            } => write!(
                f,
                "insufficient funds in the treasury; {requested} of token {token_type:?} requested, but only {actual} available"
            ),
            SystemTransactionError::InsufficientTreasuryFunds {
                requested: None,
                actual,
                token_type,
            } => write!(
                f,
                "insufficient funds in the treasury; > 2^128 of token {token_type:?} requested, but only {actual} available"
            ),
            SystemTransactionError::CommitmentAlreadyPresent(cm) => {
                write!(f, "faerie-gold attempt with commitment {:?}", cm)
            }
        }
    }
}

impl Error for SystemTransactionError {}

#[derive(Debug)]
#[non_exhaustive]
pub enum TransactionInvalid<D: DB> {
    EffectsMismatch {
        declared: Box<Effects>,
        actual: Box<Effects>,
    },
    ContractAlreadyDeployed(ContractAddress),
    ContractNotPresent(ContractAddress),
    Zswap(zswap::error::TransactionInvalid),
    Transcript(onchain_runtime::error::TranscriptRejected<D>),
    InsufficientClaimable {
        requested: u128,
        token_type: TokenType,
        claimable: u128,
        claimant: coin_structure::coin::PublicKey,
    },
    VerifierKeyNotFound(EntryPointBuf, ContractOperationVersion),
    VerifierKeyAlreadyPresent(EntryPointBuf, ContractOperationVersion),
    ReplayCounterMismatch(ContractAddress),
}

impl<D: DB> Display for TransactionInvalid<D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use TransactionInvalid::*;
        match self {
            EffectsMismatch { declared, actual } => write!(
                formatter,
                "declared effects {declared:?} don't match computed effects {actual:?}"
            ),
            ContractNotPresent(addr) => {
                write!(formatter, "call to non-existant contract {:?}", addr)
            }
            ContractAlreadyDeployed(addr) => {
                write!(formatter, "contract already deployed {:?}", addr)
            }
            Zswap(err) => err.fmt(formatter),
            Transcript(err) => err.fmt(formatter),
            InsufficientClaimable {
                requested,
                token_type,
                claimable,
                claimant,
            } => {
                write!(
                    formatter,
                    "insufficient funds for requested claim: {requested} tokens of type {token_type:?} requested by {claimant:?}; only {claimable} available"
                )
            }
            VerifierKeyNotFound(ep, ver) => write!(
                formatter,
                "the verifier key for {ep:?} version {ver:?} was not present"
            ),
            VerifierKeyAlreadyPresent(ep, ver) => write!(
                formatter,
                "the verifier key for {ep:?} version {ver:?} was already present"
            ),
            ReplayCounterMismatch(addr) => write!(
                formatter,
                "the signed counter for {addr:?} did not match the expected one; likely replay attack"
            ),
        }
    }
}

impl<D: DB> Error for TransactionInvalid<D> {}

impl<D: DB> From<zswap::error::TransactionInvalid> for TransactionInvalid<D> {
    fn from(err: zswap::error::TransactionInvalid) -> TransactionInvalid<D> {
        TransactionInvalid::Zswap(err)
    }
}

impl<D: DB> From<onchain_runtime::error::TranscriptRejected<D>> for TransactionInvalid<D> {
    fn from(err: onchain_runtime::error::TranscriptRejected<D>) -> TransactionInvalid<D> {
        TransactionInvalid::Transcript(err)
    }
}

#[derive(Debug)]
pub struct GuaranteedSectionLimitExceeded {
    pub(crate) real_cost: u128,
    pub(crate) without_size_cost: u128,
    pub(crate) bound: u128,
}

impl Display for GuaranteedSectionLimitExceeded {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "Exceeded the cost limit of the guaranteed section (real cost: {}, without tx size cost: {}, limit: {}",
            self.real_cost, self.without_size_cost, self.bound
        )
    }
}

impl Error for GuaranteedSectionLimitExceeded {}

#[derive(Debug)]
#[non_exhaustive]
pub enum MalformedTransaction {
    VerifierKeyNotSet {
        address: ContractAddress,
        operation: EntryPointBuf,
    },
    TransactionTooLarge {
        tx_size: usize,
        limit: u64,
    },
    VerifierKeyTooLarge {
        actual: u64,
        limit: u64,
    },
    VerifierKeyNotPresent {
        address: ContractAddress,
        operation: EntryPointBuf,
    },
    ContractNotPresent(ContractAddress),
    InvalidProof(VerifyingError),
    BindingCommitmentOpeningInvalid,
    NotNormalized,
    FallibleWithoutCheckpoint,
    ClaimReceiveFailed(coin::Commitment),
    ClaimSpendFailed(coin::Commitment),
    ClaimNullifierFailed(coin::Nullifier),
    ClaimCallFailed {
        address: ContractAddress,
        entry_point: HashOutput,
        comm: Fr,
    },
    InvalidSchnorrProof,
    UnclaimedCoinCom(coin::Commitment),
    UnclaimedNullifier(coin::Nullifier),
    Unbalanced(TokenType, i128),
    Zswap(zswap::error::MalformedOffer),
    BuiltinDecode(crate::base_crypto::fab::InvalidBuiltinDecode),
    GuaranteedLimit(GuaranteedSectionLimitExceeded),
    MergingContracts,
    CantMergeTypes,
    ClaimOverflow,
    ClaimCoinMismatch,
    KeyNotInCommittee {
        address: ContractAddress,
        key_id: usize,
    },
    InvalidCommitteeSignature {
        address: ContractAddress,
        key_id: usize,
    },
    ThresholdMissed {
        address: ContractAddress,
        signatures: usize,
        threshold: usize,
    },
    TooManyZswapEntries,
    UnsupportedProofVersion {
        op_version: String,
    },
    GuaranteedTranscriptVersion {
        op_version: String,
    },
    FallibleTranscriptVersion {
        op_version: String,
    },
}

impl Display for MalformedTransaction {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use MalformedTransaction::*;
        match self {
            ContractNotPresent(addr) => {
                write!(formatter, "call to non-existant contract {:?}", addr)
            }
            VerifierKeyNotPresent { address, operation } => write!(
                formatter,
                "operation {address:?}/{operation:?} does not have a verifier key",
            ),
            InvalidProof(err) => {
                write!(formatter, "failed to verify proof: ")?;
                err.fmt(formatter)
            }
            TransactionTooLarge { tx_size, limit } => write!(
                formatter,
                "transaction too large (size: {tx_size}, limit: {limit})"
            ),
            VerifierKeyTooLarge { actual, limit } => write!(
                formatter,
                "verifier key for operation too large for deserialization (size: {actual}, limit: {limit})"
            ),
            VerifierKeyNotSet { address, operation } => write!(
                formatter,
                "tried to deploy {:?}/{:?} without a verifier key",
                address, operation
            ),
            BindingCommitmentOpeningInvalid => write!(
                formatter,
                "transaction binding commitment was incorrectly opened"
            ),
            FallibleWithoutCheckpoint => write!(
                formatter,
                "fallible transcript did not start with a checkpoint"
            ),
            NotNormalized => write!(formatter, "transaction is not in normal form"),
            ClaimReceiveFailed(com) => write!(
                formatter,
                "failed to claim coin commitment receive for {:?}",
                com
            ),
            ClaimSpendFailed(com) => write!(
                formatter,
                "failed to claim coin commitment spend for {:?}",
                com
            ),
            ClaimNullifierFailed(nul) => write!(
                formatter,
                "failed to claim coin commitment nullifier for {:?}",
                nul
            ),
            ClaimCallFailed {
                address,
                entry_point,
                ..
            } => write!(
                formatter,
                "failed to claim call to {:?}/{:?}",
                address, entry_point
            ),
            InvalidSchnorrProof => write!(
                formatter,
                "failed to verify Fiat-Shamir transformed Schnorr proof"
            ),
            UnclaimedCoinCom(com) => write!(
                formatter,
                "a contract-owned coin output was left unclaimed: {:?}",
                com
            ),
            UnclaimedNullifier(nul) => write!(
                formatter,
                "a contract-owned coin input was unauthorized: {:?}",
                nul
            ),
            Unbalanced(tt, bal) => write!(
                formatter,
                "the transaction has negative balance {} in token type {:?}",
                bal, tt
            ),
            Zswap(err) => err.fmt(formatter),
            BuiltinDecode(err) => err.fmt(formatter),
            GuaranteedLimit(err) => err.fmt(formatter),
            MergingContracts => write!(formatter, "attempted to merge contract transactions"),
            CantMergeTypes => write!(
                formatter,
                "attempted to merge transaction types that are not mergable"
            ),
            ClaimOverflow => write!(formatter, "claimed coin value overflows deltas"),
            ClaimCoinMismatch => write!(
                formatter,
                "declared coin in ClaimMint doesn't match real coin"
            ),
            KeyNotInCommittee { address, key_id } => write!(
                formatter,
                "declared signture for key id {key_id} does not correspond to a committee member for contract {address:?}"
            ),
            InvalidCommitteeSignature { address, key_id } => write!(
                formatter,
                "signature for key id {key_id} invalid for contract {address:?}"
            ),
            ThresholdMissed {
                address,
                signatures,
                threshold,
            } => write!(
                formatter,
                "threshold update for contract {address:?} does not meet required threshold ({signatures}/{threshold} signatures)"
            ),
            TooManyZswapEntries => write!(
                formatter,
                "excessive Zswap entries exceeding 2^16 safety margin"
            ),
            UnsupportedProofVersion { op_version } => write!(
                formatter,
                "unsupported proof version provided for contract operation: {op_version}"
            ),
            GuaranteedTranscriptVersion { op_version } => write!(
                formatter,
                "unsupported guaranteed transcript version provided for contract operation: {op_version}"
            ),
            FallibleTranscriptVersion { op_version } => write!(
                formatter,
                "unsupported fallible transcript version provided for contract operation: {op_version}"
            ),
        }
    }
}

impl Error for MalformedTransaction {}

impl From<zswap::error::MalformedOffer> for MalformedTransaction {
    fn from(err: zswap::error::MalformedOffer) -> MalformedTransaction {
        MalformedTransaction::Zswap(err)
    }
}

impl From<crate::base_crypto::fab::InvalidBuiltinDecode> for MalformedTransaction {
    fn from(err: crate::base_crypto::fab::InvalidBuiltinDecode) -> MalformedTransaction {
        MalformedTransaction::BuiltinDecode(err)
    }
}

impl From<GuaranteedSectionLimitExceeded> for MalformedTransaction {
    fn from(err: GuaranteedSectionLimitExceeded) -> MalformedTransaction {
        MalformedTransaction::GuaranteedLimit(err)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum QueryFailed<D: DB> {
    MissingCall,
    InvalidContract(ContractAddress),
    InvalidInput { value: Value, ty: Alignment },
    Runtime(onchain_runtime::error::TranscriptRejected<D>),
    Zswap(zswap::error::OfferCreationFailed),
}

impl<D: DB> Display for QueryFailed<D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use QueryFailed::*;
        match self {
            MissingCall => write!(formatter, "attempted to run query prior to starting a call"),
            InvalidContract(addr) => write!(formatter, "contract {:?} does not exist", addr),
            InvalidInput { value, ty } => write!(
                formatter,
                "invalid input value {:?} for type {:?}",
                value, ty
            ),
            Runtime(err) => err.fmt(formatter),
            Zswap(err) => err.fmt(formatter),
        }
    }
}

impl<D: DB> Error for QueryFailed<D> {}

impl<D: DB> From<zswap::error::OfferCreationFailed> for QueryFailed<D> {
    fn from(err: zswap::error::OfferCreationFailed) -> QueryFailed<D> {
        QueryFailed::Zswap(err)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum TransactionConstructionError {
    TransactionEmpty,
    UnfinishedCall {
        address: ContractAddress,
        operation: EntryPointBuf,
    },
    ProofFailed(ProvingError),
    MissingVerifierKey {
        address: ContractAddress,
        operation: EntryPointBuf,
    },
}

impl Display for TransactionConstructionError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use TransactionConstructionError::*;
        match self {
            TransactionEmpty => write!(formatter, "attempted to create empty transaction"),
            UnfinishedCall { address, operation } => write!(
                formatter,
                "unfinished call to {:?}/{:?}",
                address, operation
            ),
            ProofFailed(err) => {
                err.fmt(formatter)?;
                write!(formatter, " -- while assembling transaction")
            }
            MissingVerifierKey { address, operation } => write!(
                formatter,
                "attempted to create proof for {:?}/{:?}, which lacks a verifier key",
                address, operation,
            ),
        }
    }
}

impl Error for TransactionConstructionError {}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum TransactionProvingError<D: DB> {
    LeftoverEntries {
        address: ContractAddress,
        entry_point: EntryPointBuf,
        entries: Transcript<D>,
    },
    RanOutOfEntries {
        address: ContractAddress,
        entry_point: EntryPointBuf,
    },
    MissingKeyset(KeyLocation),
    Proving(ProvingError),
}

impl<D: DB> Display for TransactionProvingError<D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        use TransactionProvingError::*;
        match self {
            LeftoverEntries {
                address,
                entry_point,
                entries,
            } => write!(
                formatter,
                "too many transcript entries for {:?}/{:?}: {:?} leftover",
                address, entry_point, entries
            ),
            RanOutOfEntries {
                address,
                entry_point,
            } => write!(
                formatter,
                "ran out of transcript entries for {:?}/{:?}",
                address, entry_point
            ),
            MissingKeyset(keyloc) => write!(
                formatter,
                "attempted proof, but couldn't find keys with ID {keyloc:?}"
            ),
            Proving(e) => e.fmt(formatter),
        }
    }
}

impl<D: DB> Error for TransactionProvingError<D> {}

impl<D: DB> From<ProvingError> for TransactionProvingError<D> {
    fn from(err: ProvingError) -> TransactionProvingError<D> {
        TransactionProvingError::Proving(err)
    }
}

#[derive(Debug)]
pub enum PartitionFailure<D: DB> {
    Transcript(TranscriptRejected<D>),
    NonForest,
}

impl<D: DB> From<TranscriptRejected<D>> for PartitionFailure<D> {
    fn from(err: TranscriptRejected<D>) -> Self {
        PartitionFailure::Transcript(err)
    }
}

impl<D: DB> Display for PartitionFailure<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PartitionFailure::NonForest => {
                write!(f, "call graph was not a forest; cannot partition")
            }
            PartitionFailure::Transcript(e) => e.fmt(f),
        }
    }
}

impl<D: DB> Error for PartitionFailure<D> {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            PartitionFailure::Transcript(err) => Some(err),
            _ => None,
        }
    }
}
