use crate::{NetworkId, Version, Versioned};
use borsh::BorshSerialize;
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    io::Write,
    time::{Duration, SystemTime},
};

// Top-level serialization function
pub fn serialize<T: Serializable, W: Write>(
    value: &T,
    mut writer: W,
    network_id: NetworkId,
) -> Result<(), std::io::Error> {
    if T::NETWORK_SPECIFIC {
        Serializable::serialize(&network_id, &mut writer)?;
    }
    <T as Serializable>::serialize(value, &mut writer)
}

pub fn serialized_size<T: Serializable>(value: &T) -> usize {
    T::serialized_size(value) + T::NETWORK_SPECIFIC as usize
}

/// Binary serialization with embedded versioning.
///
/// See [`crate::Deserializable`] for the deserialization counterpart.
pub trait Serializable
where
    Self: Sized + Versioned,
{
    // Function broken out to allow custom serialization logic to not interfere
    // with automatic versioning
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error>;

    // Default behaviour serializes the version information and uses
    // BorshSerialize::serialize to serialize the object. This needs overloading
    // objects of Serializable objects
    fn serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        if let Some(version) = Self::VERSION {
            <u8 as Serializable>::serialize(&version.major, writer)?;
            <u8 as Serializable>::serialize(&version.minor, writer)?;
        }
        Self::unversioned_serialize(value, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize;

    fn serialized_size(value: &Self) -> usize {
        let version_size = match Self::VERSION {
            Some(..) => 2,
            _ => 0,
        };
        Self::unversioned_serialized_size(value) + version_size
    }
}

impl Serializable for Version {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        Serializable::serialize(&value.major, writer)?;
        Serializable::serialize(&value.minor, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::serialized_size(&value.major) + Serializable::serialized_size(&value.minor)
    }
}

impl<T: Serializable> Serializable for Vec<T>
where
    Self: Sized,
{
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        <u32 as Serializable>::serialize(&(value.len() as u32), writer)?;
        for elem in value {
            <T as Serializable>::serialize(elem, writer)?
        }
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        4 + value.iter().fold(0, |acc, x| acc + T::serialized_size(x))
    }
}

impl<K: Serializable + Ord, V: Serializable> Serializable for HashMap<K, V> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        <u32 as Serializable>::serialize(&(value.len() as u32), writer)?;
        let mut kvs = value.iter().collect::<Vec<_>>();
        kvs.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        for (k, v) in kvs.into_iter() {
            <K as Serializable>::serialize(k, writer)?;
            <V as Serializable>::serialize(v, writer)?;
        }
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        4 + value.iter().fold(0, |acc, (k, v)| {
            acc + K::serialized_size(k) + V::serialized_size(v)
        })
    }
}

impl<T: Serializable + Ord> Serializable for HashSet<T> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        <u32 as Serializable>::serialize(&(value.len() as u32), writer)?;
        let mut elems = value.iter().collect::<Vec<_>>();
        elems.sort();
        for elem in elems.into_iter() {
            <T as Serializable>::serialize(elem, writer)?;
        }

        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        4 + value
            .iter()
            .fold(0, |acc, elem| acc + T::serialized_size(elem))
    }
}

impl<'a, T> Serializable for &'a T
where
    T: Serializable + 'a,
    Self: Borrow<T>,
{
    fn unversioned_serialize<W: Write>(
        _value: &Self,
        _writer: &mut W,
    ) -> Result<(), std::io::Error> {
        unreachable!()
    }

    fn serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        <T as Serializable>::serialize(value, writer)
    }

    fn unversioned_serialized_size(_value: &Self) -> usize {
        unreachable!()
    }

    fn serialized_size(value: &Self) -> usize {
        T::serialized_size(value)
    }
}

impl<T: Serializable> Serializable for Option<T> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        match value {
            Some(v) => {
                <u8 as Serializable>::serialize(&1, writer)?;
                <T as Serializable>::serialize(v, writer)?;
                Ok(())
            }
            None => {
                <u8 as Serializable>::serialize(&0, writer)?;
                Ok(())
            }
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        match value {
            Some(v) => 1 + T::serialized_size(v),
            None => 1,
        }
    }
}

impl Serializable for String {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        BorshSerialize::serialize(value, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        // Because of weirdness with text encoding it's safest to just
        // serialize the target value and count the bytes
        let mut bytes: Vec<u8> = Vec::new();
        BorshSerialize::serialize(value, &mut bytes).expect("String should be serializable");
        bytes.len()
    }
}

impl Serializable for &str {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        BorshSerialize::serialize(value, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        let mut bytes: Vec<u8> = Vec::new();
        BorshSerialize::serialize(value, &mut bytes).expect("String should be serializable");
        bytes.len()
    }
}

impl Serializable for SystemTime {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        let secs_f64 = value
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "SystemTime before UNIX_EPOCH",
                )
            })?
            .as_secs_f64();
        BorshSerialize::serialize(&secs_f64, writer)
    }

    #[inline(always)]
    fn unversioned_serialized_size(_value: &Self) -> usize {
        8
    }
}

impl Serializable for Duration {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        let secs_f64: f64 = value.as_secs_f64();
        BorshSerialize::serialize(&secs_f64, writer)
    }

    #[inline(always)]
    fn unversioned_serialized_size(_value: &Self) -> usize {
        8
    }
}
