{ pkgs }:

let
  rootDir = "$ROOT_DIR";
  inherit (pkgs.rustTools) rust;
in
pkgs.mkShell {
  packages = with pkgs; [
    # base
    docker
    git
    git-cliff
    jq
    just
    less
    ncurses
    nix
    which
    # linters & formatters
    nixfmt-rfc-style
    taplo
    # rust
    cargo-edit
    cargo-expand
    cargo-license
    cargo-udeps
    rust
    # midnight js
    compactc
    nodejs_22
    typescript-language-server
  ];

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}"
  '';

  # envs
  RUST_LOG = "info,oura=warn,tower_http::trace=debug";
}
