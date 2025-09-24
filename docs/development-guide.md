## Development Guide

Set up your development environment for the Midnight DID Resolver using Nix and Cargo. This guide will help you get started quickly, whether you're a new contributor or an experienced developer.

### Prerequisites

Before you begin, ensure you have the following:

- **Nix:** Install [Nix](https://nixos.org/download.html) and enable [flakes](https://nixos.wiki/wiki/Flakes).
- **Midnight Indexer URL:** Obtain the URL of your Midnight Indexer instance.

### Setting Up the Development Environment

1. **Enter the Nix development shell:**
   ```bash
   nix develop
   ```

2. **Run the resolver in development mode:**
   ```bash
   cargo run -p midnight-did-resolver serve --indexer-url <INDEXER_URL>
   ```
   Replace `<INDEXER_URL>` with your Midnight Indexer instance URL.

3. **Access the Swagger UI:**
   - Open [http://localhost:8080](http://localhost:8080) in your browser to view the API documentation.

### Testing and Linting

To verify code quality and run tests across the Cargo workspace (including linting), use:

```bash
nix flake check
```

This command will automatically execute all defined checks, run the test suite, and perform linting to ensure your code meets project standards before contributing.
