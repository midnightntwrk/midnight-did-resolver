# Testing Strategy

This repository uses a layered strategy. Unit tests provide fast feedback and
package-level coverage thresholds; Docker and browser tests prove service
boundaries and user-visible capabilities; release checks verify the published
container artifacts and demo.

## Confidence model

A release candidate is considered high confidence only when all of these layers
pass:

1. **Static and unit checks**: lint, type checking, deterministic unit tests,
   and per-package coverage thresholds.
2. **Service boundary checks**: HTTP contract tests and resolver container
   smoke tests.
3. **Stateful integration checks**: a real local Midnight node, indexer, proof
   server, and DID lifecycle.
4. **Browser checks**: Playwright exercises the manager UI and its long-running
   operations against real service dependencies.
5. **Artifact checks**: both Docker images build, start, expose health
   endpoints, publish multi-platform manifests, and carry provenance/SBOM
   attestations.

`npm run test:all` is intentionally only the fast unit layer. It is not a
release-signoff command because it does not run Docker integration or Playwright
scenarios.

## Test matrix

| Component | Layer | Command | Capabilities covered |
| --- | --- | --- | --- |
| Resolver | Unit | `npm test -w @midnight-ntwrk/midnight-did-resolver-service` | Configuration, endpoint policy, HTTP routes, DID resolution errors, caching, timeouts, and response mapping |
| Resolver | Container smoke | `npm run test:integration -w @midnight-ntwrk/midnight-did-resolver-service` | Resolver image startup, health/readiness, Swagger availability, malformed DID handling |
| Resolver | Stateful integration | Same command, `resolver.did-flow.test.ts` | Deploy, update, resolve, deactivate, and observe the DID lifecycle through the resolver API using Testcontainers |
| Manager | Unit | `npm test -w @midnight-ntwrk/midnight-did-manager-service` | Configuration, lifecycle delegation, wallet/session state, errors, payload normalization, signatures, profiles, and persistence logic |
| Manager | Browser E2E | `npm run test:e2e:standalone -w @midnight-ntwrk/midnight-did-manager-service` | Wallet setup, funding preparation, DID lifecycle, all supported key curves, relations/services/aliases, signatures, deactivation, restart, persistence, profile isolation, and structured negative API responses |
| Manager | Container smoke | `docker-runtime` CI job | The actual manager image starts with demo dependencies, uses its persistent data directory, and serves health/readiness endpoints |
| Manager | Preprod E2E | `npm run test:e2e:preprod -w @midnight-ntwrk/midnight-did-manager-service` | Real preprod funding and wallet behavior; manual/nightly only because it requires external services and funds |
| Secret storage | Unit/coverage | `npm run coverage -w @midnight-ntwrk/midnight-did-secret-storage` | Encryption, persistence, seed handling, derivation, supported curves, Veramo adapter behavior, and failure cases |

The required CI workflow runs the light unit lane, all package coverage
thresholds, resolver Docker integration, manager standalone Playwright E2E,
manager and resolver Docker runtime smoke, and Docker image builds. Failed runs
retain coverage, E2E diagnostics, and container logs.

## Playwright scenarios

Playwright is useful here because the manager is a browser-driven stateful
application rather than only an HTTP service. It verifies the rendered UI,
controls, disabled/enabled state, asynchronous operation polling, API responses,
DID document updates, and persistence across a real process restart. Traces,
screenshots, videos, and service logs make failures diagnosable in CI.

The current standalone scenario already proves the main happy path:

- guided wallet setup and profile selection;
- funding preparation and session start;
- DID deployment and document visibility;
- Ed25519, Jubjub, and P-256 key generation;
- verification methods, authentication relation, service, alias, and
  deactivation;
- string, canonical JSON, and bytes signing/verification;
- manager restart with persisted profile state; and
- isolation of a newly selected profile.

The scenario should remain deterministic by keeping one stateful lifecycle
serial and using a fresh data directory per run. As it grows, split it into
focused specs with shared fixtures rather than adding more steps to one very
large test:

1. `wallet-setup.spec.ts`: validation, profile selection, funding preparation,
   unlock/close, and disabled controls;
2. `did-lifecycle.spec.ts`: deploy, verification methods, relations, services,
   aliases, resolve, and deactivate;
3. `signatures.spec.ts`: string, JSON, bytes, local-key, public-JWK, and
   DID-document verification;
4. `restart-persistence.spec.ts`: restart, stored profiles, session state, and
   profile isolation; and
5. `error-paths.spec.ts`: malformed input, missing keys, unavailable contracts,
   rejected operations, and attempts to use a closed/deactivated session.

The first four are partly covered by the existing scenario and unit/API tests;
`error-paths.spec.ts` and the split fixtures are the next Playwright additions.
Chromium is sufficient for release-signoff of the current server-rendered UI.
Firefox/WebKit should be added only if browser-specific support becomes a
product requirement.

Playwright does not replace API/unit tests: it is slower and less precise for
branch-level logic. It is the right tool for proving that the browser, manager
HTTP API, operation queue, persistence, and UI state agree end to end.

## Known gaps and prioritised follow-up

### P0 — before calling the next candidate production-ready

- **Manager image runtime smoke**: the required `docker-runtime` CI job now runs
  the actual manager and resolver images with the demo dependencies, verifies
  health/readiness, and exercises the persistent data directory. Repeat the
  same check against the exact published RC tags before stable release.
- **Playwright negative paths**: the standalone scenario now checks malformed
  profile/session requests and missing operations. Add focused coverage for
  closed sessions, unavailable contracts, and failed long-running operations.
- **Release-image demo check**: start the demo with the exact candidate image
  references, verify both application health endpoints, and confirm manager
  state survives `demo-down`/restart without deleting the data directory.

### P1 — strengthen coverage after the candidate

- Split the monolithic manager scenario into focused specs and shared fixtures
  so failures identify one capability and retries do not hide unrelated bugs.
- Add an HTTP contract test matrix generated from the manager and resolver
  OpenAPI schemas, including required fields, status codes, and error payloads.
- Run preprod funding E2E on a scheduled/manual workflow with isolated test
  profiles and explicit funding limits. Do not run it as a required PR check.
- Add a small compatibility matrix for Node 24 and supported Docker/Compose
  versions if those become part of the support policy.

## Coverage and CI gates

Run package coverage and enforce thresholds independently:

```bash
npm run coverage
```

The command produces text, JSON, JSON-summary, and HTML reports under each
package's `coverage/` directory. A strong aggregate cannot hide a regression in
one package. Current minimums are:

- secret-storage: 80% lines/statements/functions, 70% branches;
- resolver: 85% lines/statements/functions, 75% branches; and
- manager: 80% lines/statements/functions, 70% branches.

Every pull request targeting `develop` or `main` runs the required coverage
job. Reports are retained for 14 days, including on failure.

## Local release-signoff commands

Fast validation:

```bash
./run.sh --light --strict
npm run coverage
npm run docs:build
```

Resolver Docker and DID lifecycle checks:

```bash
npm run test:integration -w @midnight-ntwrk/midnight-did-resolver-service
```

Manager browser checks:

```bash
npx playwright install-deps chromium
npm run playwright:install -w @midnight-ntwrk/midnight-did-manager-service
npm run test:e2e:standalone -w @midnight-ntwrk/midnight-did-manager-service
```

Candidate demo check:

```bash
cp demo/.env.example demo/.env
# Set RESOLVER_IMAGE and MANAGER_IMAGE to the candidate tags.
just demo-config
just demo
curl --fail http://127.0.0.1:3001/health
curl --fail http://127.0.0.1:3010/health
just demo-down
```

Use `just demo-clean` only when intentionally deleting persisted manager state.

## Non-goals

- Do not use real mainnet state or funding in pull-request tests.
- Do not treat coverage percentage as a substitute for behavior and failure-path
  tests.
- Do not require preprod availability for ordinary PRs.
- Do not claim a release is verified from image build success alone; run the
  published-image demo and manifest/attestation checks as a separate gate.
