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
        excludedPaths = [
          "CONTRIBUTING.md"
          "docs"
          ".github"
          ".gitignore"
          "justfile"
          "nix"
          "README.md"
          "SECURITY.md"
          "tests"
        ];
      in
      !(builtins.elem baseName excludedPaths);
    src = ./../..;
  };
  doCheck = false;

  preBuild = rustTools.patchScript;
}
