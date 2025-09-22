{ pkgs }:

{
  default = import ./midnight-did-resolver.nix { inherit pkgs; };
}
