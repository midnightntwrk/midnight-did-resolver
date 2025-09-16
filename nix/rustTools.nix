{
  rust-bin,
  rust-overlay,
  fetchurl,
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
      "identus-apollo-0.5.0" = "sha256-4fRIrQVDVL3h6I25I77e10v6ed9A8KsX9M7y1XO52rg=";
      "midnight-base-crypto-1.0.0-alpha.1" = "sha256-nVIiIpuXwb1+dLD/U4F2hMXSm4ROPJ79x6FviFd3qpc=";
      "midnight-circuits-4.0.0" = "sha256-29EYVorD4KxR/ZmSqIWsZnjZE36z1F8eZ9budGyKM3A=";
    };
  };

  postPatch =
    let
      bls_filecoin_2p14 = fetchurl {
        url = "https://github.com/midnightntwrk/midnight-ledger/raw/ledger-6.1.0-alpha.2/static/bls_filecoin_2p14";
        sha256 = "SSPlp/u3Fdgc21wDucDiEXaNNczFLYL0nD2TvPjTalY=";
      };
    in
    ''
      mkdir -p /build/cargo-vendor-dir/static
      cp ${bls_filecoin_2p14} /build/cargo-vendor-dir/static/bls_filecoin_2p14
    '';
}
