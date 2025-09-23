use crate::base_crypto::fab::{Aligned, AlignedValue, Alignment, AlignmentAtom};
use crate::base_crypto::hash::{HashOutput, persistent_commit};
use crate::base_crypto::repr::MemWrite;
#[cfg(feature = "proptest")]
use crate::serialize::NoStrategy;
use crate::serialize::{self, Deserializable, Serializable, Version, Versioned};
#[cfg(all(feature = "proptest", test))]
use crate::serialize::{NetworkId, deserialize, serialize, serialized_size};
use crate::storage::base_crypto::signatures::VerifyingKey;
use crate::storage::base_storable;
#[cfg(feature = "proptest")]
use crate::storage::serialize::randomised_serialization_test;
use crate::storage::{
    arena::ArenaKey,
    storable::Loader,
    storage::{Array, HashMap, Map},
};
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::merkle_tree::MerkleTree;
use crate::transient_crypto::proofs::VerifierKey;
use crate::transient_crypto::repr::FieldRepr;
#[cfg(feature = "proptest")]
use coin_structure::serialize::simple_arbitrary;
use coin_structure::storage::Storable;
use coin_structure::storage::arena::Sp;
use coin_structure::storage::db::DB;
use derive_where::derive_where;
#[cfg(any(test, feature = "fake"))]
use fake::Dummy;
use hex::ToHex;
#[cfg(feature = "proptest")]
use proptest::arbitrary::Arbitrary;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
use rand::Rng;
use rand::distributions::{Distribution, Standard};
#[cfg(feature = "serde")]
use serde::{
    Deserialize, Deserializer, Serialize, Serializer, de, de::MapAccess, de::SeqAccess,
    de::Visitor, ser::SerializeStruct,
};
use std::borrow::Borrow;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::io::{self, Read, Write};
#[cfg(any(feature = "proptest", feature = "serde"))]
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

#[cfg(feature = "proptest")]
fn proptest_valid<D: DB>(value: &StateValue<D>) -> bool {
    match value {
        StateValue::Array(arr) => arr.len() < 16,
        _ => true,
    }
}

#[derive(Default)]
#[derive_where(Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[cfg_attr(feature = "proptest", proptest(filter = "proptest_valid"))]
#[non_exhaustive]
pub enum StateValue<D: DB> {
    #[default]
    Null,
    // FIXME: We need to figure out how the storage works for Cells in better
    // detail, especially when it if split into multiple loads.
    Cell(Arc<AlignedValue>),
    Map(HashMap<AlignedValue, StateValue<D>, D>),
    Array(Array<StateValue<D>, D>),
    BoundedMerkleTree(
        // The `Serializable::unversioned_serialize` impl requires this.
        #[cfg_attr(
            feature = "proptest",
            proptest(filter = "|mt| !(mt.height() == 0 || mt.height() > 32)")
        )]
        MerkleTree<(), D>,
    ),
}

impl<D: DB> Storable<D> for StateValue<D> {
    fn children(&self) -> Vec<coin_structure::storage::arena::ArenaKey<<D as DB>::Hasher>> {
        match self {
            StateValue::Null | StateValue::Cell(_) => Vec::new(),
            StateValue::Map(m) => m.children(),
            StateValue::Array(a) => a.children(),
            StateValue::BoundedMerkleTree(m) => m.children(),
        }
    }

    fn to_binary_repr<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error>
    where
        Self: Sized,
    {
        match self {
            StateValue::Null => <u8 as Serializable>::serialize(&0, writer),
            StateValue::Cell(c) => {
                <u8 as Serializable>::serialize(&1, writer)?;
                <AlignedValue as Serializable>::serialize(c, writer)
            }
            StateValue::Map(m) => {
                <u8 as Serializable>::serialize(&2, writer)?;
                m.to_binary_repr(writer)
            }
            StateValue::Array(a) => {
                <u8 as Serializable>::serialize(&3, writer)?;
                a.to_binary_repr(writer)
            }
            StateValue::BoundedMerkleTree(b) => {
                <u8 as Serializable>::serialize(&4, writer)?;
                b.to_binary_repr(writer)
            }
        }
    }

    fn from_binary_repr<R: std::io::Read>(
        reader: &mut R,
        child_hashes: &mut impl Iterator<
            Item = coin_structure::storage::arena::ArenaKey<<D as DB>::Hasher>,
        >,
        loader: &impl coin_structure::storage::storable::Loader<D>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        let disc = <u8 as Deserializable>::deserialize(reader, loader.get_recursion_depth())?;
        match disc {
            0 => Ok(StateValue::Null),
            1 => Ok(StateValue::Cell(Arc::new(
                <AlignedValue as Deserializable>::deserialize(
                    reader,
                    loader.get_recursion_depth(),
                )?,
            ))),
            2 => Ok(StateValue::Map(HashMap::from_binary_repr(
                reader,
                child_hashes,
                loader,
            )?)),
            3 => Ok(StateValue::Array(Array::from_binary_repr(
                reader,
                child_hashes,
                loader,
            )?)),
            4 => Ok(StateValue::BoundedMerkleTree(MerkleTree::from_binary_repr(
                reader,
                child_hashes,
                loader,
            )?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unrecognized discriminant: {}.", disc),
            )),
        }
    }
}

// We need to manually implement `Drop` to avoid implicit unbounded recursion, which could lead to
// stack overflows. See https://rust-unofficial.github.io/too-many-lists/first-drop.html.
impl<D: DB> Drop for StateValue<D> {
    fn drop(&mut self) {
        // Early return for non-recursive types. This ensures that we have a base-case for Drop,
        // as we'll end up recursing at least once otherwise, because we keep a queue of state
        // values otherwise.
        match self {
            StateValue::Null | StateValue::Cell(_) | StateValue::BoundedMerkleTree(_) => return,
            StateValue::Map(m) if m.size() == 0 => return,
            StateValue::Array(a) if a.is_empty() => return,
            _ => {}
        }
        // This allows us to escape from the &mut to a owned reference
        let mut frontier = vec![std::mem::take(self)];
        while let Some(mut curr) = frontier.pop() {
            match &mut curr {
                StateValue::Map(m) => {
                    // Map doesn't have `into_iter`, so we clone the contents. But we drop it
                    // first, so that's okay!
                    let values = m.iter().collect::<Vec<_>>();
                    let mut tmp = HashMap::new();
                    std::mem::swap(m, &mut tmp);
                    // This drop will only decrement ref counts, because everything is also backed
                    // in values
                    drop(tmp);
                    // There's now a chance that our values are the only Arc instance left.
                    frontier.extend(
                        values
                            .into_iter()
                            .flat_map(Sp::into_inner)
                            .flat_map(|kv| Sp::into_inner(kv.1)),
                    );
                }
                StateValue::Array(a) => {
                    // Array doesn't have `into_iter`, so we clone the contents. But we drop it
                    // first, so that's okay!
                    let values = a.iter().collect::<Vec<_>>();
                    let mut tmp = Array::new();
                    std::mem::swap(a, &mut tmp);
                    // This drop will only decrement ref counts, because everything is also backed
                    // in values
                    drop(tmp);
                    // There's now a chance that our values are the only Arc instance left.
                    frontier.extend(values.into_iter().flat_map(Sp::into_inner));
                }
                _ => {}
            }
            // It is now safe to drop curr, as this has an empty map/array in it.
            drop(curr);
        }
    }
}

impl<D: DB> Distribution<StateValue<D>> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StateValue<D> {
        let disc = rng.gen_range(0..40);
        match disc {
            20..=36 => StateValue::Cell(Arc::new(rng.gen())),
            37..=38 => {
                let mut mt: MerkleTree<(), D> = rng.gen();
                // The `Serializable::unversioned_serialize` impl requires this.
                while mt.height() == 0 || mt.height() > 32 {
                    mt = rng.gen();
                }
                StateValue::BoundedMerkleTree(mt)
            }
            39 => StateValue::Map(rng.gen()),
            40 => StateValue::Array(rng.gen()),
            _ => StateValue::Null,
        }
    }
}

impl<D: DB> Versioned for StateValue<D> {
    const VERSION: Option<serialize::Version> = Some(Version { major: 2, minor: 2 });
    const LIMIT_RECURSION: bool = true;
    const NETWORK_SPECIFIC: bool = true;
}

impl<D: DB> Serializable for StateValue<D> {
    fn unversioned_serialized_size(value: &Self) -> usize {
        match value {
            StateValue::Null => 1,
            StateValue::Cell(val) => Serializable::serialized_size(&**val),
            StateValue::Array(arr) => 1 + Array::serialized_size(arr),
            StateValue::Map(map) => 1 + Serializable::serialized_size(&map),
            StateValue::BoundedMerkleTree(tree) => {
                let leaves = tree.iter().collect::<Vec<_>>();
                1 + int_size(leaves.len() as u64)
                    + leaves
                        .into_iter()
                        .map(|(i, h)| int_size(i) + Serializable::serialized_size(&h))
                        .sum::<usize>()
            }
        }
    }

    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        match value {
            StateValue::Null => writer.write_all(&[0b1111_0000][..])?,
            StateValue::Cell(val) => Serializable::serialize(&**val, writer)?,
            StateValue::Array(arr) => {
                writer.write_all(&[0b1110_0000 | arr.len() as u8][..])?;
                Serializable::serialize(arr, writer)?;
            }
            StateValue::Map(map) => {
                writer.write_all(&[0b1111_0001][..])?;
                Serializable::serialize(map, writer)?;
            }
            StateValue::BoundedMerkleTree(tree) => {
                let h = tree.height();
                if h == 0 || h > 32 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid Merkle tree height {h}"),
                    ));
                }
                writer.write_all(&[0b1100_0000 | (h - 1)][..])?;
                let leaves = tree.iter().collect::<Vec<_>>();
                write_int(writer, leaves.len() as u64)?;
                for (i, h) in leaves.into_iter() {
                    write_int(writer, i)?;
                    Serializable::serialize(&h, writer)?;
                }
            }
        }

        Ok(())
    }
}

impl<D: DB> Deserializable for StateValue<D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 2, minor: 0 })
            | Some(Version { major: 2, minor: 1 })
            | Some(Version { major: 2, minor: 2 }) => {
                let mut buf = [0u8];
                reader.read_exact(&mut buf[..])?;
                match buf[0] {
                    0b0000_0000..=0b1011_1111 => {
                        let mut pre_reader = buf[..].chain(reader);
                        Ok(StateValue::Cell(Arc::new(
                            <AlignedValue as Deserializable>::deserialize(
                                &mut pre_reader,
                                recursion_depth,
                            )?,
                        )))
                    }
                    0b1100_0000..=0b1101_1111 => {
                        let h = (buf[0] & 0b0001_1111) + 1;
                        let max_idx = (1u64 << h) - 1;
                        let n = read_int(reader)?;
                        let parts = (0..n)
                            .map(|_| {
                                let i = read_int(reader)?;
                                if i > max_idx {
                                    return Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "index exceeded bounds of its Merkle tree",
                                    ));
                                }
                                let val = <HashOutput as Deserializable>::deserialize(
                                    reader,
                                    recursion_depth,
                                )?;
                                Ok((i, val))
                            })
                            .collect::<Result<Vec<_>, io::Error>>()?;
                        if parts.windows(2).all(|win| win[0].0 < win[1].0) {
                            Ok(StateValue::BoundedMerkleTree(
                                parts
                                    .into_iter()
                                    .fold(MerkleTree::blank(h), |tree, (i, val)| {
                                        tree.update_hash(i, val, ())
                                    }),
                            ))
                        } else {
                            Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Merkle tree indicies not in order, or duplicate indicies",
                            ))
                        }
                    }
                    0b1110_0000..=0b1110_1111 => match version {
                        Some(Version { major: 2, minor: 2 }) => Ok(Self::Array(
                            Deserializable::deserialize(reader, recursion_depth)?,
                        )),
                        Some(Version { major: 2, minor: 0 })
                        | Some(Version { major: 2, minor: 1 }) => {
                            Ok(Self::Array(Array::versioned_deserialize(
                                reader,
                                Some(&Version { major: 1, minor: 0 }),
                                recursion_depth,
                            )?))
                        }
                        _ => Err(Self::deserialization_error(
                            version,
                            "Unsupported version.".to_string(),
                        )),
                    },
                    0b1111_0000 => Ok(StateValue::Null),
                    0b1111_0001 => match version {
                        Some(Version { major: 2, minor: 0 }) => Ok(StateValue::Map(
                            Map::<AlignedValue, StateValue<D>, D>::versioned_deserialize(
                                reader,
                                Some(&Version { major: 1, minor: 0 }),
                                recursion_depth,
                            )?
                            .into(),
                        )),
                        Some(Version { major: 2, minor: 1 })
                        | Some(Version { major: 2, minor: 2 }) => Ok(StateValue::Map(
                            Deserializable::deserialize(reader, recursion_depth)?,
                        )),
                        _ => Err(Self::deserialization_error(
                            version,
                            "Unsupported version.".to_string(),
                        )),
                    },
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "reserved state type tag",
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

impl<D: DB> FieldRepr for StateValue<D> {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        use StateValue::*;
        match self {
            Null => writer.write(&[0.into()]),
            Cell(v) => {
                writer.write(&[1.into()]);
                v.field_repr(writer);
            }
            Map(m) => {
                writer.write(&[(2u128 | ((m.size() as u128) << 4)).into()]);
                let mut sorted = m.iter().collect::<Vec<_>>();
                sorted.sort();
                for kv in sorted.into_iter() {
                    kv.0.field_repr(writer);
                    kv.1.field_repr(writer);
                }
            }
            Array(arr) => {
                writer.write(&[(3u64 | ((arr.len() as u64) << 4)).into()]);
                for elem in arr.iter() {
                    elem.field_repr(writer);
                }
            }
            BoundedMerkleTree(t) => {
                let entries = t.iter().collect::<Vec<_>>();
                writer.write(&[(4u128
                    | ((t.height() as u128) << 4)
                    | ((entries.len() as u128) << 12))
                    .into()]);
                for entry in entries.into_iter() {
                    entry.field_repr(writer);
                }
            }
        }
    }

    fn field_size(&self) -> usize {
        use StateValue::*;
        match self {
            Null => 1,
            Cell(v) => 1 + v.field_size(),
            Map(m) => {
                1 + m
                    .iter()
                    .map(|kv| kv.0.field_size() + kv.1.field_size())
                    .sum::<usize>()
            }
            Array(arr) => 1 + arr.iter().map(|s| s.field_size()).sum::<usize>(),
            BoundedMerkleTree(t) => 1 + t.iter().map(|(_, v)| 1 + v.field_size()).sum::<usize>(),
        }
    }
}

#[cfg(feature = "serde")]
impl<D: DB> Serialize for StateValue<D> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            StateValue::Null => {
                let mut ser = ser.serialize_struct("StateValue", 1)?;
                ser.serialize_field("tag", "null")?;
                ser.end()
            }
            StateValue::Cell(val) => {
                let mut ser = ser.serialize_struct("StateValue", 2)?;
                ser.serialize_field("tag", "cell")?;
                ser.serialize_field("content", val)?;
                ser.end()
            }
            StateValue::Map(val) => {
                let mut ser = ser.serialize_struct("StateValue", 2)?;
                ser.serialize_field("tag", "map")?;
                ser.serialize_field("content", val)?;
                ser.end()
            }
            StateValue::Array(val) => {
                let mut ser = ser.serialize_struct("StateValue", 2)?;
                ser.serialize_field("tag", "array")?;
                ser.serialize_field("content", val)?;
                ser.end()
            }
            StateValue::BoundedMerkleTree(val) => {
                let mut ser = ser.serialize_struct("StateValue", 2)?;
                ser.serialize_field("tag", "boundedMerkleTree")?;
                ser.serialize_field("content", val)?;
                ser.end()
            }
        }
    }
}

#[cfg(feature = "serde")]
struct StateValueVisitor<D: DB>(PhantomData<D>);

#[cfg(feature = "serde")]
impl<'de, D: DB> Visitor<'de> for StateValueVisitor<D> {
    type Value = StateValue<D>;
    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a state value")
    }

    fn visit_seq<V: SeqAccess<'de>>(self, mut seq: V) -> Result<StateValue<D>, V::Error> {
        let tag: String = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        match &tag[..] {
            "null" => Ok(StateValue::Null),
            "cell" => Ok(StateValue::Cell(
                seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?,
            )),
            "map" => Ok(StateValue::Map(
                seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?,
            )),
            "array" => Ok(StateValue::Array(
                seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?,
            )),
            "boundedMerkleTree" => Ok(StateValue::BoundedMerkleTree(
                seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?,
            )),
            tag => Err(de::Error::unknown_variant(
                tag,
                &["null", "cell", "map", "array", "boundedMerkleTree"],
            )),
        }
    }

    fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<StateValue<D>, V::Error> {
        let first_key: String = map
            .next_key()?
            .ok_or_else(|| de::Error::missing_field("tag"))?;
        match &first_key[..] {
            "tag" => {
                let tag: String = map.next_value()?;
                fn get_content<'de2, V: MapAccess<'de2>, T: Deserialize<'de2>>(
                    map: &mut V,
                ) -> Result<T, V::Error> {
                    let entry: (String, T) = map
                        .next_entry()?
                        .ok_or_else(|| de::Error::missing_field("content"))?;
                    if &entry.0[..] == "content" {
                        Ok(entry.1)
                    } else {
                        Err(de::Error::unknown_field(&entry.0[..], &["tag", "content"]))
                    }
                }
                match &tag[..] {
                    "null" => Ok(StateValue::Null),
                    "cell" => Ok(StateValue::Cell(get_content(&mut map)?)),
                    "map" => Ok(StateValue::Map(get_content(&mut map)?)),
                    "array" => Ok(StateValue::Array(get_content(&mut map)?)),
                    "boundedMerkleTree" => {
                        Ok(StateValue::BoundedMerkleTree(get_content(&mut map)?))
                    }
                    tag => Err(de::Error::unknown_variant(
                        tag,
                        &["null", "cell", "map", "array", "boundedMerkleTree"],
                    )),
                }
            }
            "content" => Err(de::Error::custom(
                "limitation of current deserialization: StateValue tag must preceed contents",
            )),
            field => Err(de::Error::unknown_field(field, &["tag", "content"])),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, D1: DB> Deserialize<'de> for StateValue<D1> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_struct(
            "StateValue",
            &["tag", "content"],
            StateValueVisitor(PhantomData),
        )
    }
}

#[macro_export]
macro_rules! stval {
    (null) => {
        StateValue::Null
    };
    (($val:expr)) => {
        StateValue::Cell(Arc::new($val.into()))
    };
    ({MT($height:expr) {$($key:expr => $val:expr),*}}) => {
        StateValue::BoundedMerkleTree(MerkleTree::blank($height)$(.update_hash($key, $val, ()))*)
    };
    ({$($key:expr => $val:tt),*}) => {
        StateValue::Map(HashMap::new()$(.insert($key.into(), stval!($val)))*)
    };
    ({$key:expr => $val:tt}; $n:expr) => {
        {
            StateValue::Map((0..$n).into_iter().map(|x|{
                (AlignedValue::from($key + x as u32), stval!($val))
            }).collect())
        }
    };
    ([$($val:tt),*]) => {
        StateValue::Array(Array::new_from_slice(&[$(stval!($val)),*]))
    };
    ([$elem:tt; $n:expr]) => {
        StateValue::Array(Array::new_from_slice(&vec![stval!($elem); $n]))
    };
}

pub use stval;

impl<D: DB> Debug for StateValue<D> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use StateValue::*;
        match self {
            Null => write!(formatter, "null"),
            Cell(v) => write!(formatter, "{v:?}"),
            Map(m) => {
                write!(formatter, "Map ")?;
                formatter
                    .debug_map()
                    .entries(m.iter().map(|kv| (kv.0.clone(), kv.1.clone())))
                    .finish()
            }
            Array(arr) => {
                write!(formatter, "Array({}) ", arr.len())?;
                formatter.debug_list().entries(arr.iter()).finish()
            }
            BoundedMerkleTree(t) => {
                write!(formatter, "MerkleTree({}) ", t.height())?;
                formatter.debug_map().entries(t.iter()).finish()
            }
        }
    }
}

impl<D: DB> StateValue<D> {
    pub fn log_size(&self) -> usize {
        use StateValue::*;
        match self {
            Null => 0,
            Cell(_) => {
                // TODO: This will change!
                1
            }
            Map(m) => (m.size() as u128).next_power_of_two().ilog2() as usize,
            Array(_) => 1,
            BoundedMerkleTree(t) => t.height() as usize,
        }
    }
}

impl<D: DB> From<AlignedValue> for StateValue<D> {
    fn from(val: AlignedValue) -> StateValue<D> {
        StateValue::Cell(Arc::new(val))
    }
}

pub fn write_int<W: Write>(writer: &mut W, int: u64) -> io::Result<()> {
    match int {
        0..=0x7F => writer.write_all(&[int as u8][..]),
        0x80..=0x3FFF => writer.write_all(&[0x80 | (int % 0x80) as u8, (int >> 7) as u8][..]),
        0x4000..=0x1FFFFF => writer.write_all(
            &[
                0x80 | (int % 0x80) as u8,
                0x80 | ((int >> 7) % 0x80) as u8,
                (int >> 14) as u8,
            ][..],
        ),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "too many entries to serialize state value length!",
        )),
    }
}

pub fn int_size(int: u64) -> usize {
    match int {
        0..=0x7F => 1,
        0x80..=0x3FFF => 2,
        0x4000..=0x1FFFFF => 3,
        _ => 4,
    }
}

pub fn read_int<R: Read>(reader: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; 3];
    reader.read_exact(&mut buf[0..1])?;
    if (buf[0] & 0x80) == 0 {
        return Ok(buf[0] as u64);
    }
    reader.read_exact(&mut buf[1..2])?;
    if (buf[1] & 0x80) == 0 {
        return Ok((buf[0] & 0x7f) as u64 | ((buf[1] as u64) << 7));
    }
    reader.read_exact(&mut buf[2..3])?;
    if (buf[2] & 0x80) == 0 {
        Ok((buf[0] & 0x7f) as u64 | (((buf[1] & 0x7f) as u64) << 7) | ((buf[2] as u64) << 14))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "reserved range for deserializing state value length",
        ))
    }
}

enum MaybeStr<'a> {
    Str(&'a str),
    Bytes(&'a [u8]),
}

#[cfg(feature = "serde")]
struct MaybeStrVisitor<T>(PhantomData<T>);

#[cfg(feature = "serde")]
impl<'de, T: From<Vec<u8>>> serde::de::Visitor<'de> for MaybeStrVisitor<T> {
    type Value = T;
    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("[byte]string")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(v.into_bytes().into())
    }

    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        self.visit_byte_buf(v.to_vec())
    }

    fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
        Ok(v.into())
    }
    // Required for serde_json compatibility. See
    // https://github.com/serde-rs/json/pull/557
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut res = Vec::new();
        while let Some(byte) = seq.next_element()? {
            res.push(byte);
        }
        Ok(res.into())
    }
}

fn maybe_str(buf: &[u8]) -> MaybeStr<'_> {
    // For alphanumeric characters, as well as the following: '+-_":/\?#$%^*&.
    // we will use a string as-is. For others, we will use byte enocding.
    // This is to permit arbitrary bytes, while presenting strings to users
    // where sensible.
    fn permitted(c: u8) -> bool {
        c.is_ascii_alphanumeric() || b"'+-_\":/\\?#$^*&.".contains(&c)
    }
    if buf.iter().copied().all(permitted) {
        if let Ok(s) = std::str::from_utf8(buf) {
            return MaybeStr::Str(s);
        }
    }
    MaybeStr::Bytes(buf)
}

macro_rules! idty {
    ($refty:ident, $bufty:ident) => {
        pub type $refty<'a> = &'a [u8];

        #[derive(
            FieldRepr,
            Clone,
            Hash,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Versioned,
            Serializable,
            Deserializable,
        )]
        #[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
        #[cfg_attr(feature = "proptest", derive(Arbitrary))]
        pub struct $bufty(pub Vec<u8>);

        #[cfg(feature = "serde")]
        impl Serialize for $bufty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                match maybe_str(&self.0) {
                    MaybeStr::Str(s) => serializer.serialize_str(s),
                    MaybeStr::Bytes(b) => serializer.serialize_bytes(b),
                }
            }
        }

        impl From<Vec<u8>> for $bufty {
            fn from(vec: Vec<u8>) -> $bufty {
                $bufty(vec)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> Deserialize<'de> for $bufty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserializer.deserialize_any(MaybeStrVisitor(PhantomData))
            }
        }

        impl Debug for $bufty {
            fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
                match maybe_str(&self.0) {
                    MaybeStr::Str(s) => formatter.write_str(s),
                    MaybeStr::Bytes(b) => formatter.write_str(&b.encode_hex::<String>()),
                }
            }
        }

        impl Deref for $bufty {
            type Target = [u8];
            fn deref(&self) -> &[u8] {
                &self.0
            }
        }

        impl Borrow<[u8]> for $bufty {
            fn borrow(&self) -> &[u8] {
                &self.0
            }
        }

        impl From<&[u8]> for $bufty {
            fn from(e: &[u8]) -> $bufty {
                $bufty(e.to_owned())
            }
        }
    };
}

idty!(EntryPoint, EntryPointBuf);
base_storable!(EntryPointBuf);
#[cfg(feature = "proptest")]
randomised_serialization_test!(EntryPointBuf);

impl Distribution<EntryPointBuf> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> EntryPointBuf {
        let length = rng.gen_range(0..10);
        EntryPointBuf(
            vec![0; length]
                .iter()
                .map(|_| rng.gen::<u8>())
                .collect::<Vec<u8>>()
                .to_owned(),
        )
    }
}

impl EntryPointBuf {
    pub fn ep_hash(&self) -> HashOutput {
        persistent_commit(
            &self[..],
            HashOutput(*b"midnight:entry-point\0\0\0\0\0\0\0\0\0\0\0\0"),
        )
    }
}

impl Aligned for EntryPointBuf {
    fn alignment() -> Alignment {
        Alignment::singleton(AlignmentAtom::Compress)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serializable)]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct ContractMaintenanceAuthority {
    pub committee: Vec<VerifyingKey>,
    pub threshold: u32,
    pub counter: u32,
}

base_storable!(ContractMaintenanceAuthority);

impl ContractMaintenanceAuthority {
    pub fn new() -> Self {
        ContractMaintenanceAuthority {
            committee: vec![],
            threshold: 1,
            counter: 0,
        }
    }
}

impl Default for ContractMaintenanceAuthority {
    fn default() -> Self {
        Self::new()
    }
}

impl Versioned for ContractMaintenanceAuthority {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
}

impl Deserializable for ContractMaintenanceAuthority {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 1, minor: 0 }) => Ok(ContractMaintenanceAuthority {
                committee: Deserializable::deserialize(reader, recursion_depth)?,
                threshold: Deserializable::deserialize(reader, recursion_depth)?,
                counter: Deserializable::deserialize(reader, recursion_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[derive(Storable)]
#[derive_where(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[storable(db = D)]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct ContractState<D: DB> {
    pub data: StateValue<D>,
    pub operations: HashMap<EntryPointBuf, ContractOperation, D>,
    pub maintenance_authority: ContractMaintenanceAuthority,
}

impl<D: DB> Serializable for ContractState<D> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        <StateValue<D> as Serializable>::serialize(&value.data, writer)?;
        <HashMap<EntryPointBuf, ContractOperation, D> as Serializable>::serialize(
            &value.operations,
            writer,
        )?;
        ContractMaintenanceAuthority::serialize(&value.maintenance_authority, writer)?;
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        StateValue::<D>::serialized_size(&value.data)
            + HashMap::<EntryPointBuf, ContractOperation, D>::serialized_size(&value.operations)
            + ContractMaintenanceAuthority::serialized_size(&value.maintenance_authority)
    }
}

impl<D: DB> FieldRepr for ContractState<D> {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        self.data.field_repr(writer);
        for a in self.operations.iter() {
            a.0.field_repr(writer);
            a.1.field_repr(writer);
        }
    }

    fn field_size(&self) -> usize {
        self.data.field_size()
            + self
                .operations
                .iter()
                .map(|a| a.0.field_size() + a.1.field_size())
                .sum::<usize>()
    }
}

impl<D: DB> Debug for ContractState<D> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "ContractState (")?;
        self.data.fmt(formatter)?;
        self.operations.fmt(formatter)?;
        write!(formatter, "ContractState )")?;
        Ok(())
    }
}

impl<D: DB> Versioned for ContractState<D> {
    const VERSION: Option<Version> = Some(Version { major: 3, minor: 0 });
}

impl<D: DB> Deserializable for ContractState<D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 3, minor: 0 }) => Ok(Self {
                data: <StateValue<D> as Deserializable>::deserialize(reader, recursion_depth)?,
                operations:
                    <HashMap<EntryPointBuf, ContractOperation, D> as Deserializable>::deserialize(
                        reader,
                        recursion_depth,
                    )?,
                maintenance_authority: Deserializable::deserialize(reader, recursion_depth)?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl<D: DB> Serialize for ContractState<D> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut ser = ser.serialize_struct("ContractState", 2)?;
        ser.serialize_field("data", &self.data)?;
        ser.serialize_field(
            "operations",
            &self
                .operations
                .iter()
                .map(|a| (a.0.deref().clone(), a.1.deref().clone()))
                .collect::<std::collections::HashMap<_, _>>(),
        )?;
        ser.end()
    }
}

impl<D: DB> Default for ContractState<D> {
    fn default() -> Self {
        ContractState {
            data: StateValue::Null,
            operations: HashMap::new(),
            maintenance_authority: ContractMaintenanceAuthority::default(),
        }
    }
}

#[derive(Versioned, Serializable, Deserializable, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub struct ContractOperation {
    pub v2: Option<VerifierKey>,
}

base_storable!(ContractOperation);

impl ContractOperation {
    pub fn new(vk: Option<VerifierKey>) -> Self {
        ContractOperation { v2: vk }
    }

    pub fn latest(&self) -> Option<&VerifierKey> {
        self.v2.as_ref()
    }

    pub fn latest_mut(&mut self) -> &mut Option<VerifierKey> {
        &mut self.v2
    }
}

#[cfg(feature = "proptest")]
simple_arbitrary!(ContractOperation);
#[cfg(feature = "proptest")]
randomised_serialization_test!(ContractOperation);

impl Distribution<ContractOperation> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ContractOperation {
        let some: bool = rng.gen();
        if some {
            ContractOperation {
                v2: Some(rng.gen()),
            }
        } else {
            ContractOperation { v2: None }
        }
    }
}

impl FieldRepr for ContractOperation {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        match self.v2 {
            Some(ref vk) => {
                writer.write(&[0x01.into()]);
                let mut bytes: Vec<u8> = Vec::new();
                <VerifierKey as Serializable>::serialize(vk, &mut bytes)
                    .expect("VerifierKey is serializable");
                bytes.field_repr(writer);
            }
            None => writer.write(&[0x00.into()]),
        }
    }

    fn field_size(&self) -> usize {
        match self.v2 {
            Some(ref vk) => {
                let mut bytes: Vec<u8> = Vec::new();
                <VerifierKey as Serializable>::serialize(vk, &mut bytes)
                    .expect("VerifierKey is serializable");
                1 + bytes.into_iter().fold(0, |acc, b| acc + b.field_size())
            }
            None => 1,
        }
    }
}

impl Debug for ContractOperation {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "<verifier key>")
    }
}

#[cfg(any(test, feature = "fake"))]
impl<F> Dummy<F> for ContractOperation {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, _rng: &mut R) -> Self {
        ContractOperation { v2: None }
    }
}

#[cfg(test)]
mod tests {
    use coin_structure::storage::db::InMemoryDB;

    use super::*;

    fn test_compact_int(x: u64) {
        let mut bytes = Vec::new();
        write_int(&mut bytes, x).unwrap();
        let ptr = &mut &bytes[..];
        let y = read_int(ptr).unwrap();
        assert_eq!(x, y);
        assert!(ptr.is_empty());
    }

    #[test]
    fn test_ints() {
        test_compact_int(0x0);
        test_compact_int(0x1);
        test_compact_int(0x42);
        test_compact_int(0x80);
        test_compact_int(0xff);
        test_compact_int(0x100);
        test_compact_int(0x1000);
        test_compact_int(0x10000);
    }

    #[test]
    fn test_nested_drop() {
        let mut sv: StateValue<InMemoryDB> = StateValue::Null;
        for i in 0..12_000 {
            sv = StateValue::Array(Array::new().insert(0, sv));
            //sv = StateValue::Map(default_storage().new_map().insert(0u8.into(), sv));
            if i % 100 == 0 {
                dbg!(i);
            }
        }
        drop(sv);
        println!("drop(sv) finished!");
    }

    fn test_ser<T: Serializable + Deserializable + Eq + Debug>(val: T) {
        dbg!(&val);
        let mut bytes = Vec::new();
        T::serialize(&val, &mut bytes).unwrap();
        assert_eq!(bytes.len(), T::serialized_size(&val));
        let copy = T::deserialize(&mut bytes.as_slice(), 0).unwrap();
        assert_eq!(val, copy);
    }

    #[test]
    fn test_state_ser() {
        test_ser::<StateValue<InMemoryDB>>(stval!((512u64)));
        test_ser::<StateValue<InMemoryDB>>(stval!({ 512u64 => (12u64) }));
        test_ser::<StateValue<InMemoryDB>>(stval!([(512u64)]));
        test_ser::<StateValue<InMemoryDB>>(stval!(null));
        test_ser::<StateValue<InMemoryDB>>(stval!({MT(12) {}}));
    }
}
