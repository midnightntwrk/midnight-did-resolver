# ADR: Service Runtime Hardening

Status: accepted for the TypeScript resolver-service migration branch.

## Context

The resolver repository now owns deployable TypeScript services that sit above the core `midnight-did` packages:

- `did-manager-service`
- `did-resolver-service`
- `secret-storage`

A Claude PR review of the service migration identified issues that are easy to miss in local happy-path testing: concurrent manager operations, non-dev secret defaults, resolver endpoint override policy, timeout cleanup, DID binding during detached-signature verification, and DID-resolution error consistency.

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

`DID_MANAGER_SECRET_PASSPHRASE` is required for `preprod` and `mainnet` manager profiles.

`DID_MANAGER_ALLOW_DEV_SECRET_PASSPHRASE=true` exists only as an explicit local-testing escape hatch. Standalone mode may still use the dev fallback because it is intended for local-only development.

### Detached verification binds the method to the DID

When `verificationMethodId` is used as the verification source, the request DID must match the resolved DID and the resolver must return the same absolute method id. A resolver returning a key from another DID is rejected before signature verification.

### Resolver endpoint overrides are public-network only

User-provided `indexerUrl` and `indexerWsUrl` overrides reject credentials and localhost/private/link-local/non-public IP literals. Default configured endpoints are still allowed to be local for standalone development.

This protects the public resolver API from being used as a simple SSRF primitive while preserving local standalone defaults.

### DID-resolution errors use one payload helper

Resolver app and service code share the DID-resolution error payload helper. `notFound` remains `notFound` with HTTP 404 rather than being coerced to `internalError`.

### Resolver timeout cleanup is explicit

Resolver service timeouts use a cleanup path that clears the timer and aborts local timeout state after `Promise.race` settles. The upstream resolver API does not currently expose an abort signal, so this is a best-effort boundary around the service request.

## Validation

Focused validation for these decisions:

```bash
npm --ignore-scripts run test -w @midnight-ntwrk/midnight-did-manager-service -- src/test/config.test.ts src/test/signature-service.test.ts src/test/app.test.ts
npm run test -w @midnight-ntwrk/midnight-did-resolver-service -- src/test/indexer-endpoint-policy.test.ts src/test/resolution-errors.test.ts src/test/service.test.ts src/test/app.test.ts
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
