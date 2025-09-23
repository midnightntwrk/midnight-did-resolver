#[cfg(test)]
mod tests {
    use midnight_coin_structure::coin::{Info, QualifiedInfo, SecretKey, TokenType};
    use midnight_coin_structure::serialize::test_file_deserialize;

    #[test]
    fn test_secret_key_versioned_deserialize() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/SecretKey_2_0.bin");
        let _: SecretKey = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn test_token_type_versioned_deserialize() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/TokenType_2_0.bin");
        let _: TokenType = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn test_info_versioned_deserialize() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/Info_2_0.bin");
        let _: Info = test_file_deserialize(path).unwrap();
    }

    #[test]
    fn test_qualified_info_versioned_deserialize() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/QualifiedInfo_2_0.bin");
        let _: QualifiedInfo = test_file_deserialize(path).unwrap();
    }
}
