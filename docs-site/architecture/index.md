# Architecture

The resolver repository owns service-side identity runtime concerns that sit above the core DID method packages.

```mermaid
graph TD
  Core[midnight-did core packages]
  Secret[secret-storage]
  Resolver[did-resolver-service]
  Manager[did-manager-service]
  Indexer[(Midnight indexer)]
  Node[(Midnight node)]
  Proof[(Proof server)]

  Resolver --> Core
  Resolver --> Indexer
  Manager --> Core
  Manager --> Secret
  Manager --> Node
  Manager --> Proof
```

Use these architecture pages when changing boundaries between resolver, manager, and local key custody.


## Decisions

- [ADR: Resolver vs Manager Service Split](/architecture/adr-service-split)
- [ADR: Shared Seed and Local Profiles](/architecture/adr-shared-seed-and-profiles)
- [ADR: HD Key Derivation and Ledger Compatibility](/architecture/adr-hd-key-derivation-and-ledger-compatibility)
- [ADR: Service Runtime Hardening](/architecture/adr-service-runtime-hardening)
