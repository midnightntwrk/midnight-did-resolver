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

> **Note:** Currently, only the Nix package is supported. Other package distribution methods will be available soon.

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

## References

- [W3C DID Core Specification](https://www.w3.org/TR/did-core/)
- [W3C DID Resolution Specification](https://www.w3.org/TR/did-resolution/)
- [Midnight DID repository](https://github.com/midnightntwrk/midnight-did)
- [Midnight Indexer repository](https://github.com/midnightntwrk/midnight-indexer)
