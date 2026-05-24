---
name: midnight-identity
description: "Use this skill for midnight-did-resolver repository work: resolver service, manager service, secret storage, docs, and CI lanes."
---

# Midnight Identity Resolver Skill

Use this skill from `midnight-did-resolver` when work changes resolver runtime services, manager service, secret storage, or their docs.

## Required Context

1. Read repository-root `AGENT.md` first.
2. Align service hardening or protocol assumptions with the consuming DID/VC teams.
3. Keep DID core method/model changes in `midnight-did`; keep VC semantics in `midnight-verifiable-credentials`.

## Defaults

- Target branch is `develop` unless instructed otherwise.
- Use DCO/GPG for repository-facing commits: `git commit -S --signoff -m "<type>: <subject>"`.
- Keep `@midnight-ntwrk/midnight-did-secret-storage` as the single secret-store package boundary.

## PR Gate (required before any PR)

Run at least these before opening a PR:

```bash
./run.sh --light
./run.sh --strict secret-storage
./run.sh --strict resolver
./run.sh --strict manager
```

If any command fails, do not open the PR.

## Validation

```bash
./run.sh targets
./run.sh --light
./run.sh --light secret-storage
./run.sh --strict resolver
./run.sh --strict manager
./run.sh docs
```

Use `./run-resolver.sh` or `./run-manager.sh` for local process-level debugging when needed.
