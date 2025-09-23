{ pkgs }:

{
  default = import ./midnight-did-resolver.nix { inherit pkgs; };
  release = import ./release.nix { inherit pkgs; };
}
