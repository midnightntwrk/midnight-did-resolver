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
  name = "midnight-did-resolver-checks";
  src = lib.cleanSource ./../..;
  inherit (rustTools) cargoLock postPatch;

  nativeBuildInputs = [
    deadnix
    pkgsInternal.midnight-did-serde-js
  ];

  buildPhase = ''
    cargo b --all-features --all-targets
  '';

  checkPhase = ''
    deadnix -f
    cargo fmt --check
    cargo test
    cargo clippy --all-targets -- -D warnings

    cargo test --all-features
    cargo clippy --all-targets --all-features -- -D warnings

    # check individual feature if properly gated
    echo "checking feature gate for midnight-did"
    cargo clippy -p midnight-did --all-targets --features openapi -- -D warnings

    echo "checking feature gate for midnight-did-serde"
    cargo clippy -p midnight-did-serde --all-targets --features js-cli -- -D warnings
  '';

  installPhase = "touch $out";
}
