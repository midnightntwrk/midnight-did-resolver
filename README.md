# Midnight DID Resolver

## Overview

Decentralized Identifiers (DIDs) are a new type of identifier that enable verifiable, self-sovereign digital identities.
A DID resolver is software that takes a DID and returns its corresponding DID Document and metadata, following the [W3C DID Resolution Specification](https://www.w3.org/TR/did-resolution/).
This standard ensures interoperability across different DID methods.

**Midnight DID** is a DID method built on the Midnight blockchain, designed for secure, privacy-preserving identity management.
Each Midnight DID is represented by a smart contract on the blockchain, with the format `did:midnight:<network>:<contract-address>`.
The contract state is public and contains all information needed to reconstruct the DID Document, including verification keys, authentication relationships, service endpoints, and metadata.

This repository implements a resolver for Midnight DIDs, using the Midnight Indexer to fetch and deserialize smart contract states from the blockchain into W3C-compliant DID Documents.
It exposes an HTTP API for language-agnostic integration, ensuring seamless interoperability and identity verification across diverse systems through established standards.

## Features

- 🛡️ **Resolution for Midnight DID method**  
  Supports resolving DIDs using the Midnight blockchain, enabling secure and privacy-preserving identity management.

- 📄 **W3C-compliant DID Document models**  
  Produces DID Documents that conform to the [W3C DID Core Specification](https://www.w3.org/TR/did-core/), ensuring interoperability.

- 🔗 **W3C-compliant resolution protocol**  
  Implements the [W3C DID Resolution Specification](https://www.w3.org/TR/did-resolution/) for standardized DID resolution and metadata retrieval.

- 📚 **OpenAPI specification**  
  Provides an OpenAPI definition for easy integration with other tools and services.

## Usage Guide

Get up and running with the Midnight DID Resolver using one of the available methods.

### 🧰 Method 1: Using Nix to build binary

**1. Pre-requisites**
- **Nix**: Install [Nix](https://nixos.org/download.html) and enable [flakes](https://nixos.wiki/wiki/Flakes).
- **Private Repository Access**: Access to the private GitHub repository and credentials via `nix-config`.  
  See [Nix manual for configuring access tokens](https://nix.dev/manual/nix/2.24/command-ref/conf-file.html#conf-access-tokens).
- **Midnight Indexer URL**: Obtain the URL of your Midnight Indexer instance.

**2. Installation**
- Build the resolver binary with Nix:
  ```bash
  nix build .#midnight-did-resolver-bin
  ```

**3. Usage**
- Run the resolver:
  ```bash
  ./result/bin/midnight-did-resolver serve --indexer-url <INDEXER_URL>
  ```
- Replace `<INDEXER_URL>` with your Midnight Indexer instance URL.
- Access the Swagger UI at [http://localhost:8080](http://localhost:8080).

### 🐳 Method 2: Using Nix to build Docker image

**1. Prerequisites**
- **Nix:** Install [Nix](https://nixos.org/download.html) and enable [flakes](https://nixos.wiki/wiki/Flakes).
- **Docker:** Install [Docker](https://docs.docker.com/get-docker/) to run containerized applications.
- **Private Repository Access:** Access to the private GitHub repository and credentials via `nix-config`.  
  See [Nix manual for configuring access tokens](https://nix.dev/manual/nix/2.24/command-ref/conf-file.html#conf-access-tokens).
- **Midnight Indexer URL:** Obtain the URL of your Midnight Indexer instance.

**2. Build and Load the Docker Image**
- Build the Docker image using Nix:
  ```bash
  nix build .#midnight-did-resolver-docker
  ```
- Load the image into Docker:
  ```bash
  docker load < ./result
  ```

**3. Run the Resolver Container**
- Start the resolver using Docker:
  ```bash
  docker run -p 8080:8080 midnight-did-resolver:<TAG> serve --indexer-url <INDEXER_URL>
  ```
  - Replace `<TAG>` with the image tag (e.g., `0.1.0` or the other version).
  - Replace `<INDEXER_URL>` with your Midnight Indexer instance URL.

**4. Access the Swagger UI**
- After starting the container, access the Swagger UI at [http://localhost:8080](http://localhost:8080).

## Configuration

### Environment Variables

The resolver can be configured using environment variables or command-line arguments:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `MIDNIGHT_INDEXER_URL` | URL for the Midnight Indexer GraphQL API (e.g., `http://localhost:8088/api/v1/graphql`) | - | Yes |
| `SERVER_ADDRESS` | HTTP server binding address | `0.0.0.0` | No |
| `SERVER_PORT` | HTTP server listening port | `8080` | No |
| `SERVER_CORS_ENABLED` | Enable permissive CORS for cross-origin requests | `false` | No |
| `SERVER_EXTERNAL_URL` | Public URL for external access; used in Swagger docs and as the public endpoint | - | No |
| `RUST_LOG` | Logging level (e.g., `info`, `debug`, `warn`) | `info` | No |

### Example Usage with Environment Variables

```bash
export MIDNIGHT_INDEXER_URL=http://localhost:8088/api/v1/graphql
export SERVER_PORT=3000
export SERVER_CORS_ENABLED=true
export RUST_LOG=debug

./midnight-did-resolver serve
```

### Command-Line Arguments

All configuration options can also be provided as command-line arguments:

```bash
./midnight-did-resolver serve \
  --indexer-url http://localhost:8088/api/v1/graphql \
  --address 0.0.0.0 \
  --port 8080 \
  --cors-enabled \
  --external-url https://resolver.example.com
```

## API Documentation

### Generating OpenAPI Specification

You can generate the OpenAPI specification in JSON format:

```bash
# Output to stdout
./midnight-did-resolver generate-openapi

# Output to file
./midnight-did-resolver generate-openapi --output openapi.json
```

### Interactive API Documentation

Once the resolver is running, access the interactive Swagger UI at:
- [http://localhost:8080](http://localhost:8080) (or your configured port)

### Example DID Resolution Request

**Resolve a DID:**
```bash
curl http://localhost:8080/api/dids/<midnight_did>
```

**Response:**
```json
{
  "didDocument": {
    "@context": ["https://www.w3.org/ns/did/v1"],
    "id": "<midnight_did>",
    "verificationMethod": [...],
    "authentication": [...],
    "assertionMethod": [...],
    "service": [...]
  },
  "didResolutionMetadata": {
    "contentType": "application/did"
  },
  "didDocumentMetadata": {}
}
```

## Contributing

- [Contributing](./CONTRIBUTING.md)
- [Development Guide](./docs/development-guide.md)
- [Integration Tests](./docs/integration-tests.md)

## References

- [W3C DID Core Specification](https://www.w3.org/TR/did-core/)
- [W3C DID Resolution Specification](https://www.w3.org/TR/did-resolution/)
- [Midnight DID repository](https://github.com/midnightntwrk/midnight-did)
- [Midnight Indexer repository](https://github.com/midnightntwrk/midnight-indexer)
