use crate::base_crypto::hash::{BLANK_HASH, HashOutput};
use crate::base_crypto::repr::MemWrite;
use crate::coin::{PublicKey, SecretKey};
use crate::contract::Address;
#[cfg(feature = "proptest")]
use crate::serialize::randomised_serialization_test;
use crate::serialize::{self, Deserializable, Serializable, Versioned};
#[cfg(all(feature = "proptest", test))]
use crate::serialize::{NetworkId, deserialize, serialize, serialized_size};
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::repr::{FieldRepr, FromFieldRepr};
#[cfg(any(test, feature = "fake"))]
use fake::Dummy;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Versioned)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub enum Recipient {
    User(PublicKey),
    Contract(Address),
}

impl Deserializable for Recipient {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&serialize::Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let variant = <u8 as Deserializable>::deserialize(reader, recursion_depth)?;
        match variant {
            0 => Ok(Self::User(<PublicKey as Deserializable>::deserialize(
                reader,
                recursion_depth,
            )?)),
            1 => Ok(Self::Contract(<Address as Deserializable>::deserialize(
                reader,
                recursion_depth,
            )?)),
            _ => Err(Self::deserialization_error(
                None,
                format!("Unrecognized discriminant: {}.", variant),
            )),
        }
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(Recipient);

impl Serializable for Recipient {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        match value {
            Self::User(u) => {
                <u8 as Serializable>::serialize(&0, writer)?;
                <PublicKey as Serializable>::serialize(u, writer)?;
            }
            Self::Contract(c) => {
                <u8 as Serializable>::serialize(&1, writer)?;
                <Address as Serializable>::serialize(c, writer)?;
            }
        }

        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        1 + match value {
            Self::User(u) => PublicKey::serialized_size(u),
            Self::Contract(a) => Address::serialized_size(a),
        }
    }
}

impl FieldRepr for Recipient {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        match self {
            Recipient::User(pk) => {
                true.field_repr(writer);
                pk.0.field_repr(writer);
                BLANK_HASH.field_repr(writer);
            }
            Recipient::Contract(addr) => {
                false.field_repr(writer);
                BLANK_HASH.field_repr(writer);
                addr.0.field_repr(writer);
            }
        }
    }
    fn field_size(&self) -> usize {
        <HashOutput as FromFieldRepr>::FIELD_SIZE * 2 + <bool as FromFieldRepr>::FIELD_SIZE
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Versioned)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub enum SenderEvidence {
    User(SecretKey),
    Contract(Address),
}

impl Serializable for SenderEvidence {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        match value {
            Self::User(s) => {
                <u8 as Serializable>::serialize(&0, writer)?;
                <SecretKey as Serializable>::serialize(s, writer)?;
            }
            Self::Contract(c) => {
                <u8 as Serializable>::serialize(&1, writer)?;
                <Address as Serializable>::serialize(c, writer)?;
            }
        }

        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        1 + match value {
            Self::User(s) => SecretKey::serialized_size(s),
            Self::Contract(a) => Address::serialized_size(a),
        }
    }
}

impl FieldRepr for SenderEvidence {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        match self {
            SenderEvidence::User(sk) => {
                true.field_repr(writer);
                sk.0.field_repr(writer);
                BLANK_HASH.field_repr(writer);
            }
            SenderEvidence::Contract(addr) => {
                false.field_repr(writer);
                BLANK_HASH.field_repr(writer);
                addr.0.field_repr(writer);
            }
        }
    }
    fn field_size(&self) -> usize {
        <HashOutput as FromFieldRepr>::FIELD_SIZE * 2 + <bool as FromFieldRepr>::FIELD_SIZE
    }
}

impl From<&SenderEvidence> for Recipient {
    fn from(se: &SenderEvidence) -> Recipient {
        use SenderEvidence::*;
        match se {
            User(sk) => Recipient::User(sk.public_key()),
            Contract(addr) => Recipient::Contract(*addr),
        }
    }
}
