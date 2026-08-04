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

The core DID contract, domain model, and TypeScript API packages remain in [`midnight-did`](https://github.com/midnightntwrk/midnight-did). This repository consumes the `0.5.0` release from the public npmjs registry.

## Repository Layout

| Path | Responsibility |
| --- | --- |
| `secret-storage/` | TypeScript key custody, signing, verification, and HD derivation package. |
| `did-resolver-service/` | TypeScript resolver REST/Swagger/UI service. |
| `did-manager-service/` | TypeScript wallet-backed DID manager service and UI. |
| `docs-site/` | VitePress documentation site. |
| `infrastructure/` | Local standalone and proof-server compose files used by service scripts. |

## TypeScript Quick Start

Prerequisites:

- Node.js 24 and npm 10.
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

## Docker image publishing

GitHub Container Registry images are published via
`.github/workflows/release-docker.yml`.

- `ghcr.io/midnightntwrk/midnight-did-resolver-service:<version>`
- `ghcr.io/midnightntwrk/midnight-did-manager-service:<version>`

The workflow is configured for:

- Push to tags matching `v*` (for example `v0.1.0`).
- Manual trigger with an optional `version` input.
- Multi-platform publish (`linux/amd64`, `linux/arm64`).

For the current branch, package version defaults to workspace `0.1.0`, so the manual release command is:

```bash
gh workflow run "Release application Docker images" --ref develop -f version=0.1.0
```

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

