use crate::util::NetworkId;
use crate::{Version, Versioned};
use borsh::BorshDeserialize;
use std::io::Read;
use std::sync::Arc;
use std::{collections::HashMap, collections::HashSet, hash::Hash};

#[cfg(debug_assertions)]
pub const RECURSION_LIMIT: u32 = 50;
#[cfg(not(debug_assertions))]
pub const RECURSION_LIMIT: u32 = 250;

// Top-level deserialization function
pub fn deserialize<T: Deserializable, R: Read>(
    mut reader: R,
    network_id: NetworkId,
) -> Result<T, std::io::Error> {
    if T::NETWORK_SPECIFIC {
        let recieved_network_id = NetworkId::deserialize(&mut reader, 0)?;
        if recieved_network_id != network_id {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Invalid NETWORK_ID {:?} (expected {:?})",
                    recieved_network_id, network_id
                ),
            ));
        }
    }

    let value = <T as Deserializable>::deserialize(&mut reader, 0)?;

    let count = reader.bytes().count();

    if count == 0 {
        return Ok(value);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("Not all bytes read, {} bytes remaining", count),
    ))
}

pub trait Deserializable
where
    Self: Sized + Versioned,
{
    /// Incrementing recursion depth is handled in Self::deserialize
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error>;

    fn deserialization_error(version: Option<&Version>, info: String) -> std::io::Error {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            std::format!(
                "Invalid input data for {}, recieved version: {:?}, maximum supported version is {:?}. {}",
                std::any::type_name::<Self>(),
                version,
                Self::VERSION,
                info
            ),
        )
    }

    fn get_version<R: Read>(reader: &mut R) -> Result<Option<Version>, std::io::Error> {
        match Self::VERSION {
            Some(..) => {
                let major = u8::deserialize_reader(reader)?;
                let minor = u8::deserialize_reader(reader)?;
                Ok(Some(Version { major, minor }))
            }
            _ => Ok(None),
        }
    }

    #[inline(always)]
    fn deserialize<R: Read>(reader: &mut R, recursion_depth: u32) -> Result<Self, std::io::Error> {
        let version = Self::get_version(reader)?;

        if recursion_depth > RECURSION_LIMIT {
            Err(Self::deserialization_error(
                version.as_ref(),
                "Reached recursion limit.".to_string(),
            ))
        } else {
            Self::versioned_deserialize(
                reader,
                version.as_ref(),
                recursion_depth + (Self::LIMIT_RECURSION as u32),
            )
        }
    }
}

impl Deserializable for Version {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let major = Deserializable::deserialize(reader, recursion_depth)?;
        let minor = Deserializable::deserialize(reader, recursion_depth)?;
        Ok(Version { major, minor })
    }
}

impl<T: Deserializable> Deserializable for Vec<T> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let len = <u32 as Deserializable>::deserialize(reader, recursion_depth)?;
        let mut result = Vec::new();
        for _ in 0..len {
            result.push(<T as Deserializable>::deserialize(reader, recursion_depth)?);
        }
        Ok(result)
    }
}

impl<K: Deserializable + PartialOrd + Hash + Eq, V: Deserializable> Deserializable
    for HashMap<K, V>
{
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let len = <u32 as Deserializable>::deserialize(reader, recursion_depth)?;
        let mut result = HashMap::new();
        for _ in 0..len {
            let k = <K as Deserializable>::deserialize(reader, recursion_depth)?;
            let v = <V as Deserializable>::deserialize(reader, recursion_depth)?;
            result.insert(k, v);
        }
        Ok(result)
    }
}

impl<T: Deserializable + Hash + Eq> Deserializable for HashSet<T> {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let len = <u32 as Deserializable>::deserialize(reader, recursion_depth)?;
        let mut result = HashSet::new();
        for _ in 0..len {
            result.insert(<T as Deserializable>::deserialize(reader, recursion_depth)?);
        }
        Ok(result)
    }
}

impl<T: Deserializable> Deserializable for Option<T> {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        _version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let some = <u8 as Deserializable>::deserialize(reader, recursion_depth)?;
        match some {
            0 => Ok(None),
            1 => Ok(Some(<T as Deserializable>::deserialize(
                reader,
                recursion_depth,
            )?)),
            _ => Err(Self::deserialization_error(
                None,
                format!("Invalid discriminant: {}.", some),
            )),
        }
    }
}

impl<T: Deserializable> Deserializable for Arc<T> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        Ok(Arc::new(T::versioned_deserialize(
            reader,
            version,
            recursion_depth,
        )?))
    }
}
