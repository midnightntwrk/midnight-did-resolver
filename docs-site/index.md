# Midnight DID Resolver

This repository owns the resolver-facing runtime around the `did:midnight` method:

- Rust resolver binary and Docker/Nix packaging.
- TypeScript resolver service for REST, Swagger, and UI use.
- DID manager service for wallet-backed DID lifecycle operations.
- Secret-storage package for local key custody, signing, verification, and HD derivation.

The core DID contract, domain model, and API packages remain in [`midnight-did`](https://github.com/midnightntwrk/midnight-did). In the identity workspace, this repository consumes that checkout through local `file:` dependencies.

## Quick Commands

```bash
npm ci
./run.sh --light --strict
./run.sh docs
just build
just test
```

Use [Local Development](/guide/local-development) for setup details.
