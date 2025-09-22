{
  lib,
  makeRustPlatform,
  rust,
  rustTools,
  cargoLock,
  buildFeatures ? [ ],
}:

let
  rustPlatform = makeRustPlatform {
    cargo = rust;
    rustc = rust;
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

  preBuild = rustTools.patchScript;
}
