> [!NOTE]
> This page is generated from `README.md` in the repository.
> Edit the source file instead of this generated page.
# Midnight DID Resolver

This repository owns the resolver-facing runtime around the `did:midnight` method.

It contains:

- Rust resolver binary, indexer client, serde layer, Nix build, and Docker package.
- TypeScript resolver service with REST, Swagger, and a browser UI.
- DID manager service for wallet-backed DID lifecycle operations.
- Secret-storage package for local key custody, signing, verification, and HD derivation.
- VitePress documentation that can be published to GitHub Pages.

The core DID contract, domain model, and TypeScript API packages remain in [`midnight-did`](https://github.com/midnightntwrk/midnight-did). In the identity workspace this repository consumes a sibling `../midnight-did` checkout through local `file:` dependencies until DID packages are published.

## Repository Layout

| Path | Responsibility |
| --- | --- |
| `midnight-did-resolver/` | Rust HTTP resolver binary. |
| `midnight-did-indexer-client/` | Rust GraphQL indexer client. |
| `midnight-did-serde/` | Rust deserialization from ledger/indexer state. |
| `secret-storage/` | TypeScript key custody, signing, verification, and HD derivation package. |
| `did-resolver-service/` | TypeScript resolver REST/Swagger/UI service. |
| `did-manager-service/` | TypeScript wallet-backed DID manager service and UI. |
| `docs-site/` | VitePress documentation site. |
| `infrastructure/` | Local standalone and proof-server compose files used by service scripts. |

## TypeScript Quick Start

Prerequisites:

- Node.js 24 and npm 10.
- A sibling `../midnight-did` checkout on `develop`.
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

## Rust Resolver Quick Start

Prerequisites:

- Nix with flakes enabled.
- Midnight Indexer URL for live resolution.

```bash
nix develop
just build
just test
just run <INDEXER_URL>
```

Build binary:

```bash
nix build .#midnight-did-resolver-bin
./result/bin/midnight-did-resolver serve --indexer-url <INDEXER_URL>
```

Build Docker image:

```bash
nix build .#midnight-did-resolver-docker
docker load < ./result
```

## Docs

```bash
npm run docs:dev
npm run docs:build
```

The docs site uses `docs-site/` and includes TypeScript service docs plus Rust resolver design/development material.

## Configuration

Rust resolver variables and CLI options:

| Variable | Description | Default | Required |
| --- | --- | --- | --- |
| `MIDNIGHT_INDEXER_URL` | Midnight Indexer GraphQL API URL. | - | Yes |
| `SERVER_ADDRESS` | HTTP server binding address. | `0.0.0.0` | No |
| `SERVER_PORT` | HTTP server listening port. | `8080` | No |
| `SERVER_CORS_ENABLED` | Enable permissive CORS. | `false` | No |
| `SERVER_EXTERNAL_URL` | Public URL used by Swagger docs. | - | No |
| `RUST_LOG` | Logging level. | `info` | No |

TypeScript service defaults are documented in the service READMEs and docs site.

## References

- [W3C DID Core](https://www.w3.org/TR/did-core/)
- [W3C DID Resolution](https://www.w3.org/TR/did-resolution/)
- [Midnight DID](https://github.com/midnightntwrk/midnight-did)
- [Midnight Indexer](https://github.com/midnightntwrk/midnight-indexer)

## License

Apache-2.0

