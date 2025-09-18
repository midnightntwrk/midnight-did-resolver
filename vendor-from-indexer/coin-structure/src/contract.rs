use crate::base_crypto::hash::{HashOutput, persistent_commit};
use crate::base_crypto::repr::{BinaryHashRepr, MemWrite};
use crate::coin::TokenType;
use crate::hash_serde;
use crate::serialize::{self, Deserializable, Serializable, Version, Versioned};
#[cfg(all(feature = "proptest", test))]
use crate::serialize::{NetworkId, deserialize, serialize, serialized_size};
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::repr::{FieldRepr, FromFieldRepr};
#[cfg(any(test, feature = "fake"))]
use fake::Dummy;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;

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
pub struct Address(pub HashOutput);

impl rand::distributions::Distribution<Address> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Address {
        Address(rng.gen())
    }
}

impl Address {
    pub fn custom_token_type(&self, domain_sep: HashOutput) -> TokenType {
        let inner_domain_sep = HashOutput(*b"midnight:derive_token\0\0\0\0\0\0\0\0\0\0\0");
        TokenType(persistent_commit(&(domain_sep, self.0), inner_domain_sep))
    }
}

impl Versioned for Address {
    const VERSION: Option<serialize::Version> = Some(Version { major: 2, minor: 0 });
}

impl Deserializable for Address {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(Address(
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
serialize::randomised_serialization_test!(Address);
hash_serde!(Address);
