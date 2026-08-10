# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
