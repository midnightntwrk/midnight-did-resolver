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

## Getting Started

Get up and running with the Midnight DID Resolver in just a few steps.

> **Note:** This project uses [Nix](https://nixos.org/download.html) for its development environment and packaging.  
> Please ensure Nix is installed and [flakes](https://nixos.wiki/wiki/Flakes) are enabled on your system before proceeding.

Currently, the resolver is packaged and distributed via Nix only.

### Build from source with Nix

To build and run the resolver locally:

```bash
# Build and output to "result" directory
nix build .#midnight-did-resolver-bin

# Run the built artifact inside the "result" directory
./result/bin/midnight-did-resolver serve --indexer-url <INDEXER_URL>
```

Replace `<INDEXER_URL>` with the URL of your Midnight Indexer instance.

Once the server is running, open your browser and go to [http://localhost:8080](http://localhost:8080) to view the Swagger UI for interactive API documentation.

## References

- [W3C DID Core Specification](https://www.w3.org/TR/did-core/)
- [W3C DID Resolution Specification](https://www.w3.org/TR/did-resolution/)
- [Midnight DID repository](https://github.com/midnightntwrk/midnight-did)
- [Midnight Indexer repository](https://github.com/midnightntwrk/midnight-indexer)
