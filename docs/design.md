# Midnight DID Resolver Design

## Project Structure

The Midnight DID Resolver project is organized as a multi-crate cargo workspace.

### Crate Overview

- **midnight-did**  
  Core library for Decentralized Identifier (DID) logic, including contract state management, resolution algorithms, and error handling.

- **midnight-did-indexer-client**  
  GraphQL client for querying the Midnight Indexer, used to fetch contract state and DID-related data for resolution.

- **midnight-did-serde**  
  Serialization and deserialization utilities for DID documents and contracts.
  This crate implements native Rust deserialization, porting logic from the `compact-runtime`.

- **midnight-did-serde-js**  
  JavaScript/TypeScript package for DID serialization and deserialization.
  Built from the compiled DID compact contract and bundled as a JS package, it enables easy deserialization as a blackbox, invoked by the resolver via CLI during the prototype phase.
  These deserialization utilities will be deprecated in favor of a native Rust implementation.

- **midnight-did-resolver**  
  Main resolver application and CLI, orchestrating DID resolution by integrating core logic, indexer queries, and serialization.

### Crate Dependency Diagram

```mermaid
graph TD
    midnight-did
    midnight-did-indexer-client --> midnight-did
    midnight-did-serde --> midnight-did
    midnight-did-serde -- wrapped CLI --> midnight-did-serde-js
    midnight-did-resolver --> midnight-did
    midnight-did-resolver --> midnight-did-indexer-client
    midnight-did-resolver --> midnight-did-serde
```

## Resolution Flow

```mermaid
sequenceDiagram
    actor Controller as DID Controller
    actor Verifier as DID Verifier
    participant Resolver as Midnight DID Resolver
    participant Indexer as Midnight Indexer
    participant Node as Midnight Node

    Controller ->> Node: Deploy contract
    Node -->> Indexer: Index transactions

    Controller --> Verifier: Engage in SSI interaction
    Verifier ->> Resolver: Resolve Midnight DID
    Resolver ->> Indexer: Get current ContractState
    Indexer ->> Resolver: Return ContractState
    Resolver ->> Verifier: DidDocument
```

### Step-by-Step Explanation
1. **Contract Deployment**: The DID Controller deploys a DID contract to the Midnight Node.
2. **Transaction Indexing**: The Midnight Indexer syncs and indexes the contract state for queries.
3. **SSI Interaction**: The Controller and Verifier interact for identity operations (e.g., authentication, credential exchange).
4. **DID Resolution Request**: The Verifier requests DID resolution from the Midnight DID Resolver.
5. **Contract State Query**: The Resolver asks the Indexer for the latest DID contract state.
6. **State Retrieval**: The Indexer returns the current contract state to the Resolver.
7. **DID Document Delivery**: The Resolver validates and formats the DID Document, then sends it to the Verifier.
