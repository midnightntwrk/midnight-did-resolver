# Integration Tests

## Overview

The integration tests verify end-to-end DID resolver functionality using Docker Compose to orchestrate a complete test environment with:
- Midnight Node (local blockchain)
- Midnight Indexer (blockchain state indexing)
- Proof Server (DID contract operations)
- DID Resolver (service under test)

Test scenarios are located in `tests/integration-tests/src/scenarios/`.

## Initialization

Before running tests, initialize the project dependencies:

```bash
nix develop
just init
```

The `just init` command:
- Builds the `midnight-did-js` package using Nix
- Creates symlinks in `tests/integration-tests/vendor/`
- Installs npm dependencies for the integration tests

### Dependency Management

The `midnight-did-js` package is built directly from the [midnight-did](https://github.com/midnightntwrk/midnight-did) repository via a Nix flake input:

```nix
# flake.nix
inputs = {
  midnight-did-src = {
    url = "github:midnightntwrk/midnight-did-resolver/main";
    flake = false;
  };
}
```

This means the JS packages used in integration tests come directly from the upstream repository.

**To upgrade the midnight-did dependency:**
```bash
nix flake lock --update-input midnight-did-src
just init  # Rebuild with new version
```

## Running Tests

**Complete workflow:**
```bash
just e2e-up && just e2e-run && just e2e-down
```

**Individual commands:**
```bash
just e2e-up    # Build Docker image and start services
just e2e-run   # Run integration tests
just e2e-down  # Stop services and clean up volumes
```

## Troubleshooting

**Check service logs:**
```bash
cd tests/integration-tests
docker compose logs <service-name>
```

**Clean up completely:**
```bash
just e2e-down
```

**Reinitialize dependencies:**
```bash
just init
```
