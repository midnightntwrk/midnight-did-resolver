{ rust-bin, rust-overlay }:

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
      "identus-apollo-0.5.0" = "sha256-XWzEfonB+lg3VB3I9BvKPsgOi9fa3x5suW32sXNv42Y=";
      "identus-did-core-0.5.0" = "";
      "identus-did-resolver-http-0.5.0" = "";
    };
  };
}
