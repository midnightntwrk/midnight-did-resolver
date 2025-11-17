{ pkgs, self, ... }:

{
  default = pkgs.callPackage ./midnight-did-resolver.nix { };
  compactc = self.packages.${pkgs.stdenv.system}.compactc;
}
