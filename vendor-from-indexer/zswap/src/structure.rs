use crate::ZSWAP_TREE_HEIGHT;
use crate::error::MalformedOffer;
use crate::serialize;
use crate::serialize::{Deserializable, Serializable, Version, Versioned};
use crate::storage::storage::Array;
use crate::transient_crypto::merkle_tree::{MerkleTree, MerkleTreeDigest};
use coin_structure::coin::{
    Commitment, Info as CoinInfo, Nullifier, PublicKey as CoinPublicKey, TokenType,
};
use coin_structure::contract::Address as ContractAddress;
use rand::{CryptoRng, Rng};
#[cfg(feature = "serde")]
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Formatter};
use transient_crypto::commitment::{Pedersen, PedersenRandomness};
use transient_crypto::curve::{EmbeddedGroupAffine, Fr};
use transient_crypto::encryption;
use transient_crypto::proofs::ProofPreimage;
use transient_crypto::repr::{FieldRepr, FromFieldRepr};

macro_rules! exptfile {
    ($name:literal, $desc:literal) => {
        (
            concat!("zswap/", include_str!("../../static/version"), "/", $name),
            base_crypto::data_provider::hexhash(
                &include_bytes!(concat!("../../static/zswap/", $name, ".sha256"))
                    .split_at(64)
                    .0,
            ),
            $desc,
        )
    };
}

/// Files provided by Midnight's data provider for Zswap.
pub const ZSWAP_EXPECTED_FILES: &[(&str, [u8; 32], &str)] = &[
    exptfile!(
        "spend.prover",
        "zero-knowledge proving key for Zswap inputs"
    ),
    exptfile!(
        "spend.verifier",
        "zero-knowledge verifying key for Zswap inputs"
    ),
    exptfile!("spend.bzkir", "ZKIR source for Zswap inputs"),
    exptfile!(
        "output.prover",
        "zero-knowledge proving key for Zswap outputs"
    ),
    exptfile!(
        "output.verifier",
        "zero-knowledge verifying key for Zswap outputs"
    ),
    exptfile!("output.bzkir", "ZKIR source for Zswap outputs"),
    exptfile!(
        "sign.prover",
        "zero-knowledge proving key for Zswap signing operations"
    ),
    exptfile!(
        "sign.verifier",
        "zero-knowledge verifying key for Zswap signing operations"
    ),
    exptfile!("sign.bzkir", "ZKIR source for Zswap signing operations"),
];

pub(crate) const COIN_CIPHERTEXT_LEN: usize = 6;
#[derive(Debug, Clone, Hash, PartialEq, Eq, Versioned, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CoinCiphertext {
    pub c: EmbeddedGroupAffine,
    pub ciph: [Fr; COIN_CIPHERTEXT_LEN],
}

impl Serializable for CoinCiphertext {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        <EmbeddedGroupAffine as Serializable>::serialize(&value.c, writer)?;
        // Because this is unversioned we need not send COIN_CIPHERTEXT_LEN
        for elem in value.ciph {
            <Fr as Serializable>::serialize(&elem, writer)?;
        }
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        EmbeddedGroupAffine::serialized_size(&value.c)
            + value
                .ciph
                .iter()
                .map(Serializable::serialized_size)
                .sum::<usize>()
    }
}

impl Deserializable for CoinCiphertext {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursive_depth: u32,
    ) -> Result<Self, std::io::Error> {
        Ok(Self {
            c: EmbeddedGroupAffine::deserialize(reader, recursive_depth)?,
            ciph: {
                let mut res = [Fr::default(); COIN_CIPHERTEXT_LEN];
                for byte in res.iter_mut() {
                    *byte = Fr::deserialize(reader, recursive_depth)?;
                }
                res
            },
        })
    }
}

impl CoinCiphertext {
    pub fn new<R: Rng + CryptoRng + ?Sized>(
        rng: &mut R,
        coin: &CoinInfo,
        pk: encryption::PublicKey,
    ) -> CoinCiphertext {
        pk.encrypt(rng, coin)
            .try_into()
            .expect("ciphertext should have ciphertext length")
    }
}

impl TryFrom<encryption::Ciphertext> for CoinCiphertext {
    type Error = ();

    fn try_from(ciph: encryption::Ciphertext) -> Result<Self, ()> {
        if ciph.ciph.len() != COIN_CIPHERTEXT_LEN {
            return Err(());
        }
        let mut arr = [0.into(); COIN_CIPHERTEXT_LEN];
        arr.copy_from_slice(&ciph.ciph);
        Ok(CoinCiphertext {
            c: ciph.c,
            ciph: arr,
        })
    }
}

impl From<CoinCiphertext> for encryption::Ciphertext {
    fn from(ciph: CoinCiphertext) -> encryption::Ciphertext {
        encryption::Ciphertext {
            c: ciph.c,
            ciph: ciph.ciph.to_vec(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serializable)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// A mint to a specific public key, authorized by the user's private key.
pub struct AuthorizedMint<P> {
    pub coin: CoinInfo,
    pub recipient: CoinPublicKey,
    pub proof: P,
}

impl<P> Versioned for AuthorizedMint<P> {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 0 });
}

impl<P> AuthorizedMint<P> {
    pub fn erase_proof(&self) -> AuthorizedMint<()> {
        AuthorizedMint {
            coin: self.coin,
            recipient: self.recipient,
            proof: (),
        }
    }
}

impl<P: Deserializable> Deserializable for AuthorizedMint<P> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&crate::serialize::Version>,
        recursive_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(Self {
                coin: Deserializable::deserialize(reader, recursive_depth)?,
                recipient: Deserializable::deserialize(reader, recursive_depth)?,
                proof: Deserializable::deserialize(reader, recursive_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Versioned, Serializable, Deserializable)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Input<P> {
    pub nullifier: Nullifier,
    pub value_commitment: Pedersen,
    pub contract_address: Option<ContractAddress>,
    pub merkle_tree_root: MerkleTreeDigest,
    pub proof: P,
}

impl<P> Debug for AuthorizedMint<P> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "<mint of {} of token {:?} for recipient {:?}>",
            self.coin.value, self.coin.type_, self.recipient
        )
    }
}

impl<P> Input<P> {
    pub fn erase_proof(&self) -> Input<()> {
        Input {
            nullifier: self.nullifier,
            value_commitment: self.value_commitment,
            contract_address: self.contract_address,
            merkle_tree_root: self.merkle_tree_root,
            proof: (),
        }
    }
}

impl Input<ProofPreimage> {
    pub fn binding_randomness(&self) -> PedersenRandomness {
        // NOTE: This is tied to the implementation in construct.rs
        // rc is the last input, and should be a single Fr element.
        (*self
            .proof
            .inputs
            .last()
            .expect("must have witness to extract from"))
        .try_into()
        .expect("extracted binding randomness is invalid")
    }
}

impl<P> Debug for Input<P> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self.contract_address {
            Some(addr) => write!(
                formatter,
                "<shielded input {:?} for: {:?}>",
                self.nullifier, addr
            ),
            None => write!(formatter, "<shielded input {:?}>", self.nullifier),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Versioned, Serializable, Deserializable)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Output<P> {
    pub coin_com: Commitment,
    pub value_commitment: Pedersen,
    pub contract_address: Option<ContractAddress>,
    pub ciphertext: Option<CoinCiphertext>,
    pub proof: P,
}

impl<P> Output<P> {
    pub fn erase_proof(&self) -> Output<()> {
        Output {
            coin_com: self.coin_com,
            value_commitment: self.value_commitment,
            contract_address: self.contract_address,
            ciphertext: self.ciphertext.clone(),
            proof: (),
        }
    }
}

impl Output<ProofPreimage> {
    pub fn binding_randomness(&self) -> PedersenRandomness {
        // NOTE: This is tied to the implementation in construct.rs.
        // rc is the last input, and should be a single Fr element.
        // NOTE: rc negated because output commitments are subtracted
        -PedersenRandomness::try_from(
            *self
                .proof
                .inputs
                .last()
                .expect("must have witness to extract from"),
        )
        .expect("extracted binding randomness is invalid")
    }
}

impl<P> Debug for Output<P> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self.contract_address {
            Some(addr) => write!(
                formatter,
                "<shielded output {:?} for: {:?}>",
                self.coin_com, addr
            ),
            None => write!(formatter, "<shielded output {:?}>", self.coin_com),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serializable, Deserializable, Versioned)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Transient<P> {
    pub nullifier: Nullifier,
    pub coin_com: Commitment,
    pub value_commitment_input: Pedersen,
    pub value_commitment_output: Pedersen,
    pub contract_address: Option<ContractAddress>,
    pub ciphertext: Option<CoinCiphertext>,
    pub proof_input: P,
    pub proof_output: P,
}

impl<P> Transient<P> {
    pub fn erase_proof(&self) -> Transient<()> {
        Transient {
            nullifier: self.nullifier,
            coin_com: self.coin_com,
            value_commitment_input: self.value_commitment_input,
            value_commitment_output: self.value_commitment_output,
            contract_address: self.contract_address,
            ciphertext: self.ciphertext.clone(),
            proof_input: (),
            proof_output: (),
        }
    }
}

impl Transient<ProofPreimage> {
    pub fn binding_randomness(&self) -> PedersenRandomness {
        self.as_input().binding_randomness() + self.as_output().binding_randomness()
    }
}

impl<P: Clone> Transient<P> {
    pub(crate) fn as_input(&self) -> Input<P> {
        Input {
            nullifier: self.nullifier,
            value_commitment: self.value_commitment_input,
            contract_address: self.contract_address,
            merkle_tree_root: MerkleTree::<_>::blank(ZSWAP_TREE_HEIGHT)
                .update_hash(0, self.coin_com.0, ())
                .root(),
            proof: self.proof_input.clone(),
        }
    }

    pub(crate) fn as_output(&self) -> Output<P> {
        Output {
            coin_com: self.coin_com,
            value_commitment: self.value_commitment_output,
            contract_address: self.contract_address,
            ciphertext: self.ciphertext.clone(),
            proof: self.proof_output.clone(),
        }
    }
}

impl<P> Debug for Transient<P> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self.contract_address {
            Some(addr) => {
                write!(
                    formatter,
                    "<shielded transient coin {:?} {:?} for: {:?}>",
                    self.coin_com, self.nullifier, addr
                )
            }
            None => write!(
                formatter,
                "<shielded transient coin {:?} {:?}>",
                self.coin_com, self.nullifier
            ),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serializable)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// A Zswap offer consists of a potentially unbalanced set of Zswap
/// inputs/outputs.
///
/// All vectors must be sorted to be valid, and `deltas` must be key-unique
/// (i.e. not contain tuples sharing their first element `(a, b)` and `(a, c)`).
/// This is to have a canonical representation while operating on sets and maps.
pub struct Offer<P> {
    /// A set of Inputs
    pub inputs: Vec<Input<P>>,
    /// A set of Outputs
    pub outputs: Vec<Output<P>>,
    /// A set of "transient" Zswap coins: Coins that are created and spent in
    /// the same transaction
    pub transient: Vec<Transient<P>>,
    /// A map from types (coin colors) to the offer value in this type.
    /// A positive value means more coins have been spent, a negative value
    /// means more coins were created.
    pub deltas: Vec<(TokenType, i128)>,
}

impl<P> Versioned for Offer<P> {
    const VERSION: Option<crate::serialize::Version> = Some(Version { major: 4, minor: 0 });
}

impl<P: Deserializable> Deserializable for Offer<P> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&crate::serialize::Version>,
        recursive_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 4, minor: 0 }) => Ok(Self {
                inputs: Vec::<Input<P>>::deserialize(reader, recursive_depth)?,
                outputs: Vec::<Output<P>>::deserialize(reader, recursive_depth)?,
                transient: Vec::<Transient<P>>::deserialize(reader, recursive_depth)?,
                deltas: Vec::<(TokenType, i128)>::deserialize(reader, recursive_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

impl Offer<ProofPreimage> {
    pub fn binding_randomness(&self) -> PedersenRandomness {
        self.inputs
            .iter()
            .map(|i| i.binding_randomness())
            .chain(self.outputs.iter().map(|o| o.binding_randomness()))
            .chain(self.transient.iter().map(|t| t.binding_randomness()))
            .fold(0.into(), |a, b| a + b)
    }
}

impl<P> Offer<P> {
    pub fn erase_proofs(&self) -> Offer<()> {
        Offer {
            inputs: self.inputs.iter().map(Input::erase_proof).collect(),
            outputs: self.outputs.iter().map(Output::erase_proof).collect(),
            transient: self.transient.iter().map(Transient::erase_proof).collect(),
            deltas: self.deltas.clone(),
        }
    }
}

impl<P> Debug for Offer<P> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_map()
            .entry(&Symbol("inputs"), &self.inputs)
            .entry(&Symbol("outputs"), &self.outputs)
            .entry(&Symbol("transient"), &self.transient)
            .entry(
                &Symbol("deltas"),
                &self
                    .deltas
                    .iter()
                    .copied()
                    .map(DebugDelta)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

struct DebugDelta((TokenType, i128));

impl Debug for DebugDelta {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{:?} -> {:?}", self.0.0, self.0.1)
    }
}

pub fn normalize_deltas<I: Iterator<Item = (TokenType, i128)>>(
    deltas: I,
) -> Vec<(TokenType, i128)> {
    let mut new_deltas: Vec<_> = deltas
        .fold(BTreeMap::new(), |mut map, (k, v)| {
            *map.entry(k).or_insert(0) += v;
            map
        })
        .into_iter()
        .collect();
    new_deltas.retain(|(_, v)| *v != 0);
    new_deltas.sort();
    new_deltas
}

impl<P: Clone + Ord> Offer<P> {
    pub fn normalize(&mut self) {
        self.inputs.sort();
        self.outputs.sort();
        self.transient.sort();
        self.deltas = normalize_deltas(self.deltas.iter().cloned());
    }

    #[instrument(skip(self, other))]
    pub fn merge(&self, other: &Self) -> Result<Self, MalformedOffer> {
        let inputs1: BTreeSet<_> = self.inputs.iter().cloned().collect();
        let inputs2: BTreeSet<_> = other.inputs.iter().cloned().collect();
        let outputs1: BTreeSet<_> = self.outputs.iter().cloned().collect();
        let outputs2: BTreeSet<_> = other.outputs.iter().cloned().collect();
        let transient1: BTreeSet<_> = self.transient.iter().cloned().collect();
        let transient2: BTreeSet<_> = other.transient.iter().cloned().collect();
        if inputs1.is_disjoint(&inputs2)
            && outputs1.is_disjoint(&outputs2)
            && transient1.is_disjoint(&transient2)
        {
            let mut res = Offer {
                inputs: inputs1.into_iter().chain(inputs2.into_iter()).collect(),
                outputs: outputs1.into_iter().chain(outputs2.into_iter()).collect(),
                transient: transient1
                    .iter()
                    .chain(transient2.iter())
                    .cloned()
                    .collect(),
                deltas: self
                    .deltas
                    .iter()
                    .chain(other.deltas.iter())
                    .cloned()
                    .collect(),
            };
            res.normalize();
            Ok(res)
        } else {
            warn!("overlap in coins attempted to merge");
            Err(MalformedOffer::NonDisjointCoinMerge)
        }
    }
}

struct Symbol(&'static str);

impl Debug for Symbol {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

pub const INPUT_PIS: usize = 68;
pub const OUTPUT_PIS: usize = 77;
pub const AUTHORIZED_MINT_PIS: usize = 13;
