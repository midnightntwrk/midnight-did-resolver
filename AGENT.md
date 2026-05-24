# AGENT

Engineering guide for agents and engineers working in `midnight-did-resolver`.

This repository is intentionally focused on did:midnight runtime services and resolver infrastructure.

## Purpose

`midnight-did-resolver` owns DID resolution and lifecycle services that sit outside the core `midnight-did` Compact/API packages:

- Resolver and wallet API services (TypeScript services + Swagger + UI).
- DID manager service and browser/CLI helper flows.
- Local secret storage package and key-derivation/signing helpers.
- Rust resolver binary, serde/indexer clients, and runtime packaging.
- Documentation site for resolver/mananger runtime and service docs.

The core DID contract, DID document/domain model, and API packages remain in `midnight-did`.

## Repository Layout

| Path | Package / Component | Responsibility |
| --- | --- | --- |
| `did-resolver-service/` | `@midnight-ntwrk/midnight-did-resolver-service` | Public resolver REST API, status reporting, and docs/API surface. |
| `did-manager-service/` | `@midnight-ntwrk/midnight-did-manager-service` | DID lifecycle orchestration for wallet-backed issuance, updates, deactivation, and resolver-bound operations. |
| `secret-storage/` | `@midnight-ntwrk/secret-storage` | Key custody, signing, verification, seed derivation, and store persistence primitives. |
| `midnight-did-resolver/` | - | Rust resolver binary and HTTP service crate entrypoint. |
| `midnight-did-indexer-client/` | - | Rust GraphQL indexer client for ledger state retrieval. |
| `midnight-did-serde/` | - | Rust serde layer for indexer/ledger structures. |
| `docs-site/` | - | VitePress documentation and generated API reference content. |
| `infrastructure/` | - | Docker, compose stacks, and local runtime environment definitions for services. |
| `midnight-did/` | git subtree/dependency | Workspace-local companion to `@midnight-ntwrk/midnight-did` package sources in unresolved local mode. |

## Quick Start

Prerequisites:

- Node.js 24 / npm 10
- Docker (for service and integration lanes)
- Nix with flakes (for Rust resolver builds)
- Local `../midnight-did` checkout in workspace mode (or the intended local dependency source)

```bash
npm ci
./run.sh --light --strict
```

## Runner Targets

Primary entry:

```bash
./run.sh [target] [--light|--strict] [--no-coverage] [--skip-coverage]
```

Useful targets:

```bash
./run.sh targets
./run.sh resolver --light
./run.sh manager --light
./run.sh secret-storage --light --strict
./run.sh docs
```

Start local services in standalone/preprod styles:

```bash
./start-resolver.sh --preprod
./start-manager.sh --preprod
```

## Development Boundaries

- Do not move or rehome components from `midnight-did` into this repo.
- Do not merge resolver-service API shape changes into DID core PRs.
- Keep endpoint overrides non-private / non-loopback unless explicitly guarded for SSRF-hardening requirements.
- Treat secret material and storage directories as disposable unless explicitly persisted by test/lifecycle design.

## Local Testing

For Rust resolver flow:

```bash
nix develop
nix build .#midnight-did-resolver-bin
```

For service validation:

```bash
./run.sh --light --strict
./run.sh --strict
```

For docs:

```bash
npm run docs:dev
```

## Branch, PR, and Commit Discipline

Default target branch is `origin/develop`.

Recommended cycle:

1. Keep service/runtime changes within this repository; only update dependency sources via normal package/version updates.
2. Run focused tests for the owning component first.
3. Run `./run.sh --light --strict`.
4. Run any required strict/full lanes for impacted service packages.
5. Commit with DCO and GPG before pushing:

```bash
git commit -S --signoff -m "<type>: <subject>"
```

## Root workspace coordination

When running this repo from `midnight-identity-workspace`, use workspace targets:

```bash
cd /Users/ysh/iohk/midnight-identity-workspace
./run.sh --light
```

The workspace root orchestrator now treats `midnight-did-resolver` as a required sibling pipeline after `midnight-did`.
