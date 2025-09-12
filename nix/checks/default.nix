{ pkgs, ... }:

{
  default = pkgs.callPackage ./midnight-did-resolver.nix { };
}
