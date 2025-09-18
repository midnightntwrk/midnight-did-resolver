use std::env;
use std::fs::File;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../flake.lock");
    println!("cargo:rerun-if-changed=generate-rust-macros.ss");

    let vendored: String = env::var("CARGO_FEATURE_VENDORED").unwrap_or("0".into());
    let is_vendored = vendored == "1";

    if !is_vendored {
        let macro_out = Path::new(&"./vendored").join("program_fragments.rs");
        assert!(
            Command::new("scheme")
                .arg("--script")
                .arg("generate-rust-macros.ss")
                .stdout(File::create(macro_out).unwrap())
                .status()
                .unwrap()
                .success()
        );
    }
}
