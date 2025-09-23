use crate::{Deserializable, Serializable, Version, Versioned};
#[cfg(all(feature = "proptest", test))]
use crate::{deserialize, serialize, serialized_size};
use borsh::{BorshDeserialize, BorshSerialize};
use konst::{const_cmp, option::unwrap};
#[cfg(feature = "proptest")]
use proptest::strategy::ValueTree;
#[cfg(feature = "proptest")]
use proptest::{
    strategy::{NewTree, Strategy},
    test_runner::TestRunner,
};
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
#[cfg(feature = "proptest")]
use rand::Rng;
#[cfg(feature = "proptest")]
use rand::distributions::{Distribution, Standard};
#[cfg(feature = "proptest")]
use std::marker::PhantomData;
use std::{
    cmp::Ordering,
    fmt::Debug,
    io::{BufWriter, Read},
};

#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub enum NetworkId {
    Undeployed = 0,
    DevNet = 1,
    TestNet = 2,
    MainNet = 3,
}

pub trait VecExt {
    fn with_bounded_capacity(n: usize) -> Self;
}

impl<T> VecExt for Vec<T> {
    fn with_bounded_capacity(n: usize) -> Self {
        const MEMORY_LIMIT: usize = 1 << 25; // 32 MiB
        let alloc_limit = MEMORY_LIMIT / std::mem::size_of::<T>();
        Self::with_capacity(usize::min(alloc_limit, n))
    }
}

pub trait ReadExt: Read {
    fn read_exact_to_vec(&mut self, n: usize) -> std::io::Result<Vec<u8>> {
        const CHUNK_SIZE: usize = 4096;
        let mut res = Vec::with_capacity(CHUNK_SIZE);
        let mut len = 0;
        while n > len {
            let new_len = usize::min(n, len + CHUNK_SIZE);
            res.resize(new_len, 0);
            self.read_exact(&mut res[len..])?;
            len = new_len;
        }
        Ok(res)
    }
}

impl<R: Read> ReadExt for R {}

impl Versioned for NetworkId {
    const VERSION: Option<Version> = None;
}

impl Serializable for NetworkId {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        Serializable::unversioned_serialize(&(*value as u8), writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::unversioned_serialized_size(&(*value as u8))
    }
}

impl Deserializable for NetworkId {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        let discriminant = u8::versioned_deserialize(reader, version, recursion_depth)?;
        Ok(match discriminant {
            0 => NetworkId::Undeployed,
            1 => NetworkId::DevNet,
            2 => NetworkId::TestNet,
            3 => NetworkId::MainNet,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown network ID: {discriminant}"),
                ));
            }
        })
    }
}
// macro to implement Serializable and Deserializable for root objects
#[macro_export]
macro_rules! const_serializable {
    ($val:ty, $bytes:expr) => {
        impl Versioned for $val {
            const VERSION: Option<Version> = None;
        }

        impl Serializable for $val
        where
            $val: BorshSerialize,
        {
            #[inline(always)]
            fn unversioned_serialize<W: std::io::Write>(
                value: &Self,
                writer: &mut W,
            ) -> Result<(), std::io::Error> {
                value.serialize(writer)
            }

            #[inline(always)]
            fn unversioned_serialized_size(_value: &Self) -> usize {
                $bytes
            }
        }

        impl Deserializable for $val
        where
            $val: BorshDeserialize,
        {
            #[inline(always)]
            fn versioned_deserialize<R: std::io::Read>(
                reader: &mut R,
                _version: Option<&Version>,
                _recursion_depth: u32,
            ) -> Result<Self, std::io::Error> {
                Self::deserialize_reader(reader)
            }
        }
    };
}

const_serializable!((), 0);
const_serializable!(bool, 1);
const_serializable!(u8, 1);
const_serializable!(u16, 2);
const_serializable!(u32, 4);
const_serializable!(u64, 8);
const_serializable!(u128, 16);
const_serializable!(i8, 1);
const_serializable!(i16, 2);
const_serializable!(i32, 4);
const_serializable!(i64, 8);
const_serializable!(i128, 16);

macro_rules! tuple_serializable {
    (($a:tt, $aidx: tt)$(, ($as:tt, $asidx: tt))*) => {
        impl<$a: Versioned, $($as: Versioned,)*> Versioned for ($a, $($as,)*) {
            const VERSION: Option<Version> = None;
            const NETWORK_SPECIFIC: bool = $a::NETWORK_SPECIFIC $(|| $as::NETWORK_SPECIFIC)*;
        }

        impl<$a: Serializable,$($as: Serializable,)*> Serializable for ($a,$($as,)*) {
            fn unversioned_serialize<W: std::io::Write>(
                value: &Self,
                writer: &mut W,
            ) -> Result<(), std::io::Error> {
                <$a as Serializable>::serialize(&(value.$aidx), writer)?;
                $(<$as as Serializable>::serialize(&(value.$asidx), writer)?;)*
                Ok(())
            }

            fn unversioned_serialized_size(value: &Self) -> usize {
                <$a as Serializable>::serialized_size(&(value.$aidx)) $(+ <$as as Serializable>::serialized_size(&(value.$asidx)))*
            }
        }

        impl<$a: Deserializable,$($as: Deserializable,)*> Deserializable for ($a,$($as,)*) {
            fn versioned_deserialize<R: std::io::Read>(reader: &mut R, _version: Option<&Version>, recursion_depth: u32) -> Result<Self, std::io::Error> {
                Ok((
                <$a as Deserializable>::deserialize(reader, recursion_depth)?,
                $(<$as as Deserializable>::deserialize(reader, recursion_depth)?,)*
                ))
            }
        }
    }
}

tuple_serializable!((A, 0));
tuple_serializable!((A, 0), (B, 1));
tuple_serializable!((A, 0), (B, 1), (C, 2));
tuple_serializable!((A, 0), (B, 1), (C, 2), (D, 3));
tuple_serializable!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
tuple_serializable!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
tuple_serializable!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13),
    (O, 14)
);
tuple_serializable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13),
    (O, 14),
    (P, 15)
);

#[macro_export]
macro_rules! unversioned_deserialize {
    ($type:ty) => {
        impl Deserializable for $type {
            fn versioned_deserialize<R: std::io::Read>(
                reader: &mut R,
                _version: Option<&Version>,
                _recursion_depth: u32,
            ) -> Result<Self, std::io::Error> {
                Self::deserialize_reader(reader)
            }
        }
    };
}

unversioned_deserialize!(String);

pub fn gen_static_serialize_file<T: Serializable>(value: &T) -> Result<(), std::io::Error> {
    let mut file = BufWriter::new(match T::VERSION {
        Some(Version { major, minor }) => std::fs::File::create(format!(
            "{}_{}_{}.bin",
            std::any::type_name::<T>(),
            major,
            minor
        ))?,
        None => std::fs::File::create(format!("{}.bin", std::any::type_name::<T>()))?,
    });
    crate::serialize(&value, &mut file, NetworkId::Undeployed)
}

pub fn test_file_deserialize<T: Serializable + Deserializable>(
    path: std::path::PathBuf,
) -> Result<T, std::io::Error> {
    let bytes = std::fs::read(path)?;
    crate::deserialize(&mut bytes.as_slice(), NetworkId::Undeployed)
}

#[cfg(feature = "proptest")]
pub struct NoSearch<T>(T);

#[cfg(feature = "proptest")]
impl<T: Debug + Clone> ValueTree for NoSearch<T> {
    type Value = T;

    fn current(&self) -> T {
        self.0.clone()
    }

    fn simplify(&mut self) -> bool {
        false
    }

    fn complicate(&mut self) -> bool {
        false
    }
}

#[derive(Debug)]
#[cfg(feature = "proptest")]
pub struct NoStrategy<T>(pub PhantomData<T>);

#[cfg(feature = "proptest")]
impl<T: Debug + Clone> Strategy for NoStrategy<T>
where
    Standard: Distribution<T>,
{
    type Tree = NoSearch<T>;
    type Value = T;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        Ok(NoSearch(runner.rng().gen()))
    }
}

#[macro_export]
#[cfg(feature = "proptest")]
macro_rules! randomised_serialization_test {
    ($type:ty) => {
        #[cfg(test)]
        ::paste::paste! {
            #[allow(non_snake_case)]
            #[test]
            fn [<proptest_deserialize_ $type>]() where $type: proptest::prelude::Arbitrary {
                use rand::Rng;
                let mut runner = proptest::test_runner::TestRunner::default();

                runner.run(&<$type as proptest::prelude::Arbitrary>::arbitrary(), |v| {
                    let mut bytes: Vec<u8> = Vec::new();
                    let network = match rand::thread_rng().gen_range(0..4) {
                        0 => NetworkId::Undeployed,
                        1 => NetworkId::DevNet,
                        2 => NetworkId::TestNet,
                        3 => NetworkId::MainNet,
                        _ => unreachable!(),
                    };
                    serialize(&v, &mut bytes, network).unwrap();
                    let des_result: $type = deserialize(&mut bytes.as_slice(), network).unwrap();
                    assert_eq!(des_result, v);

                    Ok(())
                }).unwrap();
            }

            #[allow(non_snake_case)]
            #[test]
            fn [<proptest_serialized_size_ $type>]() where $type: proptest::prelude::Arbitrary {
                use rand::Rng;
                let mut runner = proptest::test_runner::TestRunner::default();

                runner.run(&<$type as proptest::prelude::Arbitrary>::arbitrary(), |v| {
                    let mut bytes: Vec<u8> = Vec::new();
                    let network = match rand::thread_rng().gen_range(0..4) {
                        0 => NetworkId::Undeployed,
                        1 => NetworkId::DevNet,
                        2 => NetworkId::TestNet,
                        3 => NetworkId::MainNet,
                        _ => unreachable!(),
                    };
                    serialize(&v, &mut bytes, network).unwrap();
                    assert_eq!(bytes.len(), serialized_size(&v));

                    Ok(())
                }).unwrap();
            }

            #[allow(non_snake_case)]
            #[test]
            fn [<proptest_random_data_deserialize_ $type>]() {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                for _ in 0..100 {
                    let size: u8 = rng.gen();
                    let mut bytes: Vec<u8> = Vec::new();
                    for _i in 0..size {
                        bytes.push(rng.gen())
                    }
                    let _ = deserialize::<$type, _>(&mut bytes.as_slice(), NetworkId::Undeployed);
                }
            }
        }
    };
}

/// Produce a single arbitrary value without the ability to simplify or complicate
#[macro_export]
#[cfg(feature = "proptest")]
macro_rules! simple_arbitrary {
    ($type:ty) => {
        impl Arbitrary for $type {
            type Parameters = ();
            type Strategy = NoStrategy<$type>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                NoStrategy(PhantomData)
            }
        }
    };
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(NetworkId);
#[cfg(feature = "proptest")]
randomised_serialization_test!(String);

/// A compile-time check that some injected version information correspons to the
/// most recent version of a type
pub const fn check_injected_version(
    injected_version: Option<Version>,
    type_version: Option<Version>,
) {
    assert!(
        matches!(
            const_cmp!(unwrap!(injected_version), unwrap!(type_version)),
            Ordering::Equal
        ),
        "Maximum supported version not T::VERSION"
    );
}
