# List available commands
default:
    @just --list

# Setup project for development
init:
    nix build .#midnight-did-js -o tests/integration-tests/vendor
    cd tests/integration-tests && npm ci

# Format all source files
format:
    #!/usr/bin/env bash
    set -euo pipefail
    find . | grep '\.nix$' | xargs -I _ bash -c "echo running nixfmt on _ && nixfmt _"
    find . | grep '\.toml$' | xargs -I _ bash -c "echo running taplo on _ && taplo format _"
    find . -name justfile -type f | xargs -I _ bash -c "echo running just --fmt on _ && just --fmt --unstable --justfile _"
    cargo fmt

# Build the project with all features
build:
    cargo build --all-features

# Clean the build artifacts
clean:
    cargo clean

# Run nix flake check for the default check
test:
    nix build .#checks.x86_64-linux.default
