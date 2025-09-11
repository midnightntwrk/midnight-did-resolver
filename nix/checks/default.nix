{ pkgs, ... }:

{
  default = pkgs.callPackage ./resolver-checks.nix { };
}
