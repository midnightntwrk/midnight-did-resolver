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
- GitHub read access to `midnightntwrk/midnight-did` for DID package tags.

## Setup

```bash
npm ci
npm run build:did-prereqs
```

The TypeScript workspace uses package-root Git tags from `midnight-did` until DID packages are published to GitHub Packages with resolver access. The root `package.json` overrides pin all DID packages to the `0.4.0` package tags.

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
