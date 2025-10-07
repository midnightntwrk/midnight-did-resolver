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
    midnight-compactc = {
      url = "github:midnightntwrk/compactc?ref=v0.24.0";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    midnight-did-src = {
      url = "github:midnightntwrk/midnight-did?ref=did-method-implementation";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      midnight-compactc,
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

              compactc =
                if (pkgs.lib.strings.hasSuffix "-darwin" system) then
                  midnight-compactc.packages.${system}.compactc-binary-macos
                else
                  midnight-compactc.packages.${system}.compactc-binary-nixos;
            })
          ];
        };
      in
      {
        checks = import ./nix/checks/default.nix { inherit pkgs; };
        devShells = import ./nix/devShells/default.nix { inherit pkgs; };
        packages = import ./nix/packages/default.nix { inherit pkgs; };
      }
    );
}
