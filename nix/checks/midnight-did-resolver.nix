{
  lib,
  rustTools,
  makeRustPlatform,
  deadnix,
  jq,
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
    jq
  ];

  preBuild = rustTools.patchScript;

  buildPhase = ''
    cargo build --all-features
  '';

  checkPhase = ''
    deadnix -f
    cargo fmt --check

    # Automatically check all crates and all features
    CRATES=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.source == null and (.name | test("^midnight-did-"))) | .name')
    for CRATE in $CRATES; do
      echo "Checking crate: $CRATE"
      FEATURES=$(cargo metadata --no-deps --format-version 1 | jq -r --arg CRATE "$CRATE" '.packages[] | select(.name == $CRATE) | .features | keys[]')
      echo "  cargo test -p $CRATE --all-features"
      cargo test -p "$CRATE" --all-features
      echo "  cargo build -p $CRATE --all-targets --all-features"
      cargo build -p "$CRATE" --all-targets --all-features
      for FEATURE in $FEATURES; do
        echo "  cargo test -p $CRATE --features $FEATURE"
        cargo test -p "$CRATE" --features "$FEATURE"
        echo "  cargo build -p $CRATE --all-targets --features $FEATURE"
        cargo build -p "$CRATE" --all-targets --features "$FEATURE"
      done
    done
  '';

  installPhase = "touch $out";
}
