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

---

**References**

- [W3C DID Core Specification](https://www.w3.org/TR/did-core/)
- [W3C DID Resolution Specification](https://www.w3.org/TR/did-resolution/)
- [Midnight DID repository](https://github.com/midnightntwrk/midnight-did)
- [Midnight Indexer repository](https://github.com/midnightntwrk/midnight-indexer)
