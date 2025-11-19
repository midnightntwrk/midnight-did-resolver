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

# Run tests for owned crates only (excluding vendored crates)
test:
    #!/usr/bin/env bash
    set -euo pipefail
    CRATES=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.source == null and (.name | test("^midnight-did"))) | .name')
    for CRATE in $CRATES; do
      echo "Testing crate: $CRATE"
      cargo test -p "$CRATE"
    done

# Run e2e tests
[working-directory('tests/integration-tests')]
e2e-run:
    # just e2e-up
    rm -rf midnight-level-db
    npm run test
    # just e2e-down

# Start the e2e test environment
e2e-up:
    nix build .#midnight-did-resolver-docker-latest
    docker load < ./result
    cd tests/integration-tests && docker compose up -d --wait

# Stop and remove the e2e test environment
[working-directory('tests/integration-tests')]
e2e-down:
    docker compose down --volumes
