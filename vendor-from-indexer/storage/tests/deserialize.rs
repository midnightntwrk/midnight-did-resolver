#[cfg(test)]
mod tests {
    use midnight_storage::serialize::{
        self as serialize, Deserializable, Serializable, Versioned, deserialize,
        test_file_deserialize,
    };
    #[cfg(feature = "proptest")]
    use midnight_storage::serialize::{
        NetworkId, NoStrategy, randomised_serialization_test, serialize, serialized_size,
        simple_arbitrary,
    };
    use midnight_storage::storage::{Array, HashMap, Map};
    use midnight_storage::{
        Storable,
        arena::ArenaKey,
        db::{DB, InMemoryDB},
        storable::Loader,
    };
    #[cfg(feature = "proptest")]
    use proptest::arbitrary::Arbitrary;
    #[cfg(feature = "proptest")]
    use rand::Rng;
    use rand::distributions::Standard;
    use rand::prelude::*;
    use sha2::Sha256;
    #[cfg(feature = "proptest")]
    use std::marker::PhantomData;

    #[derive(
        Debug, Clone, Hash, PartialOrd, PartialEq, Ord, Eq, Serializable, Deserializable, Versioned,
    )]
    struct TestVec(Vec<bool>);

    impl<D: DB> Storable<D> for TestVec {
        fn children(&self) -> Vec<ArenaKey<D::Hasher>> {
            Vec::new()
        }

        fn to_binary_repr<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error>
        where
            Self: Sized,
        {
            Vec::<bool>::serialize(&self.0, writer)
        }

        fn from_binary_repr<R: std::io::Read>(
            reader: &mut R,
            _child_hashes: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
            _loader: &impl Loader<D>,
        ) -> Result<Self, std::io::Error>
        where
            Self: Sized,
        {
            Ok(Self(Vec::<bool>::deserialize(reader, 0)?))
        }
    }

    #[cfg(feature = "proptest")]
    simple_arbitrary!(TestVec);

    impl Distribution<TestVec> for Standard {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestVec {
            let len = rng.gen_range(0..16);
            TestVec(rng.sample_iter(Standard).take(len).collect())
        }
    }

    #[test]
    fn arena_key_deserialization_error() {
        let bytes: Vec<u8> = vec![0u8; 20];
        assert!(
            deserialize::<ArenaKey<sha2::Sha256>, &[u8]>(
                &bytes[..],
                base_crypto::serialize::NetworkId::Undeployed
            )
            .is_err()
        );
    }

    #[test]
    #[ignore = "storage does not currently support backwards compatibility"]
    fn deserialize_map() {
        // gen_static_serialize_file::<Map<TestVec, u8>>(&thread_rng().gen());
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Map_1_0.bin");
        let _: Map<TestVec, u8, InMemoryDB<Sha256>> = test_file_deserialize(path).unwrap();

        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Map_1_1.bin");
        let _: Map<TestVec, u8, InMemoryDB<Sha256>> = test_file_deserialize(path).unwrap();
    }

    #[test]
    #[ignore = "storage does not currently support backwards compatibility"]
    fn deserialize_array() {
        // gen_static_serialize_file::<Array<u8>>(&thread_rng().gen());
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Array_1_0.bin");
        let _: Array<u8, InMemoryDB<Sha256>> = test_file_deserialize(path).unwrap();

        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Array_1_1.bin");
        let _: Array<u8, InMemoryDB<Sha256>> = test_file_deserialize(path).unwrap();
    }

    #[test]
    #[ignore = "storage does not currently support backwards compatibility"]
    fn deserialze_hashmap() {
        //gen_static_serialize_file::<HashMap<TestVec, u8>>(&thread_rng().gen());
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/HashMap_1_0.bin");
        let _: HashMap<TestVec, u8, InMemoryDB<Sha256>> = test_file_deserialize(path).unwrap();

        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/HashMap_1_1.bin");
        let _: HashMap<TestVec, u8, InMemoryDB<Sha256>> = test_file_deserialize(path).unwrap();
    }

    #[cfg(feature = "proptest")]
    type SimpleArray = Array<u8>;
    #[cfg(feature = "proptest")]
    randomised_serialization_test!(SimpleArray);
    #[cfg(feature = "proptest")]
    type SimpleMap = Map<TestVec, u8>;
    #[cfg(feature = "proptest")]
    randomised_serialization_test!(SimpleMap);
    #[cfg(feature = "proptest")]
    type SimpleHashMap = HashMap<TestVec, u8>;
    #[cfg(feature = "proptest")]
    randomised_serialization_test!(SimpleHashMap);
}
