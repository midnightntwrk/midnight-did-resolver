# Local Development

## Repository Layout

| Path | Responsibility |
| --- | --- |
| `did-resolver-service/` | TypeScript resolver service with REST, Swagger, and browser UI. |
| `did-manager-service/` | TypeScript DID manager backend and single-user UI. |
| `secret-storage/` | Local encrypted key storage, signing, verification, and HD derivation. |
| `docs-site/` | VitePress documentation site for GitHub Pages. |
| `libs/midnight-did/` | DID package tarballs copied by the workspace-root sync script. |

## Prerequisites

- Node.js 24 and npm 10.
- Docker for standalone integration and browser flows.
- DID package tarballs in `libs/midnight-did/`.

## Setup

```bash
npm ci
npm run build:did-prereqs
```

The TypeScript workspace uses local tarball dependencies until DID packages are published. In the identity workspace, refresh them from the workspace root:

```bash
./scripts/sync-package-tarballs.sh --source did --destination midnight-did-resolver
```

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
