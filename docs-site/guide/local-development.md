# Local Development

## Repository Layout

| Path | Responsibility |
| --- | --- |
| `did-resolver-service/` | TypeScript resolver service with REST, Swagger, and browser UI. |
| `did-manager-service/` | TypeScript DID manager backend and single-user UI. |
| `secret-storage/` | Local encrypted key storage, signing, verification, and HD derivation. |
| `docs-site/` | VitePress documentation site for GitHub Pages. |

## Prerequisites

- Node.js 24 and npm 10.
- Docker for standalone integration and browser flows.
- Access to the public npmjs registry.

## Setup

```bash
npm ci
npm run build:did-prereqs
```

The TypeScript workspace consumes the public `midnight-did` packages from npmjs. The root `package.json` overrides keep the DID package graph pinned consistently to `0.5.0`, its ledger runtime at `8.1.0`, and its network-id singleton at `4.0.2`. These pins prevent duplicate WASM class instances and split network configuration across the DID API wallet and contract graph.

The `@midnight-ntwrk/contract` npm alias in `package.json` remains aligned with the release’s `@midnight-ntwrk/midnight-did-contract` package version to preserve the artifact path expected by the DID API runtime.

## Validation

```bash
./run.sh targets
./run.sh --light --strict
./run.sh secret-storage --strict
./run.sh resolver --light --strict
./run.sh manager --light --strict
./run.sh docs
```

## Manager Secret Policy

The manager has no default passphrase. Enter the secret-store passphrase in the browser when starting a session, including after every restart. Recovery is not implemented in the demo.
