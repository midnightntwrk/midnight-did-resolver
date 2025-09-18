//! Signature scheme for use primarily outside of proofs
//!
//! Schnorr over secp256k1, conforming to BIP340.
use k256::schnorr;
#[cfg(feature = "proptest")]
use proptest::arbitrary::Arbitrary;
use rand::distributions::{Distribution, Standard};
use rand::rngs::OsRng;
use rand::{CryptoRng, Rng};
use serialize::{Deserializable, Serializable, VecExt, Version, Versioned};
#[cfg(all(feature = "proptest", test))]
use serialize::{NetworkId, deserialize, serialize, serialized_size};
#[cfg(feature = "proptest")]
use serialize::{NoStrategy, simple_arbitrary};
use signature::{RandomizedSigner, Verifier};
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::io::{self, Read, Write};
#[cfg(feature = "proptest")]
use std::marker::PhantomData;

macro_rules! derive_via_to_bytes {
    ($ty:ty) => {
        impl Hash for $ty {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                state.write(&self.0.to_bytes()[..]);
            }
        }

        impl PartialOrd for $ty {
            fn partial_cmp(&self, other: &$ty) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $ty {
            fn cmp(&self, other: &$ty) -> Ordering {
                let left = self.0.to_bytes();
                let right = other.0.to_bytes();
                left.cmp(&right)
            }
        }
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A verifying public key
pub struct VerifyingKey(schnorr::VerifyingKey);
derive_via_to_bytes!(VerifyingKey);

#[cfg(feature = "proptest")]
simple_arbitrary!(VerifyingKey);
#[cfg(feature = "proptest")]
serialize::randomised_serialization_test!(VerifyingKey);

impl Distribution<VerifyingKey> for Standard {
    fn sample<R: Rng + ?Sized>(&self, _rng: &mut R) -> VerifyingKey {
        SigningKey::sample(OsRng).verifying_key()
    }
}

impl Versioned for VerifyingKey {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
}

impl Serializable for VerifyingKey {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&value.0.to_bytes())
    }

    fn unversioned_serialized_size(_: &Self) -> usize {
        // Key size is 32 (k256::Secp256k1::FieldBytesSize). Accessing this
        // would require an additional import for the trait.
        // Note that this is *field* size, because BIP340 encodes curve points as a field
        32
    }
}

impl Deserializable for VerifyingKey {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        version: Option<&Version>,
        _recursion_depth: u32,
    ) -> io::Result<Self> {
        match version {
            Some(Version { major: 1, minor: 0 }) => {
                let mut bytes = [0u8; 32];
                reader.read_exact(&mut bytes)?;
                Ok(VerifyingKey(
                    schnorr::VerifyingKey::from_bytes(&bytes).map_err(|_| {
                        Self::deserialization_error(
                            version,
                            "Malformed Schnorr verifying key".to_owned(),
                        )
                    })?,
                ))
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_owned(),
            )),
        }
    }
}

impl VerifyingKey {
    /// Verifies if a signature is correct
    pub fn verify(&self, msg: &[u8], signature: &Signature) -> bool {
        matches!(self.0.verify(msg, &signature.0), Ok(()))
    }
}

#[derive(Clone)]
/// A signing secret key
pub struct SigningKey(schnorr::SigningKey);

impl Versioned for SigningKey {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
}

impl SigningKey {
    /// Samples a new secret key from secure randomness
    pub fn sample<R: Rng + CryptoRng>(mut rng: R) -> Self {
        SigningKey(schnorr::SigningKey::random(&mut rng))
    }

    /// Returns the corresponding verifying public key
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(*self.0.verifying_key())
    }

    /// Signs a message
    pub fn sign<R: Rng + CryptoRng>(&self, rng: &mut R, msg: &[u8]) -> Signature {
        Signature(self.0.sign_with_rng(rng, msg))
    }
}

impl Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<secret key>")
    }
}

impl Serializable for SigningKey {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&value.0.to_bytes())
    }

    fn unversioned_serialized_size(_: &Self) -> usize {
        // Key size is 32 (k256::Secp256k1::FieldBytesSize). Accessing this
        // would require an additional import for the trait.
        32
    }
}

impl Deserializable for SigningKey {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        version: Option<&Version>,
        _recursion_depth: u32,
    ) -> io::Result<Self> {
        match version {
            Some(Version { major: 1, minor: 0 }) => {
                let mut bytes = [0u8; 32];
                reader.read_exact(&mut bytes)?;
                Ok(SigningKey(
                    schnorr::SigningKey::from_bytes(&bytes).map_err(|_| {
                        Self::deserialization_error(
                            version,
                            "Malformed Schnorr secret key".to_owned(),
                        )
                    })?,
                ))
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_owned(),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A Schnorr signature
pub struct Signature(schnorr::Signature);
derive_via_to_bytes!(Signature);

#[cfg(feature = "proptest")]
simple_arbitrary!(Signature);
#[cfg(feature = "proptest")]
serialize::randomised_serialization_test!(Signature);

impl Distribution<Signature> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Signature {
        let signing_key = SigningKey::sample(OsRng);
        let mut message = Vec::with_bounded_capacity(32);
        rng.fill_bytes(&mut message);
        signing_key.sign(&mut OsRng, &message)
    }
}

impl Versioned for Signature {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 0 });
}

impl Serializable for Signature {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&value.0.to_bytes())
    }

    fn unversioned_serialized_size(_: &Self) -> usize {
        schnorr::Signature::BYTE_SIZE
    }
}

impl Deserializable for Signature {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        version: Option<&Version>,
        _recursion_depth: u32,
    ) -> io::Result<Self> {
        match version {
            Some(Version { major: 1, minor: 0 }) => {
                let mut bytes = [0u8; 64];
                reader.read_exact(&mut bytes)?;
                Ok(Signature(
                    schnorr::Signature::try_from(&bytes[..]).map_err(|_| {
                        Self::deserialization_error(
                            version,
                            "Malformed Schnorr signature".to_owned(),
                        )
                    })?,
                ))
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_owned(),
            )),
        }
    }
}
