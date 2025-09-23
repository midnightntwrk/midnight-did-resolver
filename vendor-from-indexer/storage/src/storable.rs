//! A trait defining a Storable object, which can be assembled into a tree.

#[cfg(feature = "proptest")]
use crate::DefaultDB;
use crate::arena::{ArenaKey, Sp};
use crate::db::DB;
use crate::serialize::{Deserializable, Serializable};
use base_crypto::{
    fab::{AlignedValue, Alignment, Value},
    hash::HashOutput,
};
use crypto::digest::Digest;
#[cfg(feature = "proptest")]
use proptest::{
    prelude::*,
    strategy::{NewTree, ValueTree},
    test_runner::TestRunner,
};
use sha2::Sha256;
use std::fmt::Debug;

/// Supertrait containing all requirements for Hashers
pub trait WellBehavedHasher: Digest + Send + Sync + Default + Debug + Clone + 'static {}

impl WellBehavedHasher for Sha256 {}

/// A loader for objects, for use in [`Storable::from_binary_repr`].
///
/// The intent is to instantiate this in different ways for deserializing
/// objects from the wire, vs deserializing them from the backend.
pub trait Loader<D: DB> {
    /// Get a smart pointer to the object with the given key.
    fn get<T: Storable<D>>(&self, key: &ArenaKey<D::Hasher>) -> Result<Sp<T, D>, std::io::Error>;

    /// Allocate a new object in the arena.
    fn alloc<T: Storable<D>>(&self, obj: T) -> Sp<T, D>;

    /// Get the current recursion depth, for use with
    /// `Deserializable::deserialize`.
    fn get_recursion_depth(&self) -> u32;

    /// Convience function that takes an iterator over `ArenaKey`s, and returns `Sp<T>` keyed by
    /// `iter.next()`.
    fn get_next<T: Storable<D>>(
        &self,
        iter: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
    ) -> Result<Sp<T, D>, std::io::Error> {
        self.get(&iter.next().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "iterator should not yield None".to_string(),
        ))?)
    }
}

/// A Storable object.
///
/// Some methods have `where Self: Sized` to mark them as "explicitly non
/// dispatchable", so that `Storable` can be object safe.
///
/// Assertions:
/// * To maintain an injective relationship between `Arena` and a merkle patricia trie a `Storable`
///   object can have no more than 16 children.
pub trait Storable<D: DB>: Clone + Sync + Send + 'static {
    /// Provides an iterator over hashes of child `Sp`s, if any. These hashes
    /// will be passed back into `from_binary_repr` when deserializing.
    fn children(&self) -> Vec<ArenaKey<D::Hasher>>;

    /// Serializes self, omitting any children.
    fn to_binary_repr<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error>
    where
        Self: Sized;

    /// Instantiates self, given hashes of any children, and loader that loads
    /// children given their hash.
    fn from_binary_repr<R: std::io::Read>(
        reader: &mut R,
        child_hashes: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
        loader: &impl Loader<D>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized;
}

/// Helper function for erroring when an unrecognized discriminant is
/// encountered in implementing `Storable::from_binary_repr`.
fn bad_discriminant_error<A>() -> Result<A, std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Unrecognised discriminant",
    ))
}

#[macro_export]
/// Implements DynStorable and Storable for a type with no children
macro_rules! base_storable {
    ($val:ty) => {
        impl<D: DB> Storable<D> for $val {
            fn children(&self) -> Vec<ArenaKey<D::Hasher>> {
                Vec::new()
            }

            /// Serializes self, ommitting any children
            fn to_binary_repr<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), std::io::Error> {
                <$val as Serializable>::serialize(self, writer)
            }

            fn from_binary_repr<R: std::io::Read>(
                reader: &mut R,
                _child_hashes: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
                loader: &impl Loader<D>,
            ) -> Result<Self, std::io::Error> {
                <$val as Deserializable>::deserialize(reader, loader.get_recursion_depth())
            }
        }
    };
}

base_storable!(());
base_storable!(bool);
base_storable!(u8);
base_storable!(u16);
base_storable!(u32);
base_storable!(u64);
base_storable!(u128);
base_storable!(i8);
base_storable!(i16);
base_storable!(i32);
base_storable!(i64);
base_storable!(i128);
base_storable!(HashOutput);
base_storable!(Value);
base_storable!(Alignment);
base_storable!(AlignedValue);

#[cfg(test)]
// Storable for Vec is inherently unsafe as a Vec can be arbitrarily long whereas `Storable`
// requires that a node has no more than 16 children. However, it is useful for testing.
impl<T: Storable<D>, D: DB> Storable<D> for Vec<Sp<T, D>> {
    fn children(&self) -> Vec<ArenaKey<<D as DB>::Hasher>> {
        self.iter().map(|v| Sp::hash(v).clone().into()).collect()
    }

    fn to_binary_repr<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error>
    where
        Self: Sized,
    {
        u8::serialize(&(self.len() as u8), writer)
    }

    fn from_binary_repr<R: std::io::Read>(
        reader: &mut R,
        child_hashes: &mut impl Iterator<Item = ArenaKey<<D as DB>::Hasher>>,
        loader: &impl Loader<D>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        let len = u8::deserialize(reader, loader.get_recursion_depth())?;

        let mut value = Vec::new();

        for _ in 0..len {
            value.push(loader.get_next(child_hashes)?)
        }

        Ok(value)
    }
}

impl<T: Storable<D>, D: DB> Storable<D> for Option<Sp<T, D>> {
    fn children(&self) -> Vec<ArenaKey<D::Hasher>> {
        self.clone().map_or(vec![], |sp| vec![sp.root.clone()])
    }

    /// Serializes self, ommitting any children
    fn to_binary_repr<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        match self {
            Some(_) => <u8 as Serializable>::serialize(&0, writer),
            None => <u8 as Serializable>::serialize(&1, writer),
        }
    }

    fn from_binary_repr<R: std::io::Read>(
        reader: &mut R,
        child_hashes: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
        loader: &impl Loader<D>,
    ) -> Result<Self, std::io::Error> {
        let dis = <u8 as Deserializable>::deserialize(reader, 0)?;
        match dis {
            0 => {
                let sp = loader.get_next::<T>(child_hashes)?;
                Ok(Some(sp))
            }
            1 => Ok(None),
            _ => bad_discriminant_error(),
        }
    }
}

macro_rules! tuple_storable {
    (($a:tt, $aidx: tt) $(, ($as:tt, $asidx:tt))*) => {
        impl<$a: Storable<D1>,$($as: Storable<D1>,)* D1: DB> Storable<D1> for (Sp<$a, D1>, $(Sp<$as, D1>,)*) {
            fn children(&self) -> Vec<ArenaKey<D1::Hasher>> {
                vec![self.$aidx.hash().clone().into() $(, self.$asidx.hash().clone().into())*]
            }

            /// Serializes self, ommitting any children
            fn to_binary_repr<W: std::io::Write>(&self, _writer: &mut W) -> Result<(), std::io::Error> {
                Ok(())
            }

            fn from_binary_repr<R: std::io::Read>(
                _reader: &mut R,
                child_hashes: &mut impl Iterator<Item = ArenaKey<D1::Hasher>>,
                loader: &impl Loader<D1>,
            ) -> Result<Self, std::io::Error> {
                Ok((loader.get_next::<$a>(child_hashes)?, $(loader.get_next::<$as>(child_hashes)?, )*))
            }
        }
    }
}

tuple_storable!((A, 0));
tuple_storable!((A, 0), (B, 1));
tuple_storable!((A, 0), (B, 1), (C, 2));
tuple_storable!((A, 0), (B, 1), (C, 2), (D, 3));
tuple_storable!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
tuple_storable!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
tuple_storable!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
tuple_storable!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7)
);
tuple_storable!(
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
tuple_storable!(
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
tuple_storable!(
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
tuple_storable!(
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
tuple_storable!(
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
tuple_storable!(
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
tuple_storable!(
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
tuple_storable!(
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
#[cfg(feature = "proptest")]
/// Proptests for asserting storable properties
macro_rules! randomised_storable_test {
    ($type:ty) => {
        #[cfg(test)]
        ::paste::paste! {
            /// Test that `to_binary_repr` followed by `from_binary_repr` is the identity
            /// for argument value.
            #[allow(non_snake_case)]
            #[test]
            fn [<proptest_storable_round_trip_ $type>]() where $type: proptest::prelude::Arbitrary {
                let mut runner = proptest::test_runner::TestRunner::default();

                runner.run(&<$type as proptest::prelude::Arbitrary>::arbitrary(), |v| {
                    let sp_v = default_storage::<DefaultDB>().arena.alloc(v.clone());
                    assert_eq!(&(*sp_v), &v);

                    let mut buf = Vec::new();
                    Storable::<DefaultDB>::to_binary_repr(&v, &mut buf).unwrap();
                    let max_depth = None;
                    let arena = &default_storage().arena.clone();
                    let loader = BackendLoader::new(&arena, max_depth);
                    let v2 = <$type as Storable::<DefaultDB>>::from_binary_repr(&mut buf.as_slice(), &mut sp_v.children().into_iter(), &loader).unwrap();
                    assert_eq!(v, v2);

                    Ok(())
                }).unwrap();
            }
        }
    };
}

#[cfg(feature = "proptest")]
/// A proptest Tree for generating values of `Sp<T>`
pub struct SpTree<T: Storable<DefaultDB>, TT: ValueTree<Value = T>>(TT);

#[cfg(feature = "proptest")]
impl<T: Storable<DefaultDB> + Debug, TT: ValueTree<Value = T>> ValueTree for SpTree<T, TT> {
    type Value = Sp<T>;

    fn current(&self) -> Self::Value {
        crate::storage::default_storage()
            .arena
            .alloc(self.0.current())
    }

    fn simplify(&mut self) -> bool {
        self.0.simplify()
    }

    fn complicate(&mut self) -> bool {
        self.0.complicate()
    }
}

#[cfg(feature = "proptest")]
#[derive(Debug)]
/// A proptest testing strategy for values of `Sp<T>`
pub struct SpStrategy<T: Storable<DefaultDB>, S: Strategy<Value = T>>(S);

#[cfg(feature = "proptest")]
impl<T: Storable<DefaultDB> + Debug, S: Strategy<Value = T>> Strategy for SpStrategy<T, S> {
    type Tree = SpTree<T, S::Tree>;
    type Value = Sp<T>;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        self.0.new_tree(runner).map(|t| SpTree(t))
    }
}

#[cfg(feature = "proptest")]
impl<T: Arbitrary + Storable<DefaultDB>> Arbitrary for Sp<T> {
    type Parameters = T::Parameters;
    type Strategy = SpStrategy<T, T::Strategy>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        SpStrategy(T::arbitrary_with(args))
    }
}

#[cfg(all(feature = "proptest", test))]
mod proptests {
    use super::Storable;
    use crate::arena::{BackendLoader, Sp};
    use crate::serialize::{
        NetworkId, deserialize, randomised_serialization_test, serialize, serialized_size,
    };
    use crate::{DefaultDB, storage::default_storage};

    randomised_storable_test!(u32);
    type SimpleSpOption = Option<Sp<u8>>;
    randomised_storable_test!(SimpleSpOption);
    randomised_serialization_test!(SimpleSpOption);
    type SimpleSpTuple = (Sp<u8>, Sp<u8>);
    randomised_storable_test!(SimpleSpTuple);
    randomised_serialization_test!(SimpleSpTuple);
}
