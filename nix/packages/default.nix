{ pkgs }:

let
  version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../../version);
  callPackageRustCross =
    targetSystem: path: overrides:
    pkgs.pkgsCross."${targetSystem}".callPackage path (
      {
        rust = pkgs.rustTools.mkRustCross {
          pkgsCross = pkgs.pkgsCross."${targetSystem}";
          minimal = true;
        };
      }
      // overrides
    );
  mkResolverPackages =
    {
      buildFeatures ? [ ],
      extraPackages ? [ ],
    }:
    rec {
      midnight-did-resolver-bin = pkgs.callPackage ./midnight-did-resolver-bin.nix {
        inherit buildFeatures;
        rust = pkgs.rustTools.rustMinimal;
        inherit (pkgs.rustTools) cargoLock postPatch;
      };
      midnight-did-resolver-bin-x86_64-linux =
        callPackageRustCross "gnu64" ./midnight-did-resolver-bin.nix
          {
            inherit buildFeatures;
            inherit (pkgs.rustTools) cargoLock postPatch;
          };
      midnight-did-resolver-bin-aarch64-linux =
        callPackageRustCross "aarch64-multiplatform" ./midnight-did-resolver-bin.nix
          {
            inherit buildFeatures;
            inherit (pkgs.rustTools) cargoLock postPatch;
          };
      midnight-did-resolver-docker = pkgs.callPackage ./midnight-did-resolver-docker.nix {
        inherit version extraPackages;
        midnight-did-resolver = midnight-did-resolver-bin;
      };
      midnight-did-resolver-docker-linux-amd64 =
        pkgs.pkgsCross.gnu64.callPackage ./midnight-did-resolver-docker.nix
          {
            inherit version extraPackages;
            midnight-did-resolver = midnight-did-resolver-bin-x86_64-linux;
            tagSuffix = "-amd64";
          };
      midnight-did-resolver-docker-linux-arm64 =
        pkgs.pkgsCross.aarch64-multiplatform.callPackage ./midnight-did-resolver-docker.nix
          {
            inherit version extraPackages;
            midnight-did-resolver = midnight-did-resolver-bin-aarch64-linux;
            tagSuffix = "-arm64";
          };
    };
in
{ inherit (pkgs.pkgsInternal) midnight-did-serde-js; } // (mkResolverPackages { })
