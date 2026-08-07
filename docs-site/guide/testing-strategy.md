# Testing Strategy

## Fast Loop

Use the light runner for source changes that should not start Docker or browser infrastructure:

```bash
./run.sh --light --strict
```

This runs the secret-storage, TypeScript resolver, and manager unit lanes while skipping long-running integration and Playwright flows.

## Coverage

Run the package coverage suites and enforce the committed per-package thresholds:

```bash
npm run coverage
```

The command is Docker-free and produces text, JSON, JSON-summary, and HTML
reports under each package's `coverage/` directory. Coverage thresholds are
checked independently for secret-storage, resolver, and manager so a strong
aggregate cannot hide a regression in one package.

## Full TypeScript Loop

```bash
./run.sh --strict
```

The full lane includes resolver integration tests and manager browser tests. It expects Docker to be available and may start proof-server backed infrastructure.
