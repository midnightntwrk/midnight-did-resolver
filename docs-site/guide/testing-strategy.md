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
