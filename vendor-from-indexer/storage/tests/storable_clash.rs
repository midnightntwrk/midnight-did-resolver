use arena::Sp;
use midnight_storage::serialize::{Deserializable, Serializable, Versioned};
use midnight_storage::{self as storage, *};
use rayon::prelude::*;
use std::hash::Hash;
use storable::Loader;
use storage::{arena::ArenaKey, db::DB};

#[derive(
    Serializable,
    Deserializable,
    Versioned,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Storable,
)]
struct Foo {
    a: u8,
    b: u64,
}

#[derive(
    Serializable,
    Deserializable,
    Versioned,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Storable,
    Ord,
    Hash,
    Debug,
)]
struct Bar {
    a: u8,
    b: u64,
}

#[test]
fn test_storable_clash() {
    let storage = Storage::new(16, DefaultDB::default());
    let arena = storage.arena;
    let _ptr_a = arena.alloc(Foo { a: 5, b: 42 });
    let _ptr_b = arena.alloc(Bar { a: 5, b: 42 });
}

#[test]
fn test_parallel_clash() {
    enum Either {
        Left(Sp<Foo>),
        Right(Sp<Bar>),
    }
    fn get_a(x: Either) -> u8 {
        match x {
            Either::Left(x) => x.a,
            Either::Right(x) => x.a,
        }
    }
    for _ in 0..100 {
        (0..100)
            .into_par_iter()
            .map(|i| {
                if i % 2 == 0 {
                    Either::Left(Sp::new(Foo { a: 5, b: 42 }))
                } else {
                    Either::Right(Sp::new(Bar { a: 5, b: 42 }))
                }
            })
            .map(get_a)
            .collect::<Vec<_>>();
    }
}
