[**@midnight-ntwrk/midnight-did-secret-storage**](../index.md)

***

[@midnight-ntwrk/midnight-did-secret-storage](../index.md) / SecretStorage

# Interface: SecretStorage

## Methods

### deleteKey()

> **deleteKey**(`keyRef`): `Promise`\<`void`\>

#### Parameters

##### keyRef

`string`

#### Returns

`Promise`\<`void`\>

***

### deriveKeyFromSeed()

> **deriveKeyFromSeed**(`params`): `Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Parameters

##### params

[`DeriveKeyFromSeedInput`](../type-aliases/DeriveKeyFromSeedInput.md)

#### Returns

`Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

***

### generateKey()

> **generateKey**(`params`): `Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Parameters

##### params

[`GenerateKeyInput`](../type-aliases/GenerateKeyInput.md)

#### Returns

`Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

***

### getPublicKey()

> **getPublicKey**(`keyRef`): `Promise`\<[`PublicJwk`](../type-aliases/PublicJwk.md)\>

#### Parameters

##### keyRef

`string`

#### Returns

`Promise`\<[`PublicJwk`](../type-aliases/PublicJwk.md)\>

***

### importKey()

> **importKey**(`params`): `Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

#### Parameters

##### params

[`ImportKeyInput`](../type-aliases/ImportKeyInput.md)

#### Returns

`Promise`\<\{ `keyRef`: `string`; `publicJwk`: [`PublicJwk`](../type-aliases/PublicJwk.md); \}\>

***

### initialize()

> **initialize**(`params`): `Promise`\<`void`\>

#### Parameters

##### params

###### location

`string`

###### passphrase?

`string`

#### Returns

`Promise`\<`void`\>

***

### listKeys()

> **listKeys**(`filter?`): `Promise`\<[`StoredKeyMeta`](../type-aliases/StoredKeyMeta.md)[]\>

#### Parameters

##### filter?

###### did?

`string`

#### Returns

`Promise`\<[`StoredKeyMeta`](../type-aliases/StoredKeyMeta.md)[]\>

***

### sign()

> **sign**(`input`): `Promise`\<[`SignOutput`](../type-aliases/SignOutput.md)\>

#### Parameters

##### input

###### keyRef

`string`

###### payload

`Uint8Array`

#### Returns

`Promise`\<[`SignOutput`](../type-aliases/SignOutput.md)\>

***

### verify()

> **verify**(`input`): `Promise`\<`boolean`\>

#### Parameters

##### input

[`VerifyInput`](../type-aliases/VerifyInput.md)

#### Returns

`Promise`\<`boolean`\>
