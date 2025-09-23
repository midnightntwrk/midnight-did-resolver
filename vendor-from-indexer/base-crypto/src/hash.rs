//! Hashing functions for use across Midnight.

use crate::repr::{BinaryHashRepr, MemWrite};
use borsh::{BorshDeserialize, BorshSerialize};
use const_hex::ToHexExt;
#[cfg(any(test, feature = "fake"))]
use fake::Dummy;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use serialize::{Deserializable, Serializable, Version, Versioned, unversioned_deserialize};
#[cfg(all(feature = "proptest", test))]
use serialize::{NetworkId, deserialize, serialize, serialized_size};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug, Display, Formatter};
use std::io;

/// The number of bytes output by [`persistent_hash`].
pub const PERSISTENT_HASH_BYTES: usize = 32;

/// A wrapper around hash outputs.
#[derive(
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    BinaryHashRepr,
    BorshSerialize,
    BorshDeserialize,
    Versioned,
)]
#[cfg_attr(any(test, feature = "fake"), derive(Dummy))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct HashOutput(pub [u8; PERSISTENT_HASH_BYTES]);

#[cfg(feature = "proptest")]
serialize::randomised_serialization_test!(HashOutput);

impl Serializable for HashOutput {
    fn unversioned_serialize<W: io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        BorshSerialize::serialize(&value, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        value.0.len()
    }
}

unversioned_deserialize!(HashOutput);

/// A zeroed [`HashOutput`].
pub const BLANK_HASH: HashOutput = HashOutput([0u8; PERSISTENT_HASH_BYTES]);

impl rand::distributions::Distribution<HashOutput> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> HashOutput {
        HashOutput(rng.gen())
    }
}

impl Debug for HashOutput {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}", self.0.encode_hex())
    }
}

impl Display for HashOutput {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}", &self.0.encode_hex()[..10])
    }
}

/// A hash function that is guaranteed for long-term support.
pub fn persistent_hash(a: &[u8]) -> HashOutput {
    HashOutput(Sha256::digest(a).into())
}

/// Commits to a value using `persistent_hash`.
pub fn persistent_commit<T: BinaryHashRepr + ?Sized>(value: &T, opening: HashOutput) -> HashOutput {
    let mut writer = PersistentHashWriter::new();
    opening.binary_repr(&mut writer);
    value.binary_repr(&mut writer);
    writer.finalize()
}

/// A writer object for building large persistent commitments of data.
pub struct PersistentHashWriter(Sha256);

impl MemWrite<u8> for PersistentHashWriter {
    fn write(&mut self, buf: &[u8]) {
        self.0.update(buf);
    }
}

impl io::Write for PersistentHashWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Default for PersistentHashWriter {
    fn default() -> Self {
        PersistentHashWriter(Sha256::new())
    }
}

impl PersistentHashWriter {
    /// Initializes a black hasher.
    pub fn new() -> Self {
        Default::default()
    }

    /// Finalizes the hasher, and returns the result.
    pub fn finalize(self) -> HashOutput {
        HashOutput(self.0.finalize().into())
    }
}
