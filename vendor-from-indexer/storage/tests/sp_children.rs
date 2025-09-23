use arena::{ArenaKey, Sp};
use db::DB;
use derive_where::derive_where;
use midnight_storage::*;
use serialize::{Serializable, Versioned};
use std::io::{Read, Write};
use storable::Loader;

#[derive(Versioned, Clone, Debug)]
#[derive_where(Hash, Eq, PartialEq, PartialOrd, Ord)]
struct Foo<D: DB>(Sp<u8, D>);

impl<D: DB + Clone> Storable<D> for Foo<D> {
    fn children(&self) -> Vec<ArenaKey<D::Hasher>> {
        vec![self.0.hash().clone().into()]
    }
    fn to_binary_repr<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
    fn from_binary_repr<R: Read>(
        _reader: &mut R,
        children: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
        loader: &impl Loader<D>,
    ) -> std::io::Result<Self> {
        Ok(Foo(loader.get_next(children).unwrap()))
    }
}

#[test]
fn sp_children_test() {
    let storage = Storage::new(16, DefaultDB::default());
    let arena = storage.arena;
    let ptr_u8 = arena.alloc(42);
    let ptr_bar = arena.alloc(Foo(ptr_u8.clone()));
    let mut bytes: Vec<u8> = Vec::new();
    Sp::serialize(&ptr_bar, &mut bytes).unwrap();
    let ptr_bar_prime = arena.deserialize_sp(&mut bytes.as_slice(), 0).unwrap();
    assert_eq!(ptr_bar, ptr_bar_prime);
}
