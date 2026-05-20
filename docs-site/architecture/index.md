# Architecture

The resolver repository owns service-side identity runtime concerns that sit above the core DID method packages.

```mermaid
graph TD
  Core[midnight-did core packages]
  Secret[secret-storage]
  Resolver[did-resolver-service]
  Manager[did-manager-service]
  Rust[Rust resolver]
  Indexer[(Midnight indexer)]
  Node[(Midnight node)]
  Proof[(Proof server)]

  Resolver --> Core
  Resolver --> Indexer
  Manager --> Core
  Manager --> Secret
  Manager --> Node
  Manager --> Proof
  Rust --> Indexer
```

Use these architecture pages when changing boundaries between resolver, manager, and local key custody.
