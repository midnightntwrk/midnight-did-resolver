# AGENT

Engineering guide for agents and engineers working in `midnight-did-resolver`.

This repository owns resolver runtime services and related reusable packages:

- `apps/did-resolver-service`
- `apps/did-manager-service`
- `packages/secret-storage`
- `apps/docs-site`

Keep product code for DID method primitives (`midnight-did`) and verifiable-credential semantics (`midnight-verifiable-credentials`) in their respective repositories.

## Quick Start

Install dependencies and run targeted validation from this repository:

```bash
pnpm install
pnpm run test:all
```

For lightweight day-to-day validation:

```bash
./run.sh --light
./run.sh --light secret-storage
./run.sh --light resolver --strict
./run.sh --light manager --strict
```

## Validation and PR Gate

Use DCO/GPG for repository-facing commits:

```bash
git commit -S --signoff -m "<type>: <subject>"
```

Before opening a PR, run:

```bash
./run.sh --light
./run.sh --strict resolver
./run.sh --strict manager
./run.sh --strict secret-storage
./run.sh docs
```

If any command fails, do not open the PR.

## Scope Boundaries

- Do not reintroduce DID method, contract, or resolver-domain artifacts into this repository.
- Do not move VC issuance protocol logic here; keep it in `midnight-verifiable-credentials`.
- Keep security hardening and service-runtime changes in this repository unless the change is strictly
  about DID package internals.

## Repository Layout

- `run.sh`: local orchestration for service, manager, and secret-storage checks.
- `scripts/run-target-catalog.mjs`: command mapping for CI target names.
- `apps/docs-site`: generated/runtime docs source.
- `apps/did-manager-service` and `apps/did-resolver-service`: runnable service apps.
- `packages/secret-storage`: reusable TypeScript key custody package.
- `review/`: local review notes and investigation artifacts.
