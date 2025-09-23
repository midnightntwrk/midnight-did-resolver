use midnight_transient_crypto::merkle_tree::MerkleTree;
use serialize::test_file_deserialize;
#[cfg(all(test, feature = "proptest"))]
use serialize::{
    NetworkId, deserialize, randomised_serialization_test, serialize, serialized_size,
};

#[test]
fn deserialize_merkletree() {
    // gen_static_serialize_file::<MerkleTree<u8>>(&thread_rng().gen());
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/MerkleTree_3_0.bin");
    let _: MerkleTree<()> = test_file_deserialize(path).unwrap();
}

#[cfg(feature = "proptest")]
type SimpleMerkleTree = MerkleTree<()>;
#[cfg(feature = "proptest")]
randomised_serialization_test!(SimpleMerkleTree);
