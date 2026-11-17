# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Prepare selected next Docker candidate `1.0.0-rc.1` (planned, not published),
  aligning runtime package, legacy stamp and OpenAPI base versions to `1.0.0`.
  No candidate or stable publication is authorized; #109 security/release gates
  and #137 main-history/changelog reconciliation remain outstanding.

- Coordinate public DID 1.0.0-rc1, SDK5, Ledger9, Compact runtime 0.21.0,
  serialized-state adapters and manager wallet/provider compatibility.
- Use the Ledger9 standalone node/indexer/proof cohort, v4 endpoints and locally
  built demo service images; preserve read-only public resolver operation.
- Store required wallet lifecycle bytes and quarantine incompatible caches before
  resync; isolate new private state without erasing seeds, keys or old DBs.
- Freeze the legacy metadata HD path despite SDK4's role rename; cross-version
  baseline derivation, native Jubjub point and signature vectors are preserved.

### Migration

- **Ledger9-only:** old Ledger8 DIDs require replacement deployment, changing the
  address/DID with no automatic on-chain history continuity. No dual-era or CMA
  migration is provided. See `docs/ledger9-migration.md` before upgrading.

### Security

- Refresh inherited brace-expansion patch lines and scope the fast-uri3 patch
  override without forcing it onto parents requiring fast-uri4. Keep audit gates;
  inherited non-blocking/tooling advisories remain separately disclosed.
- Bound the SDK5-compatible topology's Dockerode4 UUID child to patched11.1.1,
  validating its actual CJS v4 API rather than waiving the UUID10 advisory.

## [0.1.0-rc.3] - 2026-09-29

### Added

- Add a runnable Docker Compose demo for the resolver and manager release-candidate images.

### Changed

- Upgrade the complete public `@midnight-ntwrk/midnight-did` package family and
  release-artifact smoke test from the previous package line to `0.7.0`.
- Project native ledger Jubjub JWK coordinates using the canonical fixed-width
  big-endian encoding provided by Midnight DID 0.7.0.

### Migration

- Invalidate DID Document snapshots and caches created by Midnight DID 0.6.0
  or earlier, re-resolve ledger-backed DIDs, and rebuild JWK thumbprints and
  JWK-derived identifiers before serving upgraded resolver traffic.
- Automatically re-project persisted file-store Jubjub public JWKs from their
  encrypted private seeds on first open, preserving existing key references
  while replacing legacy little-endian coordinates with canonical 0.7 values.

### Fixed

- Rebalance manager DID transactions whose dust fee transaction draws the same
  random intent segment id, releasing the dust reserved by the failed merge
  instead of failing the operation.
- Stop pull-request scans from `develop` cancelling the `develop` push scan.

### Security

- Remove the unused `circomlibjs` runtime dependency and its vulnerable
  `elliptic` dependency chain.
- Pin patched `fast-uri` and `js-yaml` releases and update Fastify to its
  patched 5.12 release line for release-gate audit compatibility.

## [0.1.0-rc.2] - 2026-08-10

### Added

- Add package coverage thresholds, a pull-request coverage regression gate, and
  retained coverage reports.
- Add deterministic resolver Docker integration and manager Playwright E2E smoke
  coverage, including manager restart and persistence verification.

### Changed

- Update compatible wallet runtime packages and release-candidate demo defaults.

## [0.1.0] - 2026-08-07

### Added

- Publish multi-platform resolver and manager Docker images to GHCR with
  exact-version `0.1.0` tags and stable `latest` tags.
- Add a runnable Docker Compose demo with persistent manager state and local
  Midnight node, indexer, and proof-server dependencies.
- Add release documentation for the stable Docker image handoff.

## [0.1.0-rc.1] - 2026-08-05

### Added

- Add TypeScript DID resolver and manager applications with health endpoints,
  browser interfaces, and standalone/preprod/mainnet configuration profiles.
- Add reusable encrypted secret storage with supported Midnight key profiles.
- Add multi-platform GHCR publication for resolver and manager container
  images, including provenance and SBOM attestations.
- Add repository-local Pi development-loop integration.

### Changed

- Consume the public `@midnight-ntwrk/midnight-did` `0.5.0` package family
  from npmjs.
- Run services as a non-root container user and keep development dependencies
  out of runtime images.

### Security

- Add public-repository scan, least-permission workflow, dependency audit, and
  CODEOWNERS hardening aligned with `midnight-did`.
