# Midnight DID Resolver - Integration Test Scenarios

This document outlines comprehensive integration test scenarios for the midnight-did-resolver, focusing on DID document representation and serialization logic from the smart contract to the DID document.

**Note**: These tests focus exclusively on resolver/serialization logic. Contract operation logic (Add/Update/Remove operations and state transitions) should be tested in the contract's own test suite.

## Table of Contents

1. [Basic DID Resolution](#1-basic-did-resolution)
2. [Verification Methods](#2-verification-methods)
3. [Verification Relationships](#3-verification-relationships)
4. [Service Endpoints](#4-service-endpoints)
5. [AlsoKnownAs (Aliases)](#5-alsoknownas-aliases)
6. [DID Metadata](#6-did-metadata)
7. [Deactivation](#7-deactivation)
8. [Edge Cases](#8-edge-cases)
9. [Error Handling](#9-error-handling)

---

## 1. Basic DID Resolution

### 1.1 Empty DID Document Resolution
**Description**: Verify that a newly created DID with no operations returns a valid empty DID document.

**Input**:
- Contract state: Newly deployed DID contract with no operations applied
- DID: `did:midnight:undeployed:02007dd39c6606563dd043f06a94f60659b00d4d4ff6a65d2db4ddbc277956c13aa3`

**Expected Output**:
```json
{
  "didResolutionMetadata": { "error": null },
  "didDocument": {
    "@context": [
      "https://www.w3.org/ns/did/v1",
      "https://w3c.github.io/vc-jws-2020/contexts/v1"
    ],
    "id": "did:midnight:undeployed:02007dd39c6606563dd043f06a94f60659b00d4d4ff6a65d2db4ddbc277956c13aa3",
    "alsoKnownAs": [],
    "verificationMethod": [],
    "authentication": [],
    "assertionMethod": [],
    "keyAgreement": [],
    "capabilityInvocation": [],
    "capabilityDelegation": [],
    "service": []
  },
  "didDocumentMetadata": {
    "created": "2024-01-01T00:00:00Z",
    "updated": "2024-01-01T00:00:00Z",
    "deactivated": false,
    "versionId": "0"
  }
}
```

### 1.2 DID Resolution with Contract Version
**Description**: Verify that the contract version from ledger state is correctly handled (note: contractVersion is internal to the contract and not exposed in DID document).

**Input**:
- Contract state with `contractVersion = 1`

**Expected Output**:
- DID document resolves successfully
- Version field in metadata reflects update count, not contract version

---

## 2. Verification Methods

### 2.1 Ed25519 Verification Method Serialization
**Description**: Verify that a DID contract with an Ed25519 key is correctly serialized to a DID document.

**Input**:
- Contract operation: `AddVerificationMethod`
- Verification method:
  - `id`: "key-1"
  - `type`: JsonWebKey
  - `publicKeyJwk`: `{ kty: "OKP", crv: "Ed25519", x: "VCpo2LMLhn6iWku8MKvSLg2ZAoC-nlOyPVQaO3FxVeQ" }`

**Expected Output**:
```json
{
  "verificationMethod": [{
    "id": "did:midnight:undeployed:02007...#key-1",
    "type": "JsonWebKey",
    "controller": "did:midnight:undeployed:02007...",
    "publicKeyJwk": {
      "kty": "OKP",
      "crv": "Ed25519",
      "x": "VCpo2LMLhn6iWku8MKvSLg2ZAoC-nlOyPVQaO3FxVeQ"
    }
  }]
}
```

### 2.2 JubJub Verification Method Serialization
**Description**: Verify that a DID contract with a JubJub key (Midnight-specific) is correctly serialized to a DID document.

**Input**:
- Contract operation: `AddVerificationMethod`
- Verification method:
  - `id`: "key-jubjub"
  - `type`: JsonWebKey
  - `publicKeyJwk`: `{ kty: "EC", crv: "JubJub", x: "3045022100...", y: "00ab5910f48..." }`

**Expected Output**:
```json
{
  "verificationMethod": [{
    "id": "did:midnight:undeployed:02007...#key-jubjub",
    "type": "JsonWebKey",
    "controller": "did:midnight:undeployed:02007...",
    "publicKeyJwk": {
      "kty": "EC",
      "crv": "JubJub",
      "x": "3045022100...",
      "y": "00ab5910f48..."
    }
  }]
}
```

### 2.3 Multiple Verification Methods
**Description**: Verify that multiple verification methods are correctly serialized.

**Input**:
- Multiple `AddVerificationMethod` operations with different key types (Ed25519 and JubJub)

**Expected Output**:
- DID document contains all verification methods
- Each method has unique ID with proper fragment identifier
- Methods are properly indexed in the verificationMethod array

### 2.4 Fragment Identifier Handling
**Description**: Verify that fragment identifiers are correctly added/removed during serialization.

**Input**:
- Contract stores ID as "key-1" (without #)
- DID: `did:midnight:testnet:02007...`

**Expected Output**:
- Serialized verification method ID: `did:midnight:testnet:02007...#key-1`
- Fragment identifier "#" is prepended during deserialization

---

## 3. Verification Relationships

### 3.1 Authentication Relationship
**Description**: Verify authentication relationship serialization.

**Input**:
- Verification method "key-1" added
- Operation: `AddVerificationMethodRelation` with relation=Authentication, methodId="key-1"

**Expected Output**:
```json
{
  "authentication": [
    "did:midnight:undeployed:02007...#key-1"
  ]
}
```

### 3.2 Assertion Method Relationship
**Description**: Verify assertionMethod relationship serialization.

**Input**:
- Verification method "key-1" added
- Operation: `AddVerificationMethodRelation` with relation=AssertionMethod

**Expected Output**:
```json
{
  "assertionMethod": [
    "did:midnight:undeployed:02007...#key-1"
  ]
}
```

### 3.3 Key Agreement Relationship
**Description**: Verify keyAgreement relationship serialization.

**Input**:
- Verification method "key-enc" added
- Operation: `AddVerificationMethodRelation` with relation=KeyAgreement

**Expected Output**:
```json
{
  "keyAgreement": [
    "did:midnight:undeployed:02007...#key-enc"
  ]
}
```

### 3.4 Capability Invocation Relationship
**Description**: Verify capabilityInvocation relationship serialization.

**Input**:
- Verification method "key-1" added
- Operation: `AddVerificationMethodRelation` with relation=CapabilityInvocation

**Expected Output**:
```json
{
  "capabilityInvocation": [
    "did:midnight:undeployed:02007...#key-1"
  ]
}
```

### 3.5 Capability Delegation Relationship
**Description**: Verify capabilityDelegation relationship serialization.

**Input**:
- Verification method "key-delegate" added
- Operation: `AddVerificationMethodRelation` with relation=CapabilityDelegation

**Expected Output**:
```json
{
  "capabilityDelegation": [
    "did:midnight:undeployed:02007...#key-delegate"
  ]
}
```

### 3.6 Multiple Relationships for Same Key
**Description**: Verify that a single verification method can be referenced by multiple relationships.

**Input**:
- Verification method "key-1" added
- Operations: Add to Authentication, AssertionMethod, and CapabilityInvocation

**Expected Output**:
- "key-1" appears in authentication array
- "key-1" appears in assertionMethod array
- "key-1" appears in capabilityInvocation array

---

## 4. Service Endpoints

### 4.1 Simple Service Endpoint (String)
**Description**: Verify service with string endpoint serialization.

**Input**:
- Operation: `AddService`
- Service:
  - `id`: "service-1"
  - `type`: "DIDCommV2"
  - `serviceEndpoint`: "https://example.com/didcomm"

**Expected Output**:
```json
{
  "service": [{
    "id": "did:midnight:undeployed:02007...#service-1",
    "type": "DIDCommV2",
    "serviceEndpoint": "https://example.com/didcomm"
  }]
}
```

### 4.2 Complex Service Endpoint (Object)
**Description**: Verify service with object endpoint serialization.

**Input**:
- Service with complex endpoint object containing uri, accept, and routingKeys

**Expected Output**:
```json
{
  "service": [{
    "id": "did:midnight:undeployed:02007...#didcomm-1",
    "type": "DIDCommV2",
    "serviceEndpoint": {
      "uri": "https://example.com/didcomm",
      "accept": ["didcomm/v2"],
      "routingKeys": ["did:example:mediator#key-1"]
    }
  }]
}
```

### 4.3 Service Endpoint Array
**Description**: Verify service with array of endpoints (mixed string and object).

**Input**:
- Service with serviceEndpoint as array containing both strings and objects

**Expected Output**:
```json
{
  "service": [{
    "id": "did:midnight:undeployed:02007...#service-array",
    "type": "DIDCommV2",
    "serviceEndpoint": [
      "https://example.com/endpoint1",
      {
        "uri": "wss://example.com/endpoint2",
        "routingKeys": ["did:example:mediator"]
      }
    ]
  }]
}
```

### 4.4 Service Endpoint JSON String Deserialization
**Description**: Verify that serviceEndpoint stored as JSON string in contract is properly deserialized.

**Input**:
- Contract stores serviceEndpoint as opaque string: `"{\"uri\":\"https://...\"}"`

**Expected Output**:
- Deserialized object (not escaped JSON string) in DID document

### 4.5 Multiple Services
**Description**: Verify multiple services serialization.

**Input**:
- Multiple `AddService` operations with different IDs

**Expected Output**:
- All services present in service array
- Each with unique ID

---

## 5. AlsoKnownAs (Aliases)

### 5.1 Single Alias
**Description**: Verify adding a single alias.

**Input**:
- Operation: `AddAlsoKnownAs` with value "did:example:alias1"

**Expected Output**:
```json
{
  "alsoKnownAs": ["did:example:alias1"]
}
```

### 5.2 Multiple Aliases
**Description**: Verify multiple aliases.

**Input**:
- Multiple `AddAlsoKnownAs` operations

**Expected Output**:
```json
{
  "alsoKnownAs": [
    "did:example:alias1",
    "did:example:alias2",
    "https://example.com/user/123"
  ]
}
```

### 5.3 Non-DID Alias
**Description**: Verify that non-DID URIs can be used as aliases.

**Input**:
- Operation: `AddAlsoKnownAs` with value "https://example.com/profile"

**Expected Output**:
- URI present in alsoKnownAs array

---

## 6. DID Metadata

### 6.1 Created Timestamp
**Description**: Verify created timestamp serialization from contract.

**Input**:
- Contract state with `created` field (Uint64 milliseconds): 1704067200000

**Expected Output**:
```json
{
  "didDocumentMetadata": {
    "created": "2024-01-01T00:00:00Z"
  }
}
```

### 6.2 Updated Timestamp
**Description**: Verify updated timestamp changes with operations.

**Input**:
- Initial `updated`: 1704067200000
- Operation applied
- New `updated`: 1704070800000

**Expected Output**:
- Metadata updated reflects new timestamp: "2024-01-01T01:00:00Z"

### 6.3 Version Counter
**Description**: Verify versionId increments with each operation.

**Input**:
- Initial `version` counter: 0
- After 3 operations: version counter: 3

**Expected Output**:
```json
{
  "didDocumentMetadata": {
    "versionId": "3"
  }
}
```

### 6.4 Timestamp Precision
**Description**: Verify timestamp conversion from milliseconds to ISO 8601 with second precision.

**Input**:
- Contract timestamp: 1704067234567 (includes milliseconds)

**Expected Output**:
- ISO timestamp: "2024-01-01T00:00:34Z" (seconds only, no milliseconds)

---

## 7. Deactivation

### 7.1 Deactivate Operation
**Description**: Verify that a deactivated DID contract returns proper metadata status.

**Input**:
- Contract state: `active = false`, `deactivated = true`

**Expected Output**:
```json
{
  "didDocumentMetadata": {
    "deactivated": true,
    "updated": "2024-01-15T10:30:00Z"
  }
}
```

### 7.2 Deactivated DID Document Structure
**Description**: Verify that a deactivated DID still returns the DID document but with deactivated flag.

**Input**:
- Deactivated DID with verification methods and services

**Expected Output**:
- DID document returned with all fields
- Metadata has `deactivated: true`
- Resolution successful (not an error)

---

## 8. Edge Cases

### 8.1 Maximum Verification Methods
**Description**: Verify handling when contract has many verification methods (stress test serialization).

**Input**:
- Contract with 50+ verification methods

**Expected Output**:
- All methods serialized correctly
- No truncation or errors

### 8.2 Very Long Service Endpoint JSON
**Description**: Verify handling of complex, large service endpoint objects.

**Input**:
- Service with deeply nested JSON object as endpoint

**Expected Output**:
- Full JSON object deserialized correctly

### 8.3 Unicode in Opaque Strings
**Description**: Verify that Unicode characters in IDs and values are properly handled.

**Input**:
- Service ID with Unicode: "service-测试"
- AlsoKnownAs with Unicode: "did:example:用户123"

**Expected Output**:
- Unicode preserved in serialized output

### 8.4 Empty Sets and Maps
**Description**: Verify serialization of empty collections.

**Input**:
- Contract state with empty `authenticationRelation` set
- Empty `services` map

**Expected Output**:
```json
{
  "authentication": [],
  "service": []
}
```

### 8.5 Field Encoding Large Numbers
**Description**: Verify that Field types in JubJub keys handle large numbers correctly.

**Input**:
- JubJub key with maximum field value

**Expected Output**:
- Correctly encoded in base64url format
- No overflow or truncation

### 8.6 Different Network Identifiers
**Description**: Verify DID resolution across different networks.

**Input**:
- DID: `did:midnight:testnet:02007...`
- DID: `did:midnight:mainnet:02007...`
- DID: `did:midnight:devnet:02007...`

**Expected Output**:
- Correct network reflected in DID document ID
- Resolver can distinguish networks

### 8.7 Contract Address Parsing
**Description**: Verify correct parsing of 68-character hex contract address.

**Input**:
- Contract address: "02007dd39c6606563dd043f06a94f60659b00d4d4ff6a65d2db4ddbc277956c13aa3"

**Expected Output**:
- Full DID: `did:midnight:undeployed:02007dd39c6606563dd043f06a94f60659b00d4d4ff6a65d2db4ddbc277956c13aa3`
- ID matches exactly

---

## 9. Error Handling

### 9.1 Invalid Contract State Format
**Description**: Verify error handling for malformed contract state bytes.

**Input**:
- Invalid hex string

**Expected Output**:
- Resolution error: "invalidDidDocument" or deserialization error
- Appropriate error message

### 9.2 Missing Required Contract Fields
**Description**: Verify handling when contract state is missing expected fields.

**Input**:
- Contract state missing `id` or `version` field

**Expected Output**:
- Resolution error with descriptive message

### 9.3 Invalid Key Type in Verification Method
**Description**: Verify handling of unsupported key types.

**Input**:
- Verification method with `kty: "RSA"` (if not supported)

**Expected Output**:
- Either: Error during resolution
- Or: Key omitted with warning

### 9.4 Service Endpoint Invalid JSON
**Description**: Verify handling when stored serviceEndpoint JSON string is malformed.

**Input**:
- serviceEndpoint opaque string: `"{invalid json}"`

**Expected Output**:
- Resolution error or endpoint as raw string

### 9.5 Timestamp Overflow
**Description**: Verify handling of invalid timestamp values.

**Input**:
- `created` timestamp: 0
- `updated` timestamp: maximum Uint64

**Expected Output**:
- Either: Reasonable default timestamp
- Or: Validation error

### 9.6 DID Method Mismatch
**Description**: Verify error when DID method doesn't match contract address.

**Input**:
- Requested DID: `did:midnight:testnet:0200abc...`
- Contract address: `0200xyz...` (different)

**Expected Output**:
- Resolution error: "notFound" or "invalidDid"

---

## Implementation Notes

### Test Execution Strategy

1. **Setup**: Deploy test DID contracts with various configurations
2. **Serialization**: Fetch contract state and deserialize to DID document
3. **Validation**: Compare against expected output structure
4. **Cleanup**: Optional cleanup of test contracts

### Key Areas to Focus On

1. **Fragment Identifier Handling**: Ensure "#" is properly added/removed
2. **JSON Deserialization**: Service endpoints stored as JSON strings must be parsed
3. **Timestamp Conversion**: Uint64 milliseconds → ISO 8601 string (second precision)
4. **Opaque String Handling**: Proper encoding/decoding of opaque strings
5. **Field Encoding**: JubJub x, y coordinates as base64url

### Out of Scope

- Smart contract logic validation (assertions, access control)
- ZK circuit verification
- Private key management
- Contract deployment mechanics
- Network-specific behaviors
