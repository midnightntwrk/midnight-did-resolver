# ADR: Service Runtime Hardening

Status: accepted for the TypeScript resolver-service migration branch.

## Context

The resolver repository now owns deployable TypeScript services that sit above the core `midnight-did` packages:

- `did-manager-service`
- `did-resolver-service`
- `secret-storage`

A Claude PR review of the service migration identified issues that are easy to miss in local happy-path testing: concurrent manager operations, non-dev secret defaults, resolver endpoint override policy, timeout cleanup, DID binding during detached-signature verification, DID-resolution error consistency, secret-store file persistence, and container runtime privilege.

## Decisions

### Manager operations are single-flight

`OperationStore` claims the current operation synchronously before scheduling background work. A second mutating request receives `operationBusy` while the first operation is current.

Background task failures are captured into the operation status instead of relying on unhandled promise behavior.

### Runtime mutation uses a serialized queue

`ManagerRuntimeState` owns mutable wallet/session state and serializes sensitive stop/session mutations through an async queue.

Wallet-state subscription callbacks also enter the queue before mutating balances or connection phase. This prevents subscription callbacks, idle shutdown, lock/close, and unlock startup from racing on the same runtime fields.

### Unlock cancellation is generation-guarded

Unlock startup uses an unlock generation guard. If a newer unlock or lock supersedes the old generation, the old wallet context is stopped before it can attach providers, secret storage, or ready session state.

### Non-dev secret storage requires explicit operator intent

The manager has no configured default secret-store passphrase. The browser
must provide the passphrase when starting a session, including after every
restart. This applies to standalone, preprod, and mainnet profiles.

### Detached verification binds the method to the DID

When `verificationMethodId` is used as the verification source, the request DID must match the resolved DID and the resolver must return the same absolute method id. A resolver returning a key from another DID is rejected before signature verification.

### Secret-store persistence is private and atomic

The file-backed secret store creates and rewrites the encrypted store with mode `0600`. Writes go through a same-directory temporary file, `fsync`, and atomic rename so process crashes do not leave partial JSON in place. The store keeps derived encryption key material instead of retaining the passphrase string, and wipes temporary key buffers after derivation, import, signing, and encryption/decryption helper use.

### Resolver endpoints are immutable startup configuration

The resolver accepts indexer HTTP and WebSocket endpoints only from startup
configuration. Request-level `indexerUrl` and `indexerWsUrl` fields are rejected.
Configured endpoints may still be local for standalone development.

This keeps the resolver's outbound network boundary operator-controlled and
prevents callers from turning the public API into an SSRF primitive.

### DID-resolution errors use one payload helper

Resolver app and service code share the DID-resolution error payload helper. `notFound` remains `notFound` with HTTP 404 rather than being coerced to `internalError`.

### Resolver container runs without root privileges

The resolver-service runtime Docker stage switches to the bundled `node` user after dependency install and artifact copy. Build-time package installation still runs in the build/runtime setup layer, but the deployed service process does not run as root.

### Resolver timeout cleanup is explicit

Resolver service timeouts use a cleanup path that clears the timer and aborts local timeout state after `Promise.race` settles. The upstream resolver API does not currently expose an abort signal, so this is a best-effort boundary around the service request.

## Validation

Focused validation for these decisions:

```bash
npm --ignore-scripts run test -w @midnight-ntwrk/midnight-did-manager-service -- src/test/config.test.ts src/test/signature-service.test.ts src/test/app.test.ts
npm run test -w @midnight-ntwrk/midnight-did-resolver-service -- src/test/indexer-endpoint-policy.test.ts src/test/resolution-errors.test.ts src/test/service.test.ts src/test/app.test.ts
npm run lint -w @midnight-ntwrk/midnight-did-secret-storage
npm run lint -w @midnight-ntwrk/midnight-did-manager-service
npm run lint -w @midnight-ntwrk/midnight-did-resolver-service
npm --ignore-scripts run build -w @midnight-ntwrk/midnight-did-manager-service
npm run build -w @midnight-ntwrk/midnight-did-resolver-service
npm run check:run-target-catalog
./run.sh --light --strict
```

## Follow-Up

- Keep manager routes scoped to DID CRUD, key/session management, and DID document inspection.
- If the upstream resolver package gains abort-signal support, thread the timeout `AbortSignal` into the ledger-reader/indexer call path.
- Treat endpoint-override policy changes as security-sensitive and include explicit regression tests.
