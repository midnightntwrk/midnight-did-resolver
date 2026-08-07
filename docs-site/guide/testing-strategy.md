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

Every pull request targeting `develop` or `main` runs the same command in the
required **Coverage thresholds** CI job. Reports are retained as CI artifacts
for 14 days, including when the threshold check fails.

## Full TypeScript Loop

```bash
./run.sh --strict
```

The full lane includes resolver integration tests and manager browser tests. It expects Docker to be available and may start proof-server backed infrastructure.

The pull-request **Service E2E smoke** job runs the resolver Docker integration
suite and the manager standalone Playwright suite with the same Node 24 and
Docker prerequisites. It retains Playwright traces/screenshots and service
failure diagnostics for 14 days. The manager E2E also restarts the service and
verifies that its persisted profile state remains available.
