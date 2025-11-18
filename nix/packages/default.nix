{ pkgs }:

let
  version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../../version);
  platforms = [
    {
      name = "x86_64-linux";
      cross = "gnu64";
      tagSuffix = "-amd64";
    }
    {
      name = "aarch64-linux";
      cross = "aarch64-multiplatform";
      tagSuffix = "-arm64";
    }
  ];

  mkBin =
    platform:
    pkgs.pkgsCross."${platform.cross}".callPackage ./midnight-did-resolver-bin.nix {
      inherit (pkgs.rustTools) cargoLock;
      rust = pkgs.rustTools.mkRustCross {
        pkgsCross = pkgs.pkgsCross."${platform.cross}";
        minimal = true;
      };
    };

  mkDocker =
    platform:
    pkgs.pkgsCross."${platform.cross}".callPackage ./midnight-did-resolver-docker.nix {
      inherit version;
      inherit (platform) tagSuffix;
      midnight-did-resolver = mkBin platform;
    };

  bins = builtins.listToAttrs (
    map (p: {
      name = "midnight-did-resolver-bin-" + p.name;
      value = mkBin p;
    }) platforms
  );

  dockers = builtins.listToAttrs (
    map (p: {
      name = "midnight-did-resolver-docker" + p.tagSuffix;
      value = mkDocker p;
    }) platforms
  );
in
rec {
  midnight-did-resolver-bin = pkgs.callPackage ./midnight-did-resolver-bin.nix {
    inherit (pkgs.rustTools) cargoLock;
    rust = pkgs.rustTools.rustMinimal;
  };

  midnight-did-resolver-docker = pkgs.callPackage ./midnight-did-resolver-docker.nix {
    inherit version;
    midnight-did-resolver = midnight-did-resolver-bin;
  };

  midnight-circuit-params = pkgs.callPackage ./midnight-circuit-params.nix { };

  midnight-did-compact = pkgs.callPackage ./midnight-did-compact.nix {
    inherit midnight-circuit-params;
  };

  compactc = pkgs.compactc;
}
// bins
// dockers
