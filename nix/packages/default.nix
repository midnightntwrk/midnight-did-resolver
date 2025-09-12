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
      midnight-resolver-bin = pkgs.callPackage ./midnight-resolver-bin.nix {
        inherit buildFeatures;
        rust = pkgs.rustTools.rustMinimal;
        inherit (pkgs.rustTools) cargoLock;
      };
      midnight-resolver-bin-x86_64-linux = callPackageRustCross "gnu64" ./midnight-resolver-bin.nix {
        inherit buildFeatures;
        inherit (pkgs.rustTools) cargoLock;
      };
      midnight-resolver-bin-aarch64-linux =
        callPackageRustCross "aarch64-multiplatform" ./midnight-resolver-bin.nix
          {
            inherit buildFeatures;
            inherit (pkgs.rustTools) cargoLock;
          };
      midnight-resolver-docker = pkgs.callPackage ./midnight-resolver-docker.nix {
        inherit version extraPackages;
        midnight-resolver = midnight-resolver-bin;
      };
      midnight-resolver-docker-linux-amd64 =
        pkgs.pkgsCross.gnu64.callPackage ./midnight-resolver-docker.nix
          {
            inherit version extraPackages;
            midnight-resolver = midnight-resolver-bin-x86_64-linux;
            tagSuffix = "-amd64";
          };
      midnight-resolver-docker-linux-arm64 =
        pkgs.pkgsCross.aarch64-multiplatform.callPackage ./midnight-resolver-docker.nix
          {
            inherit version extraPackages;
            midnight-resolver = midnight-resolver-bin-aarch64-linux;
            tagSuffix = "-arm64";
          };
    };
in
{ inherit (pkgs.pkgsInternal) did-midnight-serde; } // (mkResolverPackages { })
