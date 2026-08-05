# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
