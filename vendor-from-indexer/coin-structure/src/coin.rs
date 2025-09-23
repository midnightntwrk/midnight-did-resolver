use crate::base_crypto::hash::persistent_hash;
use crate::base_crypto::repr::{BinaryHashRepr, MemWrite};
use crate::hash_serde;
#[cfg(feature = "proptest")]
use crate::serialize::randomised_serialization_test;
use crate::serialize::{self, Deserializable, Serializable, Version, Versioned};
#[cfg(all(feature = "proptest", test))]
use crate::serialize::{NetworkId, deserialize, serialize, serialized_size};
use crate::transfer::{Recipient, SenderEvidence};
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::hash::HashOutput;
use crate::transient_crypto::hash::{degrade_to_transient, transient_hash, upgrade_from_transient};
use crate::transient_crypto::repr::{FieldRepr, FromFieldRepr};
#[cfg(any(test, feature = "fake"))]
use fake::Dummy;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
use rand::{CryptoRng, Rng};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use storage::db::DB;
use storage::{Storable, arena::ArenaKey, base_storable, storable::Loader};

use std::fmt::{self, Debug, Formatter};

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
    Versioned,
    Serializable,
    Deserializable,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct Nullifier(pub HashOutput);

base_storable!(Nullifier);

impl rand::distributions::Distribution<Nullifier> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Nullifier {
        Nullifier(rng.gen())
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(Nullifier);

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
    Versioned,
    Serializable,
    Deserializable,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct Commitment(pub HashOutput);

base_storable!(Commitment);

impl rand::distributions::Distribution<Commitment> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Commitment {
        Commitment(rng.gen())
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(Commitment);

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
    Versioned,
    Serializable,
    Deserializable,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct Nonce(pub HashOutput);

#[cfg(feature = "proptest")]
randomised_serialization_test!(Nonce);

impl rand::distributions::Distribution<Nonce> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Nonce {
        Nonce(rng.gen())
    }
}

#[derive(
    Default,
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
    Serializable,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct SecretKey(pub HashOutput);

impl Debug for SecretKey {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "<coin secret key>")
    }
}

impl Versioned for SecretKey {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 0 });
}

impl Deserializable for SecretKey {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(Self(
                <HashOutput as Deserializable>::deserialize(reader, recursion_depth)?,
            )),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(SecretKey);

impl SecretKey {
    pub fn public_key(&self) -> PublicKey {
        let mut data = Vec::with_capacity(38);
        self.binary_repr(&mut data);
        data.extend(b"mdn:pk");
        PublicKey(persistent_hash(&data))
    }
}

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
    Versioned,
    Serializable,
    Deserializable,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct PublicKey(pub HashOutput);
impl rand::distributions::Distribution<PublicKey> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> PublicKey {
        PublicKey(rng.gen())
    }
}

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
    Serializable,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct TokenType(pub HashOutput);

impl rand::distributions::Distribution<TokenType> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> TokenType {
        TokenType(rng.gen())
    }
}

impl Versioned for TokenType {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 0 });
}

impl Deserializable for TokenType {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(Self(
                <HashOutput as Deserializable>::deserialize(reader, recursion_depth)?,
            )),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(TokenType);

pub const NATIVE_TOKEN: TokenType = TokenType(HashOutput([0u8; 32]));

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
    Serializable,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
pub struct Info {
    pub nonce: Nonce,
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub type_: TokenType,
    pub value: u128,
}

base_storable!(Info);

impl rand::distributions::Distribution<Info> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Info {
        Info {
            nonce: rng.gen(),
            type_: rng.gen(),
            value: rng.gen(),
        }
    }
}

impl Versioned for Info {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 0 });
}

impl Deserializable for Info {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(Self {
                nonce: <Nonce as Deserializable>::deserialize(reader, recursion_depth)?,
                type_: <TokenType as Deserializable>::deserialize(reader, recursion_depth)?,
                value: <u128 as Deserializable>::deserialize(reader, recursion_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(Info);

impl Info {
    pub fn new<R: Rng + CryptoRng + ?Sized>(rng: &mut R, value: u128, type_: TokenType) -> Self {
        Info {
            nonce: rng.gen(),
            value,
            type_,
        }
    }

    pub fn evolve_from(&self, domain_sep: &[u8], value: u128, type_: TokenType) -> Self {
        Info {
            nonce: Nonce(upgrade_from_transient(transient_hash(&[
                Fr::from_le_bytes(domain_sep).expect("Domain sep should be in range for field"),
                degrade_to_transient(self.nonce.0),
            ]))),
            value,
            type_,
        }
    }

    pub fn commitment(&self, recipient: &Recipient) -> Commitment {
        let mut data = Vec::with_capacity(119);
        self.binary_repr(&mut data);
        match &recipient {
            Recipient::User(d) => (true, d.0).binary_repr(&mut data),
            Recipient::Contract(d) => (false, d.0).binary_repr(&mut data),
        }
        data.extend(b"mdn:cc");
        Commitment(persistent_hash(&data))
    }

    pub fn nullifier(&self, se: &SenderEvidence) -> Nullifier {
        let mut data = Vec::with_capacity(119);
        self.binary_repr(&mut data);
        match &se {
            SenderEvidence::User(d) => (true, d.0).binary_repr(&mut data),
            SenderEvidence::Contract(d) => (false, d.0).binary_repr(&mut data),
        }
        data.extend(b"mdn:cn");
        Nullifier(persistent_hash(&data))
    }

    pub fn qualify(&self, mt_index: u64) -> QualifiedInfo {
        QualifiedInfo {
            nonce: self.nonce,
            value: self.value,
            type_: self.type_,
            mt_index,
        }
    }
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FieldRepr,
    FromFieldRepr,
    BinaryHashRepr,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
pub struct QualifiedInfo {
    pub nonce: Nonce,
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub type_: TokenType,
    pub value: u128,
    pub mt_index: u64,
}

base_storable!(QualifiedInfo);

impl rand::distributions::Distribution<QualifiedInfo> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> QualifiedInfo {
        QualifiedInfo {
            nonce: rng.gen(),
            type_: rng.gen(),
            value: rng.gen(),
            mt_index: rng.gen(),
        }
    }
}

impl Versioned for QualifiedInfo {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 0 });
}

// Serializable can be derived via a macro, but by implementing large
// changes to the structure of QualifiedInfo are flagged by the compiler
// encouraging a programmer to update both Serializable and Deserializable
impl Serializable for QualifiedInfo {
    fn unversioned_serialize<W: std::io::prelude::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        <Nonce as Serializable>::serialize(&value.nonce, writer)?;
        <TokenType as Serializable>::serialize(&value.type_, writer)?;
        <u128 as Serializable>::serialize(&value.value, writer)?;
        <u64 as Serializable>::serialize(&value.mt_index, writer)?;

        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        <Nonce as Serializable>::serialized_size(&value.nonce)
            + <TokenType as Serializable>::serialized_size(&value.type_)
            + <u128 as Serializable>::serialized_size(&value.value)
            + <u64 as Serializable>::serialized_size(&value.mt_index)
    }
}

impl Deserializable for QualifiedInfo {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(Self {
                nonce: <Nonce as Deserializable>::deserialize(reader, recursion_depth)?,
                type_: <TokenType as Deserializable>::deserialize(reader, recursion_depth)?,
                value: <u128 as Deserializable>::deserialize(reader, recursion_depth)?,
                mt_index: <u64 as Deserializable>::deserialize(reader, recursion_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(QualifiedInfo);

impl From<&QualifiedInfo> for Info {
    fn from(qi: &QualifiedInfo) -> Info {
        Info {
            nonce: qi.nonce,
            value: qi.value,
            type_: qi.type_,
        }
    }
}

hash_serde!(
    Nullifier, Commitment, Nonce, TokenType, PublicKey, SecretKey
);
