# Midnight DID Resolver

This repository owns the resolver-facing runtime around the `did:midnight` method:

- TypeScript resolver service for REST, Swagger, and UI use.
- DID manager service for wallet-backed DID lifecycle operations.
- Secret-storage package for local key custody, signing, verification, and HD derivation.

The core DID contract, domain model, and API packages remain in [`midnight-did`](https://github.com/midnightntwrk/midnight-did). Until those packages are published, this repository consumes DID package tarballs copied into `libs/midnight-did/` by the root `midnight-identity-workspace` sync script.

## Quick Commands

```bash
pnpm install
./run.sh --light --strict
./run.sh docs
```

Use [Local Development](/guide/local-development) for setup details.
