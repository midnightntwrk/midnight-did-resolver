use derive_where::derive_where;
use midnight_storage_macros::Storable;
use proptest::arbitrary::Arbitrary;
use proptest_derive::Arbitrary;
use rand::distributions::Standard;
use rand::prelude::*;
use std::marker::PhantomData;
use storage::serialize::{self, NoStrategy, Versioned, simple_arbitrary};
use storage::{
    DefaultDB,
    arena::{ArenaKey, BackendLoader, Sp},
    db::DB,
    randomised_storable_test,
    storable::{Loader, Storable},
    storage::default_storage,
};

#[derive(Storable, Arbitrary)]
#[derive_where(Debug, Clone, PartialEq)]
#[storable(db = D)]
struct GenericFoo<D: DB>
where
    Sp<u8, D>: Arbitrary,
{
    #[storable(child)]
    child: Sp<u8, D>,
    data: u8,
}

#[derive(Debug, Storable, Clone, PartialEq, Arbitrary)]
struct Foo {
    #[storable(child)]
    child: Sp<u8>,
    data: u8,
}

#[derive(Debug, Storable, Clone, PartialEq, Arbitrary)]
struct MultiChildFoo {
    #[storable(child)]
    child_a: Sp<u8>,
    #[storable(child)]
    child_b: Sp<u8>,
    #[storable(child)]
    child_c: Sp<u8>,
}

#[derive(Versioned, Debug, Storable, Clone, PartialEq)]
struct RecursiveFoo {
    #[storable(child)]
    child: Sp<Option<Sp<Self>>>,
    in_line_child: Option<Sp<Self>>,
}

impl Distribution<RecursiveFoo> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RecursiveFoo {
        let depth: u8 = rng.gen();
        let arena = default_storage().arena.clone();
        let mut leaf = RecursiveFoo {
            child: arena.alloc(None),
            in_line_child: None,
        };
        for _ in 0..depth {
            leaf = RecursiveFoo {
                child: arena.alloc(Some(arena.alloc(leaf.clone()))),
                in_line_child: Some(arena.alloc(leaf)),
            };
        }

        leaf
    }
}

#[derive(Debug, Storable, Clone, PartialEq, Arbitrary)]
struct ConcatFoo {
    foo: Foo,
    other_foo: Foo,
}

#[derive(Debug, Storable, Clone, PartialEq, Arbitrary)]
#[repr(transparent)]
struct UnnamedInlineFoo(u8);

#[derive(Debug, Storable, Clone, PartialEq, Arbitrary)]
struct UnnamedChildFoo(#[storable(child)] Sp<u8>);

simple_arbitrary!(RecursiveFoo);

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::PartialEq;
    use std::fmt::Debug;

    randomised_storable_test!(Foo);
    randomised_storable_test!(MultiChildFoo);
    type GenericFooDefaultDB = GenericFoo<DefaultDB>;
    randomised_storable_test!(GenericFooDefaultDB);
    randomised_storable_test!(RecursiveFoo);
    randomised_storable_test!(ConcatFoo);
    randomised_storable_test!(UnnamedInlineFoo);
    randomised_storable_test!(UnnamedChildFoo);
}
