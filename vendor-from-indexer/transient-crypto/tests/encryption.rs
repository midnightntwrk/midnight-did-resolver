#[cfg(test)]
mod tests {
    use midnight_transient_crypto::curve::Fr;
    use midnight_transient_crypto::encryption::SecretKey;
    use rand::rngs::OsRng;

    #[test]
    fn encryption_test() {
        let sk = SecretKey::new(&mut OsRng);
        let pk = sk.public_key();
        let c = pk.encrypt(&mut OsRng, &Fr::from(42));
        let p: Option<Fr> = sk.decrypt(&c);
        assert_eq!(p, Some(Fr::from(42)));
    }
}
