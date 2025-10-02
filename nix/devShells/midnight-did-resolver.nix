{ pkgs }:

let
  rootDir = "$ROOT_DIR";
  inherit (pkgs.rustTools) rust;
  scripts = {
    format = pkgs.writeShellApplication {
      name = "format";
      runtimeInputs = with pkgs; [
        nixfmt-rfc-style
        taplo
      ];
      text = ''
        cd "${rootDir}"
        find . | grep '\.nix$' | xargs -I _ bash -c "echo running nixfmt on _ && nixfmt _"
        find . | grep '\.toml$' | xargs -I _ bash -c "echo running taplo on _ && taplo format _"
        find . | grep '\.dhall$' | xargs -I _ bash -c "echo running dhall format on _ && dhall format _"
        cargo fmt
      '';
    };

    build = pkgs.writeShellApplication {
      name = "build";
      text = ''
        cd "${rootDir}"
        cargo build --all-features
      '';
    };

    clean = pkgs.writeShellApplication {
      name = "clean";
      text = ''
        cd "${rootDir}"
        cargo clean
      '';
    };
  };
in
pkgs.mkShell {
  packages =
    with pkgs;
    [
      # base
      docker
      git
      git-cliff
      jq
      less
      ncurses
      which
      # config
      dhall
      dhall-json
      # rust
      cargo-edit
      cargo-expand
      cargo-license
      cargo-udeps
      rust
      # midnight js
      compactc
      nodejs_22
      pkgsInternal.midnight-did-serde-js
      typescript-language-server
    ]
    ++ (builtins.attrValues scripts);

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}"
  '';

  # envs
  RUST_LOG = "info,oura=warn,tower_http::trace=debug";
}
