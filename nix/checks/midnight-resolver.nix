{
  lib,
  rustTools,
  makeRustPlatform,
  deadnix,
  pkgsInternal,
}:

let
  inherit (rustTools) rust;
  rustPlatform = makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in
rustPlatform.buildRustPackage {
  name = "resolver-checks";
  src = lib.cleanSource ./../..;
  inherit (rustTools) cargoLock;
  nativeBuildInputs = [
    deadnix
    pkgsInternal.midnight-did-serde-js
  ];
  buildPhase = "cargo b --all-features --all-targets";
  checkPhase = ''
    deadnix -f
    cargo fmt --check
    cargo test
    cargo clippy --all-targets -- -D warnings

    cargo test --all-features
    cargo clippy --all-targets --all-features -- -D warnings

    # check individual feature if properly gated
    echo "checking feature gate for identus-did-midnight"
    cargo clippy -p midnight-did --all-targets --features openapi -- -D warnings

    echo "checking feature gate for identus-did-midnight-sources"
    cargo clippy -p midnight-did-sources --all-targets --features serde-cli -- -D warnings
    cargo clippy -p midnight-did-serde --all-targets --features indexer-api -- -D warnings
  '';
  installPhase = "touch $out";
}
