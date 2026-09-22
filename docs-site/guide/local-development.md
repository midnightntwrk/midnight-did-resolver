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

The TypeScript workspace consumes the public `midnight-did` packages from npmjs. The root `package.json` overrides keep the DID package graph pinned consistently to `0.7.0`, its ledger runtime at `8.1.0`, and its network-id singleton at `4.0.2`. These pins prevent duplicate WASM class instances and split network configuration across the DID API wallet and contract graph.

The `@midnight-ntwrk/contract` npm alias in `package.json` remains aligned with the release’s `@midnight-ntwrk/midnight-did-contract` package version to preserve the artifact path expected by the DID API runtime.

## Midnight DID 0.7 migration

Version 0.7.0 changes the DID Document projection of native ledger Jubjub
public-key coordinates from little-endian to canonical, fixed-width 32-byte
unsigned big-endian base64url values. Native ledger points, signatures,
offchain MOD1 bytes, and DID hashes do not change.

Before releasing a resolver built with 0.7.0:

1. Invalidate cached or persisted DID Document snapshots produced by 0.6.0 or
   earlier.
2. Re-resolve ledger-backed DIDs through the upgraded resolver.
3. Rebuild JWK thumbprints, JWK-derived key identifiers, and dependent caches.
4. Keep offchain MOD1 identifiers unchanged and pass only canonical big-endian
   JWK coordinates to the 0.7 domain API.

Do not reinterpret old snapshots in place or add an unmarked endian fallback.
The file-backed secret store performs a narrower, safe migration automatically:
when it opens an existing encrypted store, it recomputes each persisted Jubjub
public JWK from the stored private seed and writes the canonical 0.7 coordinates
back without changing the key reference. This does not migrate external DID
Document snapshots or caches; those still require the invalidation steps above.

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
