> [!NOTE]
> This page is generated from `README.md` in the repository.
> Edit the source file instead of this generated page.
# Midnight DID Resolver

This repository owns the TypeScript resolver-facing runtime around the `did:midnight` method.

It contains:

- TypeScript resolver service with REST, Swagger, and a browser UI.
- DID manager service for wallet-backed DID lifecycle operations.
- Secret-storage package for local key custody, signing, verification, and HD derivation.
- VitePress documentation that can be published to GitHub Pages.

The core DID contract, domain model, and TypeScript API packages remain in [`midnight-did`](https://github.com/midnightntwrk/midnight-did). In the identity workspace this repository consumes a sibling `../midnight-did` checkout through local `file:` dependencies until DID packages are published.

## Repository Layout

| Path | Responsibility |
| --- | --- |
| `apps/did-resolver-service/` | TypeScript resolver REST/Swagger/UI service. |
| `apps/did-manager-service/` | TypeScript wallet-backed DID manager service and UI. |
| `apps/docs-site/` | VitePress documentation site. |
| `packages/secret-storage/` | TypeScript key custody, signing, verification, and HD derivation package. |
| `infrastructure/` | Local standalone and proof-server compose files used by service scripts. |
| `scripts/` | Repository automation helpers. |

## Quick Start

Prerequisites:

- Node.js 24 and pnpm 10.
- A sibling `../midnight-did` checkout on `develop`.
- Docker for standalone integration and browser tests.

```bash
pnpm install
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
pnpm run docs:dev
pnpm run docs:build
```

The docs site uses `apps/docs-site/` and includes TypeScript service and package documentation.

## Configuration

TypeScript service defaults are documented in the service READMEs and docs site.

## References

- [W3C DID Core](https://www.w3.org/TR/did-core/)
- [W3C DID Resolution](https://www.w3.org/TR/did-resolution/)
- [Midnight DID](https://github.com/midnightntwrk/midnight-did)
- [Midnight Indexer](https://github.com/midnightntwrk/midnight-indexer)

## License

Apache-2.0

