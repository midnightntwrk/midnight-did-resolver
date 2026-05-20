{
  description = "A Midnight DID resolver";

  nixConfig = {
    extra-substituters = [ "https://cache.iog.io" ];
    extra-trusted-public-keys = [ "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    midnight-did-src = {
      url = "github:midnightntwrk/midnight-did/develop";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      midnight-did-src,
      ...
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-darwin" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.unfree = true;
          overlays = [
            (import rust-overlay)
            (_: prev: {
              inherit midnight-did-src;
              rustTools = prev.callPackage ./nix/rustTools.nix { inherit rust-overlay; };
              compactc = prev.callPackage ./nix/packages/compactc.nix { };
            })
          ];
        };
      in
      {
        checks = import ./nix/checks/default.nix { inherit pkgs self; };
        devShells = import ./nix/devShells/default.nix { inherit pkgs; };
        packages = import ./nix/packages/default.nix { inherit pkgs; };
      }
    );
}
