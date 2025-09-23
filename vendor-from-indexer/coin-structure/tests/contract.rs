#[cfg(test)]
mod tests {
    use midnight_coin_structure::contract::Address;
    use midnight_coin_structure::serialize::test_file_deserialize;

    #[test]
    fn static_deserialize_address() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Address_2_0.bin");
        let _: Address = test_file_deserialize(path).unwrap();
    }
}
