{
  pkgs,
  lib,
  makeRustPlatform,
  rust,
  cargoLock,
  buildFeatures ? [ ],
}:

let
  rustPlatform = makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in
let
  bls_filecoin_2p14 = pkgs.fetchurl {
    url = "https://github.com/midnightntwrk/midnight-ledger/raw/ledger-6.1.0-alpha.2/static/bls_filecoin_2p14";
    sha256 = "SSPlp/u3Fdgc21wDucDiEXaNNczFLYL0nD2TvPjTalY=";
  };
in
rustPlatform.buildRustPackage {
  inherit cargoLock buildFeatures;
  name = "midnight-did-resolver";
  src = lib.cleanSourceWith {
    filter =
      path: _:
      let
        baseName = builtins.baseNameOf path;
      in
      !(baseName == "docs" || baseName == ".github" || baseName == "README.md");
    src = ./../..;
  };
   doCheck = false;

   postPatch = ''
     mkdir -p /build/cargo-vendor-dir/static
     cp ${bls_filecoin_2p14} /build/cargo-vendor-dir/static/bls_filecoin_2p14
     ls -aoh /build/cargo-vendor-dir
   '';
}
