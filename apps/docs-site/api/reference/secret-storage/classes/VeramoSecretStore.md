[**@midnight-ntwrk/midnight-did-secret-storage**](../index.md)

***

[@midnight-ntwrk/midnight-did-secret-storage](../index.md) / VeramoSecretStore

# Class: VeramoSecretStore

## Implements

- [`SecretStorage`](../interfaces/SecretStorage.md)

## Constructors

### Constructor

> **new VeramoSecretStore**(`agent`): `VeramoSecretStore`

#### Parameters

##### agent

[`VeramoLikeAgent`](../type-aliases/VeramoLikeAgent.md)

#### Returns

`VeramoSecretStore`

## Methods

### deleteKey()

> **deleteKey**(`keyRef`): `Promise`\<`void`\>

#### Parameters

##### keyRef

`string`

#### Returns

`Promise`\<`void`\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`deleteKey`](../interfaces/SecretStorage.md#deletekey)

***

### deriveKeyFromSeed()

> **deriveKeyFromSeed**(`_params`): `Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Parameters

##### \_params

[`DeriveKeyFromSeedInput`](../type-aliases/DeriveKeyFromSeedInput.md)

#### Returns

`Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`deriveKeyFromSeed`](../interfaces/SecretStorage.md#derivekeyfromseed)

***

### generateKey()

> **generateKey**(`params`): `Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Parameters

##### params

[`GenerateKeyInput`](../type-aliases/GenerateKeyInput.md)

#### Returns

`Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`generateKey`](../interfaces/SecretStorage.md#generatekey)

***

### getPublicKey()

> **getPublicKey**(`keyRef`): `Promise`\<[`PublicJwk`](../type-aliases/PublicJwk.md)\>

#### Parameters

##### keyRef

`string`

#### Returns

`Promise`\<[`PublicJwk`](../type-aliases/PublicJwk.md)\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`getPublicKey`](../interfaces/SecretStorage.md#getpublickey)

***

### importKey()

> **importKey**(`_params`): `Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Parameters

##### \_params

[`ImportKeyInput`](../type-aliases/ImportKeyInput.md)

#### Returns

`Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`importKey`](../interfaces/SecretStorage.md#importkey)

***

### initialize()

> **initialize**(`_params?`): `Promise`\<`void`\>

#### Parameters

##### \_params?

###### location

`string`

###### passphrase?

`string`

#### Returns

`Promise`\<`void`\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`initialize`](../interfaces/SecretStorage.md#initialize)

***

### listKeys()

> **listKeys**(`filter?`): `Promise`\<[`StoredKeyMeta`](../type-aliases/StoredKeyMeta.md)[]\>

#### Parameters

##### filter?

###### did?

`string`

#### Returns

`Promise`\<[`StoredKeyMeta`](../type-aliases/StoredKeyMeta.md)[]\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`listKeys`](../interfaces/SecretStorage.md#listkeys)

***

### sign()

> **sign**(`input`): `Promise`\<\{ `format`: `"raw"`; `signature`: `Uint8Array`; \}\>

#### Parameters

##### input

###### keyRef

`string`

###### payload

`Uint8Array`

#### Returns

`Promise`\<\{ `format`: `"raw"`; `signature`: `Uint8Array`; \}\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`sign`](../interfaces/SecretStorage.md#sign)

***

### verify()

> **verify**(`_input`): `Promise`\<`boolean`\>

#### Parameters

##### \_input

[`VerifyInput`](../type-aliases/VerifyInput.md)

#### Returns

`Promise`\<`boolean`\>

#### Implementation of

[`SecretStorage`](../interfaces/SecretStorage.md).[`verify`](../interfaces/SecretStorage.md#verify)
