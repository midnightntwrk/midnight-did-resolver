//! Traits for defining new storage mechanisms

use crate::arena::{Arena, ArenaKey, Sp};
use crate::backend::StorageBackend;
use crate::db::{DB, DummyArbitrary, InMemoryDB};
use crate::merkle_patricia_trie::MerklePatriciaTrie;
use crate::serialize::{Deserializable, Serializable, Version, Versioned, check_injected_version};
use crate::storable::Loader;
use crate::{DefaultDB, DefaultHasher, Storable};
#[cfg(feature = "proptest")]
use base_crypto::serialize::NoStrategy;
use crypto::digest::Digest;
use derive_where::derive_where;
use parking_lot::{Mutex, MutexGuard};
#[cfg(feature = "proptest")]
use proptest::arbitrary::Arbitrary;
#[cfg(feature = "proptest")]
use proptest::strategy::{BoxedStrategy, Strategy};
use rand::distributions::{Distribution, Standard};
use sha2::Sha256;
use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, Index};
use std::sync::{Arc, LazyLock};

/// Storage backed by an in-memory hashmap, indexed by Sha256 hashes
pub type InMemoryStorage = Storage<InMemoryDB<Sha256>>;
/// The default size of the storage cache.
///
/// This size is in number of cache objects, not megabytes consumed! This value
/// is not well motivated, and we may want to change it later, or better yet,
/// refactor the backend to track the memory size of the cache, instead of the
/// number of cached objects.
pub const DEFAULT_CACHE_SIZE: usize = 1024 * 1024;

/// A map from key hashes to values
#[derive(Storable)]
#[derive_where(Clone, Eq, PartialEq)]
#[storable(db = D)]
pub struct HashMap<K: Serializable + Storable<D>, V: Storable<D>, D: DB = DefaultDB>(
    #[allow(clippy::type_complexity)] Map<ArenaKey<D::Hasher>, (Sp<K, D>, Sp<V, D>), D>,
);

impl<K: Serializable + Storable<D>, V: Storable<D> + PartialEq, D: DB> Hash for HashMap<K, V, D> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<K: Serializable + Storable<D> + PartialOrd, V: Storable<D> + PartialOrd, D: DB> PartialOrd
    for HashMap<K, V, D>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<K: Serializable + Storable<D> + Ord, V: Storable<D> + Ord, D: DB> Ord for HashMap<K, V, D> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<K: Debug + Serializable + Storable<D>, V: Debug + Storable<D>, D: DB> Debug
    for HashMap<K, V, D>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.iter().map(|kv| (kv.0.clone(), kv.1.clone())))
            .finish()
    }
}

#[cfg(feature = "proptest")]
impl<K: Storable<D> + Debug + Serializable, V: Storable<D> + Debug, D: DB> Arbitrary
    for HashMap<K, V, D>
where
    Standard: Distribution<V> + Distribution<K>,
{
    type Strategy = NoStrategy<HashMap<K, V, D>>;
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        NoStrategy(PhantomData)
    }
}

impl<D: DB, K: Serializable + Storable<D>, V: Storable<D>> Distribution<HashMap<K, V, D>>
    for Standard
where
    Standard: Distribution<V> + Distribution<K>,
{
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> HashMap<K, V, D> {
        let mut map = HashMap::new();
        let size: usize = rng.gen_range(0..8);
        for _ in 0..size {
            map = map.insert(rng.gen(), rng.gen())
        }

        map
    }
}

impl<K: Serializable + Storable<D>, V: Storable<D>, D: DB> Versioned for HashMap<K, V, D> {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 1 });
}

impl<K: Serializable + Storable<D>, V: Storable<D>, D: DB> Serializable for HashMap<K, V, D> {
    fn unversioned_serialized_size(value: &Self) -> usize {
        Map::<ArenaKey<D::Hasher>, (Sp<K, D>, Sp<V, D>), D>::unversioned_serialized_size(&value.0)
    }

    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        Map::<ArenaKey<D::Hasher>, (Sp<K, D>, Sp<V, D>), D>::unversioned_serialize(&value.0, writer)
    }
}

impl<K: Serializable + Storable<D>, V: Storable<D>, D: DB> Deserializable for HashMap<K, V, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        // Compile-time check that we're remembering to inject version information into
        // `Map` correctly
        const _: () = check_injected_version(
            Some(Version { major: 1, minor: 1 }),
            Map::<ArenaKey<DefaultHasher>, u8>::VERSION,
        );

        match version {
            Some(Version { major: 1, minor: 1 }) => Ok(Self(Map::versioned_deserialize(
                reader,
                Some(&Version { major: 1, minor: 1 }),
                recursion_depth,
            )?)),
            Some(Version { major: 1, minor: 0 }) => Ok(Self(Map::versioned_deserialize(
                reader,
                Some(&Version { major: 1, minor: 0 }),
                recursion_depth,
            )?)),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl<K: serde::Serialize + Serializable + Storable<D>, V: serde::Serialize + Storable<D>, D: DB>
    serde::Serialize for HashMap<K, V, D>
{
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_map(
            self.iter()
                .map(|kv| (kv.0.deref().clone(), kv.1.deref().clone())),
        )
    }
}

#[cfg(feature = "serde")]
struct HashMapVisitor<K, V, D>(PhantomData<(K, V, D)>);

#[cfg(feature = "serde")]
impl<
    'de,
    K: serde::Deserialize<'de> + Serializable + Storable<D>,
    V: serde::Deserialize<'de> + Storable<D>,
    D: DB,
> serde::de::Visitor<'de> for HashMapVisitor<K, V, D>
{
    type Value = HashMap<K, V, D>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a hashmap")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<HashMap<K, V, D>, A::Error> {
        std::iter::from_fn(|| seq.next_entry::<K, V>().transpose()).collect()
    }
}

#[cfg(feature = "serde")]
impl<
    'de,
    K: serde::Deserialize<'de> + Serializable + Storable<D1>,
    V: serde::Deserialize<'de> + Storable<D1>,
    D1: DB,
> serde::Deserialize<'de> for HashMap<K, V, D1>
{
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_map(HashMapVisitor(PhantomData))
    }
}

impl<K: Serializable + Storable<D>, V: Storable<D>, D: DB> Default for HashMap<K, V, D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Serializable + Storable<D>, V: Storable<D>, D: DB> HashMap<K, V, D> {
    /// Creates an empty map
    pub fn new() -> Self {
        Self(Map::new())
    }

    fn gen_key(key: &K) -> ArenaKey<D::Hasher> {
        let mut hasher = D::Hasher::default();
        let mut bytes: Vec<u8> = Vec::new();
        K::serialize(key, &mut bytes).expect("HashMap key should be serializable");
        hasher.update(bytes);
        ArenaKey(hasher.finalize())
    }

    /// Insert object value in map, keyed with the hash of object key. Overwrites
    /// any preexisting object under the same key
    pub fn insert(&self, key: K, value: V) -> Self {
        HashMap(self.0.insert(
            Self::gen_key(&key),
            (
                self.0.mpt.0.arena.alloc(key),
                self.0.mpt.0.arena.alloc(value),
            ),
        ))
    }

    /// Get object keyed by the hash of object key
    pub fn get(&self, key: &K) -> Option<Sp<V, D>> {
        self.0.get(&Self::gen_key(key)).map(|(_, v)| v.clone())
    }

    /// Remove object keyed by the hash of object key
    pub fn remove(&self, key: &K) -> Self {
        HashMap(self.0.remove(&Self::gen_key(key)))
    }

    /// Check if the map contains a key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.0.contains_key(&Self::gen_key(key))
    }

    /// Iterate over the key value pairs in the hashmap
    #[allow(clippy::type_complexity)]
    pub fn iter(&self) -> impl Iterator<Item = Sp<(Sp<K, D>, Sp<V, D>), D>> {
        self.0.iter().map(|(_, v)| v)
    }

    /// Number of elements in the map
    pub fn size(&self) -> usize {
        self.0.size()
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<K: Serializable + Storable<D>, V: Storable<D>, D: DB> FromIterator<(K, V)>
    for HashMap<K, V, D>
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        iter.into_iter()
            .fold(HashMap::new(), |map, (k, v)| map.insert(k, v))
    }
}

impl<K: Serializable + Deserializable + Storable<D>, V: Storable<D>, D: DB> From<Map<K, V, D>>
    for HashMap<K, V, D>
{
    fn from(value: Map<K, V, D>) -> Self {
        let mut hashmap = HashMap::new();

        for (k, v) in value.iter() {
            hashmap = hashmap.insert(k, v.deref().clone());
        }

        hashmap
    }
}

/// A 16 element array built from a MerklePatriciaTrie Node::Branch
#[derive_where(Clone; V)]
#[derive(Storable)]
#[storable(db = D)]
pub struct Array<V: Storable<D>, D: DB = DefaultDB>(
    // Array wraps MPT in an Sp to guarantee it only has one child
    #[storable(child)] Sp<MerklePatriciaTrie<V, D>, D>,
);

impl<V: Storable<D>, D: DB> Versioned for Array<V, D> {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 1 });
}

impl<V: Storable<D> + Debug + Deserializable, D: DB> Debug for Array<V, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<V: Storable<D> + Deserializable, D: DB> From<Vec<V>> for Array<V, D> {
    fn from(value: Vec<V>) -> Self {
        let mut array = Array::new();
        for (i, val) in value.iter().enumerate().take(min(value.len(), 16)) {
            array = array.insert(i, val.clone());
        }
        array
    }
}

impl<V: Storable<D> + Deserializable, D: DB> Distribution<Array<V, D>> for Standard
where
    Standard: Distribution<V>,
{
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Array<V, D> {
        let mut array = Array::new();
        for i in 0..16 {
            array = array.insert(i, rng.gen())
        }

        array
    }
}

#[cfg(feature = "proptest")]
impl<V: Debug + Serializable + Deserializable + Storable<D>, D: DB> Arbitrary for Array<V, D>
where
    Standard: Distribution<V>,
{
    type Strategy = NoStrategy<Array<V, D>>;
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        NoStrategy(PhantomData)
    }
}

impl<V: Storable<D> + PartialEq, D: DB> PartialEq for Array<V, D> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<V: Storable<D> + Eq, D: DB> Eq for Array<V, D> {}

impl<V: Storable<D> + PartialOrd, D: DB> PartialOrd for Array<V, D> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<V: Storable<D> + Ord, D: DB> Ord for Array<V, D> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<V: Storable<D>, D: DB> Serializable for Array<V, D> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        MerklePatriciaTrie::unversioned_serialize(&value.0, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        MerklePatriciaTrie::unversioned_serialized_size(&value.0)
    }
}

impl<V: Storable<D>, D: DB> Deserializable for Array<V, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        // Compile-time check that we're remembering to inject version information into
        // `MerklePatriciaTrie` correctly
        const _: () = check_injected_version(
            Some(Version { major: 1, minor: 1 }),
            MerklePatriciaTrie::<u8>::VERSION,
        );

        match version {
            Some(Version { major: 1, minor: 1 }) => {
                Ok(Self(Sp::new(MerklePatriciaTrie::versioned_deserialize(
                    reader,
                    Some(&Version { major: 1, minor: 1 }),
                    recursion_depth,
                )?)))
            }
            Some(Version { major: 1, minor: 0 }) => {
                Ok(Self(Sp::new(MerklePatriciaTrie::versioned_deserialize(
                    reader,
                    Some(&Version { major: 1, minor: 0 }),
                    recursion_depth,
                )?)))
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version".to_string(),
            )),
        }
    }
}

impl<V: Storable<D>, D: DB> Hash for Array<V, D> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.deref().hash(state);
    }
}

impl<V: Storable<D> + Deserializable, D: DB> Index<usize> for Array<V, D> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("expect element to exist")
    }
}

impl<V: Storable<D> + Deserializable, D: DB> Default for Array<V, D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Storable<D> + Deserializable, D: DB> Array<V, D> {
    /// Construct an empty new array
    pub fn new() -> Self {
        Array(Sp::new(MerklePatriciaTrie::new()))
    }

    /// Generates a new [Array] from a value slice
    pub fn new_from_slice(values: &[V]) -> Self {
        if values.len() > 16 {
            panic!(
                "Array can only hold 16 elements: values.len() == {}",
                values.len()
            );
        }
        let mut array = Array::<V, D>::new();
        for (i, v) in values.iter().enumerate() {
            array = array.insert(i, v.clone());
        }
        array
    }

    /// Number of elements in Array
    pub fn len(&self) -> usize {
        self.0.deref().clone().size()
    }

    /// Get element at index
    pub fn get(&self, index: usize) -> Option<&V> {
        if index < self.len() {
            return self.0.lookup(&[index as u8]);
        }

        None
    }

    /// Insert element at index
    pub fn insert(&self, index: usize, value: V) -> Self {
        Array(Sp::new(self.0.insert(&[index as u8], value)))
    }

    /// Iterate over the elements in the array
    pub fn iter(&self) -> ArrayIter<V, D> {
        ArrayIter::new(self)
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(feature = "serde")]
impl<V: Storable<D> + serde::Serialize + Deserializable, D: DB> serde::Serialize for Array<V, D> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_seq(self.iter().map(|v| v.deref().clone()))
    }
}

#[cfg(feature = "serde")]
struct ArrayVisitor<V, D>(PhantomData<(V, D)>);

#[cfg(feature = "serde")]
impl<'de, V: Storable<D> + serde::Deserialize<'de> + Deserializable, D: DB> serde::de::Visitor<'de>
    for ArrayVisitor<V, D>
{
    type Value = Array<V, D>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "an array")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Array<V, D>, A::Error> {
        Ok(Array::<V, D>::from(
            std::iter::from_fn(|| seq.next_element::<V>().transpose())
                .collect::<Result<Vec<V>, A::Error>>()?,
        ))
    }
}

#[cfg(feature = "serde")]
impl<'de, V: Storable<D1> + serde::Deserialize<'de> + Deserializable, D1: DB>
    serde::Deserialize<'de> for Array<V, D1>
{
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_seq(ArrayVisitor(PhantomData))
    }
}

/// An iterator over in_memory::Array
pub struct ArrayIter<'a, V: Storable<D>, D: DB> {
    array: &'a Array<V, D>,
    next_index: u8,
}

impl<'a, V: Storable<D>, D: DB> ArrayIter<'a, V, D> {
    fn new(array: &'a Array<V, D>) -> Self {
        ArrayIter {
            array,
            next_index: 0,
        }
    }
}

impl<V: Storable<D> + Deserializable, D: DB> Iterator for ArrayIter<'_, V, D> {
    type Item = Sp<V, D>;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.next_index as usize) < self.array.len() {
            let result = self.array.0.lookup_sp(&[self.next_index]);
            self.next_index += 1;
            return result;
        }

        None
    }
}

/// A persistently stored map, guaranteeing O(1) clones and logtime
/// modifications.
#[derive_where(PartialEq, Eq, PartialOrd, Ord; V)]
#[derive_where(Hash, Clone)]
pub struct Map<K, V: Storable<D>, D: DB = DefaultDB> {
    mpt: Sp<MerklePatriciaTrie<V, D>, D>,
    key_type: PhantomData<K>,
}

impl<K: Sync + Send + 'static, V: Storable<D>, D: DB> Storable<D> for Map<K, V, D> {
    /// Rather than in-lining the wrapped MPT it is a child such that we know the public Map has
    /// only a single child element (rather than up to 16)
    fn children(&self) -> Vec<ArenaKey<D::Hasher>> {
        vec![Sp::hash(&self.mpt).into()]
    }

    fn to_binary_repr<W: std::io::Write>(&self, _writer: &mut W) -> Result<(), std::io::Error>
    where
        Self: Sized,
    {
        Ok(())
    }

    fn from_binary_repr<R: std::io::Read>(
        _reader: &mut R,
        child_hashes: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
        loader: &impl Loader<D>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            mpt: loader.get_next(child_hashes)?,
            key_type: PhantomData,
        })
    }
}

impl<K, V: Storable<D>, D: DB> Versioned for Map<K, V, D> {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 1 });
}

impl<K: Serializable + Deserializable, V: Storable<D>, D: DB> FromIterator<(K, V)>
    for Map<K, V, D>
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        iter.into_iter()
            .fold(Map::new(), |map, (k, v)| map.insert(k, v))
    }
}

#[cfg(feature = "proptest")]
impl<K: Serializable + Deserializable + Debug, V: Storable<D> + Debug, D: DB> Arbitrary
    for Map<K, V, D>
where
    Standard: Distribution<V> + Distribution<K>,
{
    type Strategy = NoStrategy<Map<K, V, D>>;
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        NoStrategy(PhantomData)
    }
}

impl<K: Serializable + Deserializable, V: Storable<D>, D: DB> Distribution<Map<K, V, D>>
    for Standard
where
    Standard: Distribution<V> + Distribution<K>,
{
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Map<K, V, D> {
        let mut map = Map::new();
        let size: usize = rng.gen_range(0..8);
        for _ in 0..size {
            map = map.insert(rng.gen(), rng.gen())
        }

        map
    }
}

impl<K, V: Storable<D>, D: DB> Serializable for Map<K, V, D> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        MerklePatriciaTrie::<V, D>::unversioned_serialize(&value.mpt, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        MerklePatriciaTrie::<V, D>::unversioned_serialized_size(&value.mpt)
    }
}

impl<K, V: Storable<D>, D: DB> Deserializable for Map<K, V, D> {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        // Compile-time check that we're remembering to inject version information into
        // `MerklePatriciaTrie` correctly
        const _: () = check_injected_version(
            Some(Version { major: 1, minor: 1 }),
            MerklePatriciaTrie::<u8>::VERSION,
        );

        match version {
            Some(Version { major: 1, minor: 1 }) => Ok(Map {
                mpt: Sp::new(MerklePatriciaTrie::<V, D>::versioned_deserialize(
                    reader,
                    Some(&Version { major: 1, minor: 1 }),
                    recursion_depth,
                )?),
                key_type: PhantomData,
            }),
            Some(Version { major: 1, minor: 0 }) => Ok(Map {
                mpt: Sp::new(MerklePatriciaTrie::<V, D>::versioned_deserialize(
                    reader,
                    Some(&Version { major: 1, minor: 0 }),
                    recursion_depth,
                )?),
                key_type: PhantomData,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version".to_string(),
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl<K: serde::Serialize + Serializable + Deserializable, V: Storable<D> + serde::Serialize, D: DB>
    serde::Serialize for Map<K, V, D>
{
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_map(self.iter().map(|kv| (kv.0, kv.1.deref().clone())))
    }
}

#[cfg(feature = "serde")]
struct MapVisitor<K, V, D>(PhantomData<(K, V, D)>);

#[cfg(feature = "serde")]
impl<
    'de,
    K: serde::Deserialize<'de> + Serializable + Deserializable,
    V: Storable<D> + serde::Deserialize<'de>,
    D: DB,
> serde::de::Visitor<'de> for MapVisitor<K, V, D>
{
    type Value = Map<K, V, D>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut seq: A) -> Result<Map<K, V, D>, A::Error> {
        std::iter::from_fn(|| seq.next_entry::<K, V>().transpose()).collect()
    }
}

#[cfg(feature = "serde")]
impl<
    'de,
    K: serde::Deserialize<'de> + Serializable + Deserializable,
    V: serde::Deserialize<'de> + Storable<D1>,
    D1: DB,
> serde::Deserialize<'de> for Map<K, V, D1>
{
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_map(MapVisitor(PhantomData))
    }
}

fn to_nibbles<T: Serializable>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    T::serialize(value, &mut bytes).unwrap();
    let mut nibbles = Vec::new();
    for b in bytes {
        nibbles.push(b & 0x0f);
        nibbles.push((b & 0xf0) >> 4);
    }

    nibbles
}

fn from_nibbles<T: Deserializable>(value: &[u8]) -> std::io::Result<T> {
    if value.iter().any(|v| *v >= 16) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "nibble out of range",
        ));
    }
    let bytes = value
        .chunks(2)
        .map(|nibbles| nibbles[0] | (nibbles[1] << 4))
        .collect::<Vec<u8>>();
    T::deserialize(&mut &bytes[..], 0)
}

impl<K: Serializable + Deserializable, V: Storable<D>, D: DB> Default for Map<K, V, D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Serializable + Deserializable, V: Storable<D>, D: DB> Map<K, V, D> {
    /// Returns an empty map.
    pub fn new() -> Self {
        Self {
            mpt: Sp::new(MerklePatriciaTrie::new()),
            key_type: PhantomData,
        }
    }

    /// Insert a key-value pair into the map. Must be O(log(|self|)).
    pub fn insert(&self, key: K, value: V) -> Self {
        Map {
            mpt: Sp::new(self.mpt.insert(&to_nibbles(&key), value)),
            key_type: self.key_type,
        }
    }

    /// Remove a key from the map. Must be O(log(|self|))
    pub fn remove(&self, key: &K) -> Self {
        Map {
            mpt: Sp::new(self.mpt.remove(&to_nibbles(&key))),
            key_type: self.key_type,
        }
    }

    /// Iterate over the key-value pairs in the map in a determinsitic, but unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = (K, Sp<V, D>)> {
        self.mpt.iter().map(|(p, v)| {
            let key = from_nibbles::<K>(&p).expect("key should be decodable");
            (key, v)
        })
    }

    /// Iterator over the keys in the map in a deterministic, but unspecified order.
    pub fn keys(&self) -> impl Iterator<Item = K> {
        self.iter().map(|(k, _)| k)
    }

    /// Check if the map contains a key. Must be O(log(|self|)).
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + Serializable,
    {
        self.mpt.lookup(&to_nibbles(key)).is_some()
    }

    /// Retrieve the value stored at a key, if applicable. Must be O(log(|self|)).
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + Serializable,
    {
        self.mpt.lookup(&to_nibbles(&key))
    }

    /// Check if the map is empty. Must be O(1).
    pub fn is_empty(&self) -> bool {
        self.mpt.is_empty()
    }

    /// Retrieve the number of key-value pairs in the map. Must be O(1).
    pub fn size(&self) -> usize {
        self.mpt.deref().clone().size()
    }
}

enum Decodable<T> {
    Yes(T),
    No,
}

impl<T, E> From<Result<T, E>> for Decodable<T> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(v) => Decodable::Yes(v),
            Err(_) => Decodable::No,
        }
    }
}

impl<T: Debug> Debug for Decodable<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Decodable::Yes(v) => v.fmt(f),
            Decodable::No => write!(f, "!decode error!"),
        }
    }
}

impl<K: Deserializable + Debug, V: Storable<D> + Debug, D: DB> Debug for Map<K, V, D> {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .debug_map()
            .entries(
                self.mpt
                    .iter()
                    .map(|(k, v)| (Decodable::from(from_nibbles::<K>(&k)), v)),
            )
            .finish()
    }
}

#[derive(Clone, Debug)]
/// A factory for various storage objects
pub struct Storage<D: DB = DefaultDB> {
    /// The inner storage arena
    pub arena: Arena<D>,
}

impl<D: DB> Storage<D> {
    /// Create a new Storage type with given cache size and db.
    ///
    /// If the `cache_size` is zero, then the `StorageBackend` caches will be
    /// unbounded. Otherwise, the read cache will be strictly bounded by
    /// `cache_size`, and the write cache will be truncated to at most that size
    /// on `StorageBackend` flush operations.
    ///
    /// Note: the cache size is in *number* of objects, not number of megabytes
    /// of memory! See [`self::DEFAULT_CACHE_SIZE`] for a default choice.
    pub fn new(cache_size: usize, db: D) -> Self {
        let arena = Arena::<D>::new_from_backend(StorageBackend::new(cache_size, db), vec![0]);
        Self { arena }
    }

    /// Create a new Storage type from an existing Arena
    pub fn new_from_arena(arena: Arena<D>) -> Self {
        Self { arena }
    }
}

impl<D: DB> Deref for Storage<D> {
    type Target = Arena<D>;
    fn deref(&self) -> &Arena<D> {
        &self.arena
    }
}

impl<D: Default + DB> Default for Storage<D> {
    /// Create a new storage with the default cache size.
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_SIZE, D::default())
    }
}

type StorageMap = std::collections::HashMap<TypeId, Arc<dyn Any + Sync + Send>>;

/// Mutable global default `Storage<D>` keyed on DB type `D`.
static STORAGES: LazyLock<Mutex<StorageMap>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Return the shared storage object for DB type `D`, panicking if none is
/// available.
///
/// Use `try_get_default_storage` instead, if you want to be able to recover
/// from a missing default storage. But the intended use of default storage is
/// that you set it with `set_default_storage` during program initialization,
/// and then assume it's set from that point on, and so crashing if it's not set
/// is expected in normal usage, as it indicates an initialization bug.
///
/// # Implicit initialization of `InMemoryDB` backed storage
///
/// When `D = InMemoryDB`, if the default storage is not initialized, then
/// instead of crashing we initialize it implicitly using
/// `InMemoryDB::default`. This is to avoid needing to write boilerplate storage
/// initialization code in tests, and is not expected to be used in production,
/// where other, actually persistent dbs are used to back the storage.
///
/// # Footgun
///
/// The default storage is defined per process, so in particular all threads in
/// a process share the same default storage at each storage type.
///
/// Because `cargo test` runs tests as different threads in the same process,
/// any tests relying on the default storage may interfere with each other. For
/// most tests this probably doesn't matter, but for tests of the storage
/// itself, we need isolation.
///
/// See [`WrappedDB`] for creating disjoint default storages for the same DB
/// type, e.g. for test isolation.
pub fn default_storage<D: DB + Any>() -> Arc<Storage<D>> {
    if let Some(arc) = try_get_default_storage() {
        arc
    } else if TypeId::of::<D>() == TypeId::of::<InMemoryDB>() {
        // Implicit initialization, but only for InMemoryDB backed storage!
        set_default_storage(Storage::<D>::default).unwrap_or_else(|s| s)
    } else {
        panic!(
            "default storage is not set! you probably need to call set_default_storage in your initialization code"
        )
    }
}

/// Return `Some(default storage)` if initialized, and `None` otherwise.
///
/// In normal usage, you should call `default_storage` instead, because an unset
/// default storage is an initialization bug.
pub fn try_get_default_storage<D: DB + Any>() -> Option<Arc<Storage<D>>> {
    let storages = STORAGES.lock();
    try_get_default_storage_locked(&storages)
}

// Factored out `try_get_default_storage` logic, for reuse where the lock is
// already held.
fn try_get_default_storage_locked<D: DB + Any>(
    storages: &MutexGuard<StorageMap>,
) -> Option<Arc<Storage<D>>> {
    storages.get(&TypeId::of::<Storage<D>>()).map(|arc| {
        arc.clone()
            .downcast::<Storage<D>>()
            .expect("impossible: we only insert Storage<D>")
    })
}

/// Attempts to set the shared storage object for a given DB type.
///
/// This function is similar to
/// <https://doc.rust-lang.org/std/sync/struct.OnceLock.html#method.set>, except
/// that it takes a closure instead of a value. The semantics are:
///
/// - if the default storage is already set for `D`, then return `Err(<existing
///   value>)`
///
/// - if the default storage is not already set for `D`, then set it by calling
///   `mk_value` and return `Ok(<value just set>)`
///
/// Note: It is NOT an error when this function returns `Err(...)`, it just
/// means `mk_value` wasn't actually called. Most callers shouldn't care about
/// this distinction, but returning the `Result` allows the distinction to be
/// tracked if it matters. Normal callers are expected to ignore the result if
/// they're setting the default storage in a context where their init code runs
/// in multiple threads, e.g.
///
/// ```ignore
/// let _idontcare = set_default_storage(|| ...);
/// ```
///
/// or call `unwrap` on the result if they expect to be the only caller (since
/// failure will indicate a bug). If the caller wants the resulting storage, and
/// doesn't care where it came from, then they should call
/// `Result::unwrap_or_else(|s| s)` on the result.
pub fn set_default_storage<D: DB + Any>(
    mk_value: impl FnOnce() -> Storage<D>,
) -> Result<Arc<Storage<D>>, Arc<Storage<D>>> {
    let mut storages = STORAGES.lock();
    if let Some(arc) = try_get_default_storage_locked(&storages) {
        Err(arc)
    } else {
        let storage = mk_value();
        let arc = Arc::new(storage);
        storages.insert(TypeId::of::<Storage<D>>(), arc.clone());
        Ok(arc)
    }
}

/// Clears the shared storage object for a given DB type.
///
/// Since default storage is a global resource shared across all threads,
/// calling this function may cause other threads to crash when they
/// subsequently try to look up the default storage. We don't expect this
/// function to be used in production, but we provide it just case. Callers will
/// need to provide their own synchronization, to for example avoid a race where
/// other threads try to access the default storage between calls to this
/// function and `set_default_storage`.
///
/// # Note
///
/// This function is not "unsafe" in the formal Rust sense of causing undefined
/// behavior if called incorrectly. The `unsafe_` prefix is just to help avoid
/// someone calling it without understanding the consequences.
pub fn unsafe_drop_default_storage<D: DB + Any>() {
    STORAGES.lock().remove(&TypeId::of::<Storage<D>>());
}

/// A tagged newtype wrapper for DBs, to support creating disjoint [default
/// storage]([`default_storage`]) DBs of the same type, concurrently.
///
/// Disjoint default storage for the same DB type are needed, for example, when
/// writing tests that need to run in isolation.
///
/// See `self::tests::persist_to_disk` and
/// `self::tests::test_default_storage` for example usage.
#[derive(Clone)]
#[derive_where(Debug; D)]
pub struct WrappedDB<D: DB, T> {
    db: D,
    tag: PhantomData<T>,
}

impl<D: DB, T> WrappedDB<D, T> {
    /// Create a new `WrappedDB` from a `DB`.
    pub fn wrap(db: D) -> Self {
        Self {
            db,
            tag: PhantomData,
        }
    }
}

impl<D: Default + DB, T> Default for WrappedDB<D, T> {
    fn default() -> Self {
        Self {
            db: Default::default(),
            tag: Default::default(),
        }
    }
}

/// A pass-thru implementation of `DB`.
///
/// # Footgun
///
/// If the `DB` trait ever grows another method with a default implementation,
/// we'll need to be sure to add the pass-thru here, to preserve any possibly
/// overriding implementations provided by the wrapped db.
impl<D: DB, T: Sync + Send + 'static> DB for WrappedDB<D, T> {
    type Hasher = D::Hasher;

    fn get_node(
        &self,
        key: &ArenaKey<Self::Hasher>,
    ) -> Option<crate::backend::OnDiskObject<Self::Hasher>> {
        self.db.get_node(key)
    }

    fn get_unreachable_keys(&self) -> Vec<ArenaKey<Self::Hasher>> {
        self.db.get_unreachable_keys()
    }

    fn insert_node(
        &mut self,
        key: ArenaKey<Self::Hasher>,
        object: crate::backend::OnDiskObject<Self::Hasher>,
    ) {
        self.db.insert_node(key, object)
    }

    fn delete_node(&mut self, key: &ArenaKey<Self::Hasher>) {
        self.db.delete_node(key)
    }

    fn get_root_count(&self, key: &ArenaKey<Self::Hasher>) -> u32 {
        self.db.get_root_count(key)
    }

    fn set_root_count(&mut self, key: ArenaKey<Self::Hasher>, count: u32) {
        self.db.set_root_count(key, count)
    }

    fn get_roots(&self) -> std::collections::HashMap<ArenaKey<Self::Hasher>, u32> {
        self.db.get_roots()
    }

    fn size(&self) -> usize {
        self.db.size()
    }

    fn batch_update<I>(&mut self, iter: I)
    where
        I: Iterator<Item = (ArenaKey<Self::Hasher>, crate::db::Update<Self::Hasher>)>,
    {
        self.db.batch_update(iter)
    }

    fn batch_get_nodes<I>(
        &self,
        keys: I,
    ) -> Vec<(
        ArenaKey<Self::Hasher>,
        Option<crate::backend::OnDiskObject<Self::Hasher>>,
    )>
    where
        I: Iterator<Item = ArenaKey<Self::Hasher>>,
    {
        self.db.batch_get_nodes(keys)
    }

    fn bfs_get_nodes<C>(
        &self,
        key: &ArenaKey<Self::Hasher>,
        cache_get: C,
        truncate: bool,
        max_depth: Option<usize>,
        max_count: Option<usize>,
    ) -> Vec<(
        ArenaKey<Self::Hasher>,
        crate::backend::OnDiskObject<Self::Hasher>,
    )>
    where
        C: Fn(&ArenaKey<Self::Hasher>) -> Option<crate::backend::OnDiskObject<Self::Hasher>>,
    {
        self.db
            .bfs_get_nodes(key, cache_get, truncate, max_depth, max_count)
    }
}

#[cfg(feature = "proptest")]
/// A pass-thru implementation for `Arbitrary`.
impl<D: DB + Arbitrary, T> Arbitrary for WrappedDB<D, T> {
    type Parameters = D::Parameters;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(params: Self::Parameters) -> Self::Strategy {
        D::arbitrary_with(params)
            .prop_map(|db| WrappedDB {
                db,
                tag: PhantomData,
            })
            .boxed()
    }
}

impl<D: DB + DummyArbitrary, T> DummyArbitrary for WrappedDB<D, T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter_map() {
        let mut map = Map::<_, _>::new();
        map = map.insert(1, 4);
        map = map.insert(2, 5);
        map = map.insert(3, 6);
        for (k, v) in map.iter() {
            match (k, Sp::deref(&v)) {
                (1, 4) | (2, 5) | (3, 6) => {}
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn array_test() {
        let mut array = Array::<_>::new_from_slice(&[0, 1, 2, 3]);
        array = array.insert(4, 4);
        assert_eq!(array[0], 0);
        assert_eq!(array[1], 1);
        assert_eq!(array[2], 2);
        assert_eq!(array[3], 3);
        assert_eq!(array[4], 4);
    }

    #[test]
    fn test_map_iterators() {
        let map = Map::<_, _>::new()
            .insert(40026u64, 12u64)
            .insert(12u64, 40026u64);
        let mut keys = map.keys().collect::<Vec<_>>();
        keys.sort();
        assert_eq!(keys, vec![12u64, 40026u64]);
        let mut entries = map
            .iter()
            .map(|(k, v)| (k, *(v.deref())))
            .collect::<Vec<_>>();
        entries.sort();
        assert_eq!(entries, vec![(12u64, 40026u64), (40026u64, 12u64)]);
    }

    #[test]
    fn test_hashmap() {
        let mut hashmap = HashMap::<_, _>::new()
            .insert(40026u64, 12u64)
            .insert(12u64, 40026u64);

        assert_eq!(hashmap.get(&40026u64).map(|sp| *(sp.deref())), Some(12u64));
        assert_eq!(hashmap.get(&12u64).map(|sp| *(sp.deref())), Some(40026u64));
        hashmap = hashmap.remove(&12u64);
        assert_eq!(hashmap.get(&12u64), None);
    }

    /// Test default storage APIs, including using `WrappedDB` for isolation.
    #[test]
    fn test_default_storage() {
        // Create isolated storage types for `DefaultDB`.
        struct Tag1;
        type D1 = WrappedDB<DefaultDB, Tag1>;
        struct Tag2;
        type D2 = WrappedDB<DefaultDB, Tag2>;

        // Check that implicitly creating default storage of type InMemoryDB (if
        // necessary) works, by requesting it. Since in theory some other test
        // thread could have explicitly set the InMemoryDB default storage, this
        // test is not accurate. An accurate test could:
        //

        // - hold the STORAGES lock
        // - remove any existing InMemoryDB that was set by implicit usage in another test thread
        // - check that we get a new one implicitly
        // - reinsert the old one, if any
        // - drop the lock
        //
        // But the implicit InMemoryDB is just a hack for testing anyway, so
        // we'll just check that it's set and not worry about how :)
        {
            default_storage::<InMemoryDB>();
            assert!(try_get_default_storage::<InMemoryDB>().is_some());
        }

        // Check that default storages of other db types are not created
        // implicitly.
        assert!(try_get_default_storage::<D1>().is_none());
        let result = std::panic::catch_unwind(|| {
            default_storage::<D1>();
        });
        assert!(result.is_err());

        // Create a default storage of type D1.
        let b1 = set_default_storage::<D1>(Storage::<D1>::default).unwrap();
        let s1 = b1.arena.alloc(42u8);
        assert!(default_storage::<D1>().get::<u8>(&s1.hash()).is_ok());

        // Check that D1 and D2 have disjoint default storages, even tho they're
        // the same underlying database type.
        set_default_storage::<D2>(Storage::<D2>::default).unwrap();
        assert!(default_storage::<D2>().get::<u8>(&s1.hash()).is_err());

        // Drop the D1 default storage and see that we can create a new one.
        unsafe_drop_default_storage::<D1>();
        assert!(try_get_default_storage::<D1>().is_none());
        set_default_storage::<D1>(Storage::<D1>::default).unwrap();
        assert!(default_storage::<D1>().get::<u8>(&s1.hash()).is_err());

        // Check that dropping the default storage for D1 didn't affect existing
        // references.
        assert!(b1.get::<u8>(&s1.hash()).is_ok());
        assert!(default_storage::<D1>().get::<u8>(&s1.hash()).is_err());

        // Check that we can restore the original D1 default storage (unlikely
        // use case ...)
        let s = Arc::into_inner(b1).expect("we should have the only reference");
        unsafe_drop_default_storage::<D1>();
        set_default_storage::<D1>(|| s).unwrap();
        assert!(default_storage::<D1>().get::<u8>(&s1.hash()).is_ok());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn persist_to_disk_sqldb() {
        use crate::db::SqlDB;

        let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        test_persist_to_disk::<SqlDB<DefaultHasher>>(|| SqlDB::exclusive_file(&path));
    }

    #[cfg(feature = "parity-db")]
    #[test]
    fn persist_to_disk_paritydb() {
        use crate::db::ParityDb;

        let path = tempfile::TempDir::new().unwrap().into_path();
        test_persist_to_disk::<ParityDb<DefaultHasher>>(|| ParityDb::open(&path));
    }

    /// Test that persisting objects to disk works:
    ///
    /// - create a first storage backed by a first db
    /// - create an object, persist it, and flush the db
    /// - create a second storage, backed by a second db, pointing to the same
    ///   file as the first db
    /// - reload the object from the second storage and check its correctness
    ///
    /// This incidentally includes a test of `WrappedDB` and
    /// `set_default_storage`.
    ///
    /// This test doesn't make sense for `InMemoryDB`, because that DB doesn't
    /// persist to disk.
    #[cfg(any(feature = "sqlite", feature = "parity-db"))]
    fn test_persist_to_disk<D: DB>(mk_db: impl Fn() -> D) {
        // Create a unique wrapper type for D, to avoid conflicts with
        // other tests running using D.
        struct Tag;
        type W<D> = WrappedDB<D, Tag>;

        // Compute key in a block so that everything else gets dropped. Need to
        // drop everything to avoid needing non-exclusive access to the DB.
        let key1 = {
            let db1: W<D> = WrappedDB::wrap(mk_db());
            let storage1 = Storage::new(DEFAULT_CACHE_SIZE, db1);
            let storage1 = set_default_storage(|| storage1).unwrap();
            let arena = &storage1.arena;
            let vals1 = [1u8, 1, 2, 3, 5];
            let array1 = Array::<_, W<D>>::new_from_slice(&vals1);
            let sp1 = arena.alloc(array1.clone());
            sp1.persist();
            storage1.with_backend(|backend| backend.flush_all_changes_to_db());
            sp1.hash()
        };
        unsafe_drop_default_storage::<W<D>>();
        std::thread::sleep(std::time::Duration::from_secs(1));

        let db2: W<D> = WrappedDB::wrap(mk_db());
        let storage2 = Storage::new(DEFAULT_CACHE_SIZE, db2);
        let storage2 = set_default_storage(|| storage2).unwrap();
        let array1 = storage2.arena.get::<Array<_, _>>(&key1).unwrap();
        let vals2 = [1u8, 1, 2, 3, 5];
        let array2 = Array::<_, W<D>>::new_from_slice(&vals2);
        assert_eq!(*array1, array2);
    }
}
