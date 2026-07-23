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

The TypeScript workspace consumes the public `midnight-did` packages from npmjs. The root `package.json` overrides keep the DID package graph pinned consistently to `0.5.0-rc1`, its ledger runtime at `8.0.3`, and its network-id singleton at `4.0.2`. These pins prevent duplicate WASM class instances and split network configuration across the DID API wallet and contract graph.

The temporary `@midnight-ntwrk/contract` npm alias exposes the published contract package at the default ZK artifact path expected by `midnight-did-api@0.5.0-rc1`. Remove the alias after a DID API release resolves artifacts from the published `@midnight-ntwrk/midnight-did-contract` package name directly.

## Validation

```bash
./run.sh targets
./run.sh --light --strict
./run.sh secret-storage --strict
./run.sh resolver --light --strict
./run.sh manager --light --strict
./run.sh docs
```

## Non-Standalone Manager Secret Policy

For `preprod` and `mainnet`, set an explicit manager secret passphrase before starting the manager:

```bash
export DID_MANAGER_SECRET_PASSPHRASE=replace-with-a-local-operator-secret
```

The development fallback is standalone-only. `preprod` and `mainnet` manager profiles always require `DID_MANAGER_SECRET_PASSPHRASE`.
