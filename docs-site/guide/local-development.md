# Local Development

## Repository Layout

| Path | Responsibility |
| --- | --- |
| `midnight-did-resolver/` | Rust resolver, indexer client, serde layer, Nix and Docker package. |
| `did-resolver-service/` | TypeScript resolver service with REST, Swagger, and browser UI. |
| `did-manager-service/` | TypeScript DID manager backend and single-user UI. |
| `secret-storage/` | Local encrypted key storage, signing, verification, and HD derivation. |
| `docs-site/` | VitePress documentation site for GitHub Pages. |

## Prerequisites

- Node.js 24 and npm 10.
- Nix with flakes enabled for Rust resolver development.
- Docker for standalone integration and browser flows.
- A sibling `../midnight-did` checkout on `develop` when building TypeScript services from the identity workspace.

## Setup

```bash
npm ci
npm run build:did-prereqs
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

Rust resolver validation remains Just/Nix based:

```bash
nix develop
just build
just test
```


## Non-Standalone Manager Secret Policy

For `preprod` and `mainnet`, set an explicit manager secret passphrase before starting the manager:

```bash
export DID_MANAGER_SECRET_PASSPHRASE=replace-with-a-local-operator-secret
```

Use `DID_MANAGER_ALLOW_DEV_SECRET_PASSPHRASE=true` only for local testing when you intentionally want the development fallback outside standalone mode.
