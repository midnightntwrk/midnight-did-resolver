# Local Development

## Repository Layout

| Path | Responsibility |
| --- | --- |
| `apps/did-resolver-service/` | TypeScript resolver service with REST, Swagger, and browser UI. |
| `apps/did-manager-service/` | TypeScript DID manager backend and single-user UI. |
| `packages/secret-storage/` | Local encrypted key storage, signing, verification, and HD derivation. |
| `apps/docs-site/` | VitePress documentation site for GitHub Pages. |

## Prerequisites

- Node.js 24 and pnpm 10.
- Docker for standalone integration and browser flows.
- A sibling `../midnight-did` checkout on `develop` when building TypeScript services from the identity workspace.

## Setup

```bash
pnpm install
pnpm run build:did-prereqs
```

The TypeScript workspace uses local `file:` dependencies pointing at `../midnight-did` until DID packages are published. If you clone this repository independently, place `midnight-did` beside it or adjust the dependency paths.

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
