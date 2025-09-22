{
  rust-bin,
  rust-overlay,
}:

let
  nightlyVersion = "2025-07-08";
  rustOverrideArgs = {
    extensions = [
      "rust-src"
      "rust-analyzer"
    ];
    targets = [ ];
  };
in
rec {
  rust = mkRust { };

  rustMinimal = mkRust { minimal = true; };

  mkRust =
    {
      minimal ? false,
    }:
    if minimal then
      rust-bin.nightly.${nightlyVersion}.minimal
    else
      rust-bin.nightly.${nightlyVersion}.default.override rustOverrideArgs;

  mkRustCross =
    {
      pkgsCross,
      minimal ? false,
    }:
    let
      rust-bin = rust-overlay.lib.mkRustBin { } pkgsCross.buildPackages;
    in
    if minimal then
      rust-bin.nightly.${nightlyVersion}.minimal
    else
      rust-bin.nightly.${nightlyVersion}.default.override rustOverrideArgs;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "blstrs-0.7.1" = "sha256-nZYcVAghX5F3OJ5F2RRUrRCbBa9j1ICnZye4UHqqun0=";
      "halo2_proofs-0.3.0" = "sha256-4NQVAuHnoZrgEkEWm8m8kSZRiZyLwsMxR7+uRmPuyR4=";
      "identus-apollo-0.5.0" = "sha256-4fRIrQVDVL3h6I25I77e10v6ed9A8KsX9M7y1XO52rg=";
    };
  };

  patchScript = ''
    mkdir -p /build/cargo-vendor-dir/static
    cp ./vendor-from-indexer/static/bls_filecoin_2p14 /build/cargo-vendor-dir/static/bls_filecoin_2p14
    touch ./vendor-from-indexer/midnight-circuits/README.md
  '';
}
