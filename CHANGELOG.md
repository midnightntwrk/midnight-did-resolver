# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

### Security

- Remove the unused `circomlibjs` runtime dependency and its vulnerable
  `elliptic` dependency chain.
- Pin patched `fast-uri` and `js-yaml` releases and update Fastify to its
  patched 5.12 release line for release-gate audit compatibility.

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
