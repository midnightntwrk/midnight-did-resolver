# Testing Strategy

## Fast Loop

Use the light runner for source changes that should not start Docker or browser infrastructure:

```bash
./run.sh --light --strict
```

This runs the secret-storage, TypeScript resolver, and manager unit lanes while skipping long-running integration and Playwright flows.

## Full TypeScript Loop

```bash
./run.sh --strict
```

The full lane includes resolver integration tests and manager browser tests. It expects Docker to be available and may start proof-server backed infrastructure.

## Rust Resolver Loop

```bash
nix develop
just build
just test
just e2e-up && just e2e-run && just e2e-down
```

Use the Rust loop when changing `midnight-did-resolver/`, `midnight-did-indexer-client/`, `midnight-did-serde/`, or Nix packaging.
