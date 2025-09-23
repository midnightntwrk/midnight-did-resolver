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
  inherit (rustTools) cargoLock;

  nativeBuildInputs = [
    deadnix
    pkgsInternal.midnight-did-serde-js
  ];

  preBuild = rustTools.patchScript;

  buildPhase = ''
    cargo build --all-features
  '';

  checkPhase = ''
    deadnix -f
    cargo fmt --check

    # check individual crate and features if properly gated
    echo "checking midnight-did"
    cargo test -p midnight-did --all-features
    cargo build -p midnight-did --all-targets --all-features
    cargo build -p midnight-did --all-targets --features openapi

    echo "checking midnight-did-serde"
    cargo test -p midnight-did-serde --all-features
    cargo build -p midnight-did-serde --all-targets --all-features
    cargo build -p midnight-did-serde --all-targets --features js-cli

    echo "checking midnight-did-indexer-client"
    cargo test -p midnight-did-indexer-client --all-features
    cargo build -p midnight-did-indexer-client --all-targets --all-features

    echo "checking midnight-did-resolver"
    cargo test -p midnight-did-resolver --all-features
    cargo build -p midnight-did-resolver --all-targets --all-features
  '';

  installPhase = "touch $out";
}
