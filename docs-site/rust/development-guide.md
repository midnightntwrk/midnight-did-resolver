# Development Guide

## Table of content

- [Design](./design.md)
- [Contract Deserialization](./contract-deserialization.md)
- [Integration Tests](./integration-tests.md)

## Development Setup

### Prerequisites

Before you begin, ensure you have the following:

- **Nix:** Install [Nix](https://nixos.org/download.html) and enable [flakes](https://nixos.wiki/wiki/Flakes).
- **Midnight Indexer URL:** Obtain the URL of your Midnight Indexer instance (required for local development and testing).

### Setting Up the Development Environment

1. **Enter the Nix development shell:**
   ```bash
   nix develop
   ```

2. **Initialize the project:**
   ```bash
   just init
   ```
   This builds the `midnight-did-js` package and installs npm dependencies for integration tests.

3. **Run the resolver in development mode:**
   ```bash
   just run <INDEXER_URL>
   ```
   Or using cargo directly:
   ```bash
   cargo run -p midnight-did-resolver serve --indexer-url <INDEXER_URL>
   ```
   Replace `<INDEXER_URL>` with your Midnight Indexer instance URL.

4. **Access the Swagger UI:**
   - Open `http://localhost:8080` in your browser to view the API documentation.

## Development Workflow

This project uses [Just](https://github.com/casey/just), a command runner similar to `make`, to simplify common development tasks. Just is available in the Nix development shell.

To see all available commands:

```bash
just
```

### Code Formatting

Format all source files (Nix, TOML, justfile, and Rust):

```bash
just format
```

### Building and Testing

Build the project with all features:

```bash
just build
```

Run tests for owned crates (excludes vendored dependencies):

```bash
just test
```

Clean build artifacts:

```bash
just clean
```

### End-to-End Testing

The project includes integration tests using Docker Compose to verify DID resolver functionality with all required services.

See the [Integration Tests Guide](./integration-tests.md) for details.

**Quick commands:**
```bash
just e2e-up && just e2e-run && just e2e-down
```

## Midnight-ledger Dependencies

This project currently uses `ledger-4.0.0` as a core dependency.
However, due to conflicting feature gates in the dependency resolution process, we rely on vendored dependencies from `midnight-indexer` version `2.1.4`.
This version of `midnight-indexer` includes a `Cargo.lock` file, which ensures compatibility between `midnight-ledger` crates.
The vendored dependencies are located in the `vendor-from-indexer` directory.

As the Midnight project prepares to release a new, publicly available version (`v6.0.0`), this project will transition to that version once it becomes available.

**Transition Plan:**
- Continue using the vendored dependencies until `midnight-ledger v6.0.0` is released.
- Once `v6.0.0` is available, update the Midnight DID JS package.
- Update the `Cargo.toml` to use `midnight-ledger-6.0.0`.

For more details on dependency management and future updates, refer to the [design documentation](./design.md).
