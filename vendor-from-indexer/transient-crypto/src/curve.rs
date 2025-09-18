//! Curve selection for Midnight. This may change over time, but we are likely
//! to keep:
//!
//! * A primary prime field [Fr].
//! * Embedded elliptic curve points [EmbeddedGroupAffine].
//! * An Embedded prime field [EmbeddedFr].

use crate::macros::{fr_display, wrap_display, wrap_field_arith, wrap_group_arith};
use base_crypto::fab::{Aligned, Alignment, AlignmentAtom, AlignmentSegment};
#[cfg(any(test, feature = "fake"))]
use fake::{Dummy, Faker};
use ff::{Field, PrimeField};
use group::Group;
use group::GroupEncoding;
use midnight_circuits::ecc::curves::CircuitCurve;
#[cfg(feature = "proptest")]
use proptest::prelude::Arbitrary;
use rand::Rng;
use rand::distributions::Standard;
use rand::prelude::Distribution;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::Error as SerError};
use serialize::{Deserializable, Serializable, Version, Versioned};
#[cfg(all(feature = "proptest", test))]
use serialize::{NetworkId, deserialize, serialize, serialized_size};
#[cfg(feature = "proptest")]
use serialize::{NoStrategy, randomised_serialization_test, simple_arbitrary};
use std::cmp::Ordering;
use std::hash::Hasher;
use std::io::{self, Read, Write};
#[cfg(feature = "proptest")]
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::Mul;
use storage::{
    arena::ArenaKey,
    base_storable,
    db::DB,
    storable::{Loader, Storable},
};

/// The outer, main curve
pub mod outer {
    /// The base prime field, used to represent curve points
    pub type Base = blstrs::Base;
    /// The scalar prime field, used in circuit
    pub type Scalar = blstrs::Scalar;
    /// The affine representation of a curve point
    pub type Affine = blstrs::G1Affine;
}

/// The embedded / cycle curve, used in-circuit mainly
pub mod embedded {
    /// The base prime field, used to represent curve points; the scalar of [`outer`](super::outer)
    pub type Base = blstrs::Scalar;
    /// The scalar prime field, used in embedded proofs
    pub type Scalar = blstrs::Fr;
    /// The affine representation of a curve point over the extended curve
    /// (which contains the relevant cryptographic subgroup).
    pub type AffineExtended = blstrs::JubjubExtended;
    /// The affine representation of a curve point of the relevant cryptographic subgroup.
    pub type Affine = blstrs::JubjubSubgroup;
}

// Since field elements are large, often sparse, and very common, we handle them specially
// for Borsh serialization: We begin with one byte indicating how many bytes are
// required to little-endian encode the field element, and then that many bytes
// of little endian encoding.
//
// For this encoding to be unique, it is an error for the last byte to be zero.
// (Zero itself is represented as the zero length)
//
// In pseudocode:
//
// b = ceil(f.log2() / 8)
// little_endian(f as u<b>)
macro_rules! field_serialize {
    ($name:ident, $wrapped:ty, $to_bytes:ident) => {
        #[cfg(feature = "serde")]
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
                let mut vec = Vec::new();
                <$name as Serializable>::serialize(self, &mut vec).map_err(S::Error::custom)?;
                ser.serialize_bytes(&vec)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
                let bytes = serde_bytes::ByteBuf::deserialize(de)?;
                <$name as Deserializable>::deserialize(&mut &bytes[..], 0)
                    .map_err(serde::de::Error::custom)
            }
        }

        impl Serializable for $name {
            fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> io::Result<()> {
                let repr = value.0.$to_bytes();
                let mut b = 0;
                for (i, bytes) in repr.iter().enumerate() {
                    if *bytes != 0u8 {
                        b = i + 1;
                    }
                }
                writer.write_all(&[b as u8])?;
                writer.write_all(&repr[..b])?;
                Ok(())
            }

            fn unversioned_serialized_size(value: &Self) -> usize {
                let repr = value.0.to_repr();
                let mut b = 0;
                for (i, bytes) in repr.iter().enumerate() {
                    if *bytes != 0u8 {
                        b = i + 1;
                    }
                }
                1 + b
            }
        }

        impl Deserializable for $name {
            fn versioned_deserialize<R: Read>(
                reader: &mut R,
                _version: Option<&Version>,
                _recursion_depth: u32,
            ) -> io::Result<Self> {
                let mut len_buf = [0u8];
                reader.read_exact(&mut len_buf[..])?;
                const MAX_SIZE: usize = FR_BYTES;
                let mut cont_buf = [0u8; MAX_SIZE];
                if len_buf[0] as usize > MAX_SIZE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid length for field: {}", len_buf[0]),
                    ));
                }
                reader.read_exact(&mut cont_buf[..len_buf[0] as usize])?;
                Ok($name(
                    <Option<_>>::from(<$wrapped>::from_repr(cont_buf.into())).ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("out of field bounds"))
                    })?,
                ))
            }
        }
    };
}

/// An element of our primary prime field.
#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Versioned)]
pub struct Fr(pub outer::Scalar);
wrap_field_arith!(Fr);
fr_display!(Fr);
field_serialize!(Fr, outer::Scalar, to_bytes_le);
#[cfg(feature = "proptest")]
randomised_serialization_test!(Fr);
#[cfg(feature = "proptest")]
simple_arbitrary!(Fr);

impl std::hash::Hash for Fr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0.to_bytes_le()[..]);
    }
}

impl Distribution<Fr> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fr {
        Fr(outer::Scalar::random(rng))
    }
}

/// The number of bits required to represet [Fr].
pub const FR_BITS: usize = <outer::Scalar as PrimeField>::NUM_BITS as usize;
/// The number of bytes required to represent [Fr].
pub const FR_BYTES: usize = FR_BITS.div_ceil(8);
/// The number of bytes storable in an [Fr].
pub const FR_BYTES_STORED: usize = FR_BYTES - 1;

#[cfg(any(test, feature = "fake"))]
impl Dummy<Faker> for Fr {
    fn dummy_with_rng<R: Rng + ?Sized>(_: &Faker, rng: &mut R) -> Self {
        rng.gen()
    }
}

/// An element of our embedded prime field.
#[derive(Default, Copy, Clone, Versioned, PartialEq, Eq, PartialOrd, Ord)]
pub struct EmbeddedFr(pub embedded::Scalar);
wrap_field_arith!(EmbeddedFr);
fr_display!(EmbeddedFr);
field_serialize!(EmbeddedFr, embedded::Scalar, to_bytes);
#[cfg(feature = "proptest")]
randomised_serialization_test!(EmbeddedFr);
#[cfg(feature = "proptest")]
simple_arbitrary!(EmbeddedFr);

impl Distribution<EmbeddedFr> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> EmbeddedFr {
        EmbeddedFr(embedded::Scalar::random(rng))
    }
}

#[cfg(any(test, feature = "fake"))]
impl Dummy<Faker> for EmbeddedFr {
    fn dummy_with_rng<R: Rng + ?Sized>(_: &Faker, rng: &mut R) -> Self {
        rng.gen()
    }
}

macro_rules! derive_via {
    ($base:ty, $via:ty, $($ty:ty),*) => {
        $(
        impl From<$ty> for $base {
            fn from(val: $ty) -> $base {
                (val as $via).into()
            }
        }
        )*
    }
}

macro_rules! derive_signed {
    ($base:ty, $($ty:ty, $via:ty),*) => {
        $(
        impl From<$ty> for $base {
            fn from(val: $ty) -> $base {
                if val < 0 {
                    -<$base>::from(val.unsigned_abs())
                } else {
                    (val as $via).into()
                }
            }
        }
        )*
    }
}

impl From<bool> for Fr {
    fn from(val: bool) -> Fr {
        Fr(outer::Scalar::from(u64::from(val)))
    }
}

derive_via!(Fr, u64, u8, u16, u32);
derive_signed!(Fr, i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);
derive_via!(EmbeddedFr, u64, u8, u16, u32);
derive_signed!(EmbeddedFr, i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

impl From<u64> for Fr {
    fn from(val: u64) -> Fr {
        Fr(outer::Scalar::from(val))
    }
}

impl From<u128> for Fr {
    fn from(val: u128) -> Fr {
        Fr(outer::Scalar::from_u128(val))
    }
}

impl Aligned for Fr {
    fn alignment() -> Alignment {
        Alignment::singleton(AlignmentAtom::Field)
    }
}

impl Aligned for EmbeddedFr {
    fn alignment() -> Alignment {
        Alignment::singleton(AlignmentAtom::Field)
    }
}

impl Aligned for EmbeddedGroupAffine {
    fn alignment() -> Alignment {
        Alignment(vec![
            AlignmentSegment::Atom(AlignmentAtom::Field),
            AlignmentSegment::Atom(AlignmentAtom::Field),
        ])
    }
}

impl From<bool> for EmbeddedFr {
    fn from(val: bool) -> EmbeddedFr {
        EmbeddedFr(embedded::Scalar::from(u64::from(val)))
    }
}

impl From<u64> for EmbeddedFr {
    fn from(val: u64) -> EmbeddedFr {
        EmbeddedFr(embedded::Scalar::from(val))
    }
}

impl From<u128> for EmbeddedFr {
    fn from(val: u128) -> EmbeddedFr {
        EmbeddedFr(embedded::Scalar::from_u128(val))
    }
}

impl TryFrom<EmbeddedFr> for Fr {
    type Error = ();
    fn try_from(val: EmbeddedFr) -> Result<Fr, Self::Error> {
        <Option<Fr>>::from(<outer::Scalar>::from_repr(val.0.to_bytes()).map(Fr)).ok_or(())
    }
}

impl TryFrom<Fr> for EmbeddedFr {
    type Error = ();
    fn try_from(val: Fr) -> Result<EmbeddedFr, Self::Error> {
        <Option<EmbeddedFr>>::from(
            <embedded::Scalar>::from_repr(val.0.to_bytes_le()).map(EmbeddedFr),
        )
        .ok_or(())
    }
}

impl Fr {
    /// Interpret a little-endiang bytestring as an [Fr].
    pub fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        let mut repr = [0u8; FR_BYTES];
        if bytes.len() <= repr.len() {
            repr[..bytes.len()].copy_from_slice(bytes)
        } else {
            return None;
        }
        outer::Scalar::from_repr(repr).map(Fr).into()
    }

    /// Output an [Fr] as a little-endian bytesstring
    ///
    /// # Examples
    ///
    /// ```
    /// use midnight_transient_crypto::curve::Fr;
    /// assert_eq!(Fr::from(42), Fr::from_le_bytes(&Fr::from(42).as_le_bytes()).unwrap())
    /// ```
    pub fn as_le_bytes(&self) -> Vec<u8> {
        self.0.to_bytes_le().to_vec()
    }
}

impl EmbeddedFr {
    /// Interpret a little-endiang bytestring as an [EmbeddedFr].
    pub fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        let mut repr = [0u8; FR_BYTES];
        if bytes.len() <= repr.len() {
            repr[..bytes.len()].copy_from_slice(bytes)
        } else {
            return None;
        }
        embedded::Scalar::from_repr(repr).map(EmbeddedFr).into()
    }

    /// Output an [EmbeddedFr] as a little-endian bytesstring
    pub fn as_le_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }
}

/// An element in the embedded elliptic curve.
#[derive(Default, Copy, Clone)]
pub struct EmbeddedGroupAffine(pub embedded::Affine);
wrap_group_arith!(EmbeddedGroupAffine, EmbeddedFr);
wrap_display!(EmbeddedGroupAffine);
#[cfg(feature = "proptest")]
randomised_serialization_test!(EmbeddedGroupAffine);
#[cfg(feature = "proptest")]
simple_arbitrary!(EmbeddedGroupAffine);

impl Distribution<EmbeddedGroupAffine> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> EmbeddedGroupAffine {
        EmbeddedGroupAffine(embedded::Affine::random(rng))
    }
}

impl Mul<Fr> for EmbeddedGroupAffine {
    type Output = EmbeddedGroupAffine;

    fn mul(self, mut rhs: Fr) -> EmbeddedGroupAffine {
        let embedded_m1 = EmbeddedFr::from(0u64) - EmbeddedFr::from(1u64);
        let embedded_modulus = Fr::from_le_bytes(&embedded_m1.as_le_bytes())
            .expect("embedded modulus should fit in scalar field")
            + Fr::from(1);
        while rhs > embedded_modulus {
            rhs = rhs - embedded_modulus;
        }
        self * EmbeddedFr::try_from(rhs).expect("after reducing, rhs should fit in embedded scalar")
    }
}

impl From<embedded::Affine> for EmbeddedGroupAffine {
    fn from(g: embedded::Affine) -> Self {
        Self(g)
    }
}

#[cfg(any(test, feature = "fake"))]
impl Dummy<Faker> for EmbeddedGroupAffine {
    fn dummy_with_rng<R: Rng + ?Sized>(f: &Faker, rng: &mut R) -> Self {
        EmbeddedGroupAffine::generator() * EmbeddedFr::dummy_with_rng(f, rng)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for EmbeddedGroupAffine {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut vec = Vec::new();
        <EmbeddedGroupAffine as Serializable>::serialize(self, &mut vec)
            .map_err(<S::Error as serde::ser::Error>::custom)?;
        ser.serialize_bytes(&vec)
    }
}

impl Serializable for EmbeddedGroupAffine {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        writer.write_all(value.0.to_bytes().as_ref())
    }

    fn unversioned_serialized_size(_value: &Self) -> usize {
        size_of::<<embedded::Affine as GroupEncoding>::Repr>()
    }
}

impl Versioned for EmbeddedGroupAffine {
    const VERSION: Option<Version> = None;
}

impl Deserializable for EmbeddedGroupAffine {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        _recursion_depth: u32,
    ) -> std::io::Result<Self> {
        let mut data = <<embedded::AffineExtended as GroupEncoding>::Repr>::default();
        reader.read_exact(data.as_mut())?;
        <Option<_>>::from(embedded::Affine::from_bytes(&data).map(EmbeddedGroupAffine)).ok_or_else(
            || {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid group element encoding",
                )
            },
        )
    }
}

impl EmbeddedGroupAffine {
    /// Creates a new elliptic curve element from it's affine coordinates. It *is*
    /// checked for validity.
    pub fn new(x: Fr, y: Fr) -> Option<Self> {
        embedded::AffineExtended::from_xy(x.0, y.0).map(|p| EmbeddedGroupAffine(p.into_subgroup()))
    }

    /// Retrieves the curve point's affine `x` coordinate.
    /// Or `None` if this is the identity
    pub fn x(&self) -> Option<Fr> {
        Into::<embedded::AffineExtended>::into(self.0)
            .coordinates()
            .map(|c| Fr(c.0))
    }

    /// Retrieves the curve point's affine `y` coordinate.
    /// Or `None` if this is the identity
    pub fn y(&self) -> Option<Fr> {
        Into::<embedded::AffineExtended>::into(self.0)
            .coordinates()
            .map(|c| Fr(c.1))
    }

    /// Returns the primary generator of the embedded curve.
    pub fn generator() -> Self {
        EmbeddedGroupAffine(embedded::Affine::generator())
    }

    /// Returns the identity element for curve addition.
    pub fn identity() -> Self {
        EmbeddedGroupAffine(embedded::Affine::identity())
    }

    /// Returns if the curve point is the point at infinity.
    pub fn is_infinity(&self) -> bool {
        false
    }

    /// Whether or not this embedded curve has an infinity point in affine representation.
    pub const HAS_INFINITY: bool = true;
}

impl PartialOrd for EmbeddedGroupAffine {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for EmbeddedGroupAffine {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x().hash(state);
        self.y().hash(state);
    }
}

impl PartialEq for EmbeddedGroupAffine {
    fn eq(&self, other: &EmbeddedGroupAffine) -> bool {
        (self.x(), self.y()) == (other.x(), other.y())
    }
}

impl Eq for EmbeddedGroupAffine {}

impl Ord for EmbeddedGroupAffine {
    fn cmp(&self, other: &Self) -> Ordering {
        let a: Option<(embedded::Base, embedded::Base)> =
            Into::<embedded::AffineExtended>::into(self.0).coordinates();
        let b: Option<(embedded::Base, embedded::Base)> =
            Into::<embedded::AffineExtended>::into(other.0).coordinates();
        a.cmp(&b)
    }
}

impl AsRef<outer::Scalar> for Fr {
    fn as_ref(&self) -> &outer::Scalar {
        &self.0
    }
}

impl AsRef<embedded::Scalar> for EmbeddedFr {
    fn as_ref(&self) -> &embedded::Scalar {
        &self.0
    }
}

impl AsRef<embedded::Affine> for EmbeddedGroupAffine {
    fn as_ref(&self) -> &embedded::Affine {
        &self.0
    }
}

macro_rules! impl_smaller_ints {
    ($($ty:ty),* => $via:ty) => {
        $(
            impl TryFrom<Fr> for $ty {
                type Error = ();
                fn try_from(f: Fr) -> Result<$ty, ()> {
                    <$via>::try_from(f)?.try_into().map_err(|_| ())
                }
            }
        )*
    }
}

impl TryFrom<Fr> for u128 {
    type Error = ();
    fn try_from(f: Fr) -> Result<u128, ()> {
        let repr = f.0.to_repr();
        let limbs = repr.as_ref();
        if limbs[16..].iter().any(|limb| limb != &0) {
            Err(())
        } else {
            Ok(limbs[..16].iter().enumerate().fold(0, |acc, (i, byte)| {
                acc + ((*byte as u128) << (8 * i as u128))
            }))
        }
    }
}

impl_smaller_ints!(u8, u16, u32, u64 => u128);

impl TryFrom<Fr> for i128 {
    type Error = ();
    fn try_from(f: Fr) -> Result<i128, ()> {
        let positive_attempt = u128::try_from(f)
            .ok()
            .and_then(|uint| i128::try_from(uint).ok());
        if let Some(pos) = positive_attempt {
            return Ok(pos);
        }
        let negative_attempt = u128::try_from(-f)
            .ok()
            .and_then(|uint| i128::checked_sub_unsigned(0i128, uint));
        if let Some(neg) = negative_attempt {
            return Ok(neg);
        }
        Err(())
    }
}

impl_smaller_ints!(i8, i16, i32, i64 => i128);

impl TryFrom<Fr> for bool {
    type Error = ();
    fn try_from(f: Fr) -> Result<bool, ()> {
        match u64::try_from(f)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(()),
        }
    }
}

base_storable!(Fr);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fr_le_bytes() {
        let val = 0x1234u16;
        let val_le = val.to_le_bytes();
        assert_eq!(val_le, [0x34, 0x12]);
        assert_eq!(Fr::from_le_bytes(&val_le).unwrap(), val.into());
        let restored = Fr::from(val).as_le_bytes();
        assert_eq!(&restored[..2], &val_le);
        assert_eq!(&restored[2..], &[0u8; 30]);
    }

    #[test]
    fn test_identity_point() {
        let id = EmbeddedGroupAffine::identity();
        assert_eq!((id.x(), id.y()), (Some(0.into()), Some(1.into())));
        assert!(!id.is_infinity());
    }

    #[test]
    fn test_identity() {
        let id = EmbeddedGroupAffine::identity();
        assert_eq!(id * Fr::from(42), id);
    }

    #[test]
    fn embedded_fr_within_fr() {
        let embedded_m1 = EmbeddedFr::from(0) - EmbeddedFr::from(1);
        assert!(Fr::try_from(embedded_m1).is_ok());
        let outer_m1 = Fr::from(0) - Fr::from(1);
        assert!(EmbeddedFr::try_from(outer_m1).is_err());
    }

    #[test]
    fn test_embedded_group_borsh() {
        let elem = EmbeddedGroupAffine::identity();
        let mut writer = Vec::new();
        Serializable::serialize(&elem, &mut writer).unwrap();
        assert_eq!(
            elem,
            <EmbeddedGroupAffine as Deserializable>::deserialize(&mut writer.as_slice(), 0)
                .unwrap()
        );
    }
}
