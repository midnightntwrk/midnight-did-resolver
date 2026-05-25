# @midnight-ntwrk/midnight-did-secret-storage

Generated API reference.

**@midnight-ntwrk/midnight-did-secret-storage**

***

# @midnight-ntwrk/midnight-did-secret-storage

## Classes

- [FileSecretStore](classes/FileSecretStore.md)
- [SecretNotFoundError](classes/SecretNotFoundError.md)
- [SecretStoreError](classes/SecretStoreError.md)
- [SecretStoreInitError](classes/SecretStoreInitError.md)
- [SecretStoreLockedError](classes/SecretStoreLockedError.md)
- [SigningNotSupportedError](classes/SigningNotSupportedError.md)
- [UnsupportedCurveError](classes/UnsupportedCurveError.md)
- [VeramoSecretStore](classes/VeramoSecretStore.md)
- [VerificationFailedError](classes/VerificationFailedError.md)

## Interfaces

- [SecretStorage](interfaces/SecretStorage.md)

## Type Aliases

- [DeriveKeyFromSeedInput](type-aliases/DeriveKeyFromSeedInput.md)
- [GenerateKeyInput](type-aliases/GenerateKeyInput.md)
- [ImportKeyInput](type-aliases/ImportKeyInput.md)
- [MidnightCurve](type-aliases/MidnightCurve.md)
- [MidnightKeyType](type-aliases/MidnightKeyType.md)
- [PublicJwk](type-aliases/PublicJwk.md)
- [SecretKeyRef](type-aliases/SecretKeyRef.md)
- [Seed](type-aliases/Seed.md)
- [SignOutput](type-aliases/SignOutput.md)
- [StoredKeyMeta](type-aliases/StoredKeyMeta.md)
- [StoredPrivateRecord](type-aliases/StoredPrivateRecord.md)
- [VeramoLikeAgent](type-aliases/VeramoLikeAgent.md)
- [VerifyInput](type-aliases/VerifyInput.md)

## Variables

- [SEED\_HEX\_BYTES](variables/SEED_HEX_BYTES.md)
- [SEED\_HEX\_LENGTH](variables/SEED_HEX_LENGTH.md)
- [SeedSchema](variables/SeedSchema.md)

## Functions

- [deriveCurvePrivateFromSeed](functions/deriveCurvePrivateFromSeed.md)
- [generateCurveKey](functions/generateCurveKey.md)
- [importCurveKey](functions/importCurveKey.md)
- [isPublicJwkLedgerCompatible](functions/isPublicJwkLedgerCompatible.md)
- [normalizePublicForLedger](functions/normalizePublicForLedger.md)
- [parseSeed](functions/parseSeed.md)
- [seedToBuffer](functions/seedToBuffer.md)
- [signWithCurveKey](functions/signWithCurveKey.md)
- [verifyWithPublicJwk](functions/verifyWithPublicJwk.md)
