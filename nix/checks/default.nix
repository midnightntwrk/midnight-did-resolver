{ pkgs, ... }:

{
  default = pkgs.callPackage ./midnight-resolver.nix { };
}
