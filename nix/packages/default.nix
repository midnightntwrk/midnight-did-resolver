{
  perSystem =
    { pkgs, ... }:
    {
      packages = {
        compact-midnight = pkgs.callPackage ./compact-midnight.nix { };
        pi-coding-agent = pkgs.callPackage ./pi-coding-agent.nix { };
      };
    };
}
