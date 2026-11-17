> [!NOTE]
> This page is generated from `README.md` in the repository.
> Edit the source file instead of this generated page.
# Midnight DID Resolver

This repository owns the resolver-facing runtime around the `did:midnight` method.

It contains:

- TypeScript resolver service with REST, Swagger, and a browser UI.
- DID manager service for wallet-backed DID lifecycle operations.
- Secret-storage package for local key custody, signing, verification, and HD derivation.
- VitePress documentation that can be published to GitHub Pages.

The core DID contract, domain model, and TypeScript API packages remain in [`midnight-did`](https://github.com/midnightntwrk/midnight-did). This repository consumes the exact public `1.0.0-rc1` Ledger9 release from npmjs, not npm `latest`. See the `Ledger9 compatibility and migration profile`.

> [!IMPORTANT]
> Midnight DID 0.7.0 projects native ledger Jubjub JWK coordinates as canonical,
> fixed-width big-endian values. Deployments upgrading from 0.6.0 or earlier
> must invalidate cached or persisted DID Documents, re-resolve ledger-backed
> DIDs, and rebuild JWK thumbprints and JWK-derived identifiers. Do not add an
> unmarked little-endian fallback.

## Repository Layout

| Path                    | Responsibility                                                            |
| ----------------------- | ------------------------------------------------------------------------- |
| `secret-storage/`       | TypeScript key custody, signing, verification, and HD derivation package. |
| `did-resolver-service/` | TypeScript resolver REST/Swagger/UI service.                              |
| `did-manager-service/`  | TypeScript wallet-backed DID manager service and UI.                      |
| `docs-site/`            | VitePress documentation site.                                             |
| `infrastructure/`       | Local standalone and proof-server compose files used by service scripts.  |

## TypeScript Quick Start

Prerequisites:

- Node.js 24 and npm >=10 (tested locally with Node 24.22.1 / npm 11.19.1).
- Access to the public npmjs registry.
- Docker for standalone integration and browser tests.

```bash
npm ci
./run.sh --light --strict
```

Useful targets:

```bash
./run.sh targets
./run.sh secret-storage --strict
./run.sh resolver --light --strict
./run.sh manager --light --strict
./run.sh docs
```

Start service UIs:

```bash
./start-resolver.sh --preprod
./start-manager.sh --preprod
```

Standalone mode expects local Docker infrastructure:

```bash
docker compose -f infrastructure/standalone.yml up -d
./start-resolver.sh --standalone
./start-manager.sh --standalone
```

## Docs

```bash
npm run docs:dev
npm run docs:build
```

The docs site uses `docs-site/` and covers the TypeScript service and package surface.

## Pi development shell

This repository supports the optional Pi shell for structured `dev-loops` workflows.

- Install and use Pi:

  ```bash
  pi
  ```

- For command examples and session setup, see ``docs/pi-development.md``.

## Docker demo

The `demo/` directory provides a reproducible Docker Compose setup using the
locally built Ledger9 resolver and manager images together with node 2.1.101,
indexer 4.4.101 and proof-server 9.0.1. Older published service RC images are
not compatible with this topology:

```bash
just demo
```

This starts the stack, waits for both application health endpoints, and keeps
manager state in `~/.midnight-did/manager` between runs. Copy
`demo/.env.example` to `demo/.env` to override image references, host ports, or
the manager data directory. Open the resolver at `http://127.0.0.1:3001` and the
manager wallet at `http://127.0.0.1:3010/wallet`. See
``demo/README.md`` for the complete command reference.

## Docker image publishing

GitHub Container Registry images are published via
`.github/workflows/release-docker.yml`.

- `ghcr.io/midnightntwrk/midnight-did-resolver-service:<version>`
- `ghcr.io/midnightntwrk/midnight-did-manager-service:<version>`

The channel-aware workflow proposed for `develop` in
[#165](https://github.com/midnightntwrk/midnight-did-resolver/issues/165) is
**manual-only**, with required `channel` and full `version` inputs:

| Channel | Source branch | Version and image tags |
| --- | --- | --- |
| `rc` | Exact `develop` or `main` | `X.Y.Z-rc.N` only; never `latest` |
| `release` | Exact `main` only | `X.Y.Z` and `latest` |

`N` must be positive with no leading zeros. The base must match the root,
resolver, manager and secret-storage package versions. No `v` prefix, build
metadata or implicit version fallback is accepted; the full tag must fit 128
characters. Both images use the same version and exact dispatched source SHA,
with default `linux/amd64,linux/arm64` builds, provenance/SBOM attestations and
keyless digest signatures verified against the exact source workflow identity.

The selected next Docker candidate is **`1.0.0-rc.1` (planned, not published)**.
The four runtime manifests retain base `1.0.0`, not the full prerelease version.
Stable `1.0.0` below is future illustrative usage only, requiring separate
approval and reviewed `main` promotion; this alignment authorizes no publication.

Examples **only after separate release approval and installation of the changed
workflow at the selected ref**:

```bash
gh workflow run release-docker.yml --ref develop -f channel=rc -f version=1.0.0-rc.1
gh workflow run release-docker.yml --ref main -f channel=release -f version=1.0.0
```

Pushes, merges and `v*` tags do not publish from refs containing the changed
workflow. Until approved promotion updates `main`/the default branch, its older
workflow may still have tag-triggered/main-only behavior; this PR does not deploy
all branch copies or authorize testing them by dispatch/tag creation. Existing
artifacts remain untouched. Source eligibility is not release authorization, and
the security/reconciliation gates in #109/#137 remain outstanding.

See `Publishing` for the
transition, input contract, stable promotion/approval prerequisites and exact
source/digest/signature verification commands. Non-publishing regression checks:
`npm run test:release-policy`.

## Configuration

Resolver service variables are documented in the service README and docs site.

TypeScript service defaults are documented in the service READMEs and docs site.

## References

- [W3C DID Core](https://www.w3.org/TR/did-core/)
- [W3C DID Resolution](https://www.w3.org/TR/did-resolution/)
- [Midnight DID](https://github.com/midnightntwrk/midnight-did)
- [Midnight Indexer](https://github.com/midnightntwrk/midnight-indexer)

## License

Apache-2.0
