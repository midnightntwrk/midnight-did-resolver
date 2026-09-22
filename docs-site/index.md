# Midnight DID Resolver

This repository owns the resolver-facing runtime around the `did:midnight` method:

- TypeScript resolver service for REST, Swagger, and UI use.
- DID manager service for wallet-backed DID lifecycle operations.
- Secret-storage package for local key custody, signing, verification, and HD derivation.

The core DID contract, domain model, and API packages remain in [`midnight-did`](https://github.com/midnightntwrk/midnight-did). This repository consumes the `0.7.0` release from the public npmjs registry.

> [!IMPORTANT]
> When upgrading from Midnight DID 0.6.0 or earlier, invalidate cached or
> persisted DID Documents, re-resolve ledger-backed DIDs, and rebuild JWK
> thumbprints and JWK-derived identifiers. Version 0.7.0 emits native ledger
> Jubjub JWK coordinates in canonical fixed-width big-endian form.

## Quick Commands

```bash
npm ci
./run.sh --light --strict
./run.sh docs
```

Use [Local Development](/guide/local-development) for setup details.
