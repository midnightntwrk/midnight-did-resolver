# Integration Test Scenarios for Midnight DID Resolver

## Overview
These test scenarios focus on validating the resolver's ability to correctly resolve DID Documents with various field types, values, and edge cases. The tests assume the smart contract logic is already tested in the midnight-did repository.

---

## 1. DID Identifier Tests

### 1.1 Basic DID Resolution
- **Scenario**: Resolve a newly deployed DID (empty state)
- **Expected**: Returns minimal DID Document with only `id` field populated
- **Fields to verify**: 
  - `id` matches the requested DID
  - All arrays are empty
  - Metadata contains `created` timestamp

### 1.2 Network Variations
- **Scenario**: Resolve DIDs across different networks
- **Test cases**:
  - `did:midnight:undeployed:<address>`
  - `did:midnight:devnet:<address>`
  - `did:midnight:testnet:<address>`
  - `did:midnight:mainnet:<address>`
- **Expected**: Resolver correctly routes to appropriate network endpoint

### 1.3 Invalid DID Format
- **Scenario**: Attempt to resolve malformed DIDs
- **Test cases**:
  - Wrong method: `did:example:testnet:<address>`
  - Missing network segment: `did:midnight:<address>`
  - Invalid address length (not 68 hex chars)
  - Uppercase hex chars in address
- **Expected**: Returns appropriate error codes

---

## 2. Verification Method Tests

### 2.1 Single Verification Method - Ed25519
- **Scenario**: DID with one Ed25519 key
- **Test data**:
  ```json
  {
    "id": "#key-1",
    "type": "JsonWebKey",
    "publicKeyJwk": {
      "kty": "OKP",
      "crv": "Ed25519",
      "x": "<base64url-encoded-value>"
    }
  }
  ```
- **Expected**: Correctly deserializes and reconstructs verification method

### 2.2 Single Verification Method - JubJub
- **Scenario**: DID with one JubJub key (Midnight-specific)
- **Test data**:
  ```json
  {
    "id": "#key-jubjub",
    "type": "JsonWebKey",
    "publicKeyJwk": {
      "kty": "EC",
      "crv": "Jubjub",
      "x": "<field-value>",
      "y": "<field-value>"
    }
  }
  ```
- **Expected**: Correctly handles both x and y coordinates

### 2.3 Multiple Verification Methods
- **Scenario**: DID with multiple keys of mixed types
- **Test data**: 3+ verification methods with both Ed25519 and JubJub keys
- **Expected**: All methods correctly deserialized and returned

### 2.4 Verification Method ID Formats
- **Scenario**: Test different ID format handling
- **Test cases**:
  - Fragment identifier: `#key-1`
  - Full DID URL: `did:midnight:testnet:<addr>#key-1`
  - Relative URL: `/key-1`
- **Expected**: Resolver correctly reconstructs full DID URLs with `#` prefix

### 2.5 Edge Case - Empty publicKeyJwk Fields
- **Scenario**: Verification method with minimal/edge JWK values
- **Test cases**:
  - Minimum x value (all zeros)
  - Maximum x value (all ones)
  - Very small y value
  - Very large y value
- **Expected**: Correctly handles field encoding/decoding

---

## 3. Verification Relationship Tests

### 3.1 Single Relationship Type
- **Scenario**: Test each relationship type individually
- **Test cases**:
  - `authentication` only
  - `assertionMethod` only
  - `keyAgreement` only
  - `capabilityInvocation` only
  - `capabilityDelegation` only
- **Expected**: Correctly resolves method references

### 3.2 Multiple Relationships
- **Scenario**: One verification method referenced by multiple relationships
- **Test data**: `#key-1` in both `authentication` and `assertionMethod`
- **Expected**: Method appears in both relationship arrays

### 3.3 All Relationships Populated
- **Scenario**: DID with all relationship types having references
- **Expected**: All 5 relationship arrays populated correctly

### 3.4 Empty Relationships
- **Scenario**: DID with verification methods but no relationships
- **Expected**: All relationship arrays are empty arrays (not null)

---

## 4. Service Endpoint Tests

### 4.1 Single Service - String Endpoint
- **Scenario**: Service with simple string endpoint
- **Test data**:
  ```json
  {
    "id": "#service-1",
    "type": "DIDCommV2",
    "serviceEndpoint": "https://example.com/endpoint"
  }
  ```
- **Expected**: Correctly deserializes string endpoint

### 4.2 Single Service - Array Endpoint
- **Scenario**: Service with array of string endpoints
- **Test data**:
  ```json
  {
    "serviceEndpoint": [
      "https://primary.example.com",
      "https://backup.example.com"
    ]
  }
  ```
- **Expected**: Array correctly preserved

### 4.3 Single Service - Object Endpoint
- **Scenario**: Service with complex object endpoint
- **Test data**:
  ```json
  {
    "serviceEndpoint": {
      "uri": "https://example.com",
      "routingKeys": ["did:example:mediator#key-1"],
      "accept": ["didcomm/v2"]
    }
  }
  ```
- **Expected**: Object structure preserved (stored as JSON string, rehydrated on resolution)

### 4.4 Single Service - Mixed Array Endpoint
- **Scenario**: Service with array containing strings and objects
- **Test data**:
  ```json
  {
    "serviceEndpoint": [
      "https://example.com/endpoint",
      {
        "uri": "wss://example.com/ws",
        "routingKeys": ["did:example:mediator"]
      }
    ]
  }
  ```
- **Expected**: Mixed array correctly preserved

### 4.5 Multiple Services
- **Scenario**: DID with multiple service endpoints of different types
- **Expected**: All services correctly resolved

### 4.6 Service ID Formats
- **Scenario**: Test different service ID formats
- **Test cases**:
  - Fragment: `#service-1`
  - Full DID URL: `did:midnight:testnet:<addr>#service-1`
  - Query parameter: `?service=messaging`
  - Path: `/routing`
- **Expected**: IDs correctly reconstructed with `#` prefix for fragments

### 4.7 Service Type Variations
- **Scenario**: Different service type values
- **Test cases**:
  - Single string: `"DIDCommV2"`
  - String array: `["LinkedDomains"]` (though spec says string or array, implementation may vary)
- **Expected**: Type correctly preserved

---

## 5. AlsoKnownAs Tests

### 5.1 Empty AlsoKnownAs
- **Scenario**: DID with no aliases
- **Expected**: Empty array

### 5.2 Single Alias
- **Scenario**: DID with one alias
- **Test data**: `["did:example:123"]`
- **Expected**: Single-element array

### 5.3 Multiple Aliases
- **Scenario**: DID with multiple aliases
- **Test data**:
  ```json
  [
    "did:example:123",
    "did:midnight:testnet:0200...",
    "https://example.com/users/alice"
  ]
  ```
- **Expected**: All aliases preserved

### 5.4 Edge Case - Maximum Aliases
- **Scenario**: DID with many aliases (stress test)
- **Expected**: All aliases correctly resolved

---

## 6. DID Document Metadata Tests

### 6.1 Created Timestamp
- **Scenario**: Verify `created` timestamp format
- **Expected**: ISO 8601 UTC format with second precision (e.g., `"2024-01-01T09:30:00Z"`)

### 6.2 Updated Timestamp
- **Scenario**: Verify `updated` timestamp after updates
- **Expected**: 
  - ISO 8601 format
  - Timestamp reflects latest update
  - `updated` >= `created`

### 6.3 Version ID
- **Scenario**: Verify version tracking
- **Expected**: 
  - Monotonically increasing counter
  - String representation of counter

### 6.4 Deactivated Status - Active
- **Scenario**: Resolve active DID
- **Expected**: `deactivated: false`

### 6.5 Deactivated Status - Inactive
- **Scenario**: Resolve deactivated DID
- **Expected**: 
  - `deactivated: true`
  - DID Document still returned (for auditability)
  - Metadata includes deactivation timestamp

---

## 7. Complex Combined Scenarios

### 7.1 Fully Populated DID Document
- **Scenario**: DID with all possible fields populated
- **Test data**:
  - Multiple verification methods (both types)
  - All relationship types used
  - Multiple services (various endpoint types)
  - Multiple aliases
- **Expected**: Complete DID Document correctly resolved

### 7.2 Minimal DID Document
- **Scenario**: Newly created DID with no operations applied
- **Expected**: Only `id` and metadata fields present

### 7.3 Progressive Updates
- **Scenario**: Resolve DID at different version states
- **Test sequence**:
  1. Initial state (empty)
  2. After adding verification method
  3. After adding service
  4. After updating verification method
  5. After deactivation
- **Expected**: Each state correctly reflects applied operations

---

## 8. Error and Edge Cases

### 8.1 Non-existent DID
- **Scenario**: Attempt to resolve DID that doesn't exist
- **Expected**: `NotFound` error

### 8.2 Malformed Contract State
- **Scenario**: Contract state with invalid data
- **Expected**: Appropriate deserialization error

### 8.3 Network Mismatch
- **Scenario**: Request DID on wrong network
- **Expected**: `NotFound` or network error

### 8.4 Large Field Values
- **Scenario**: Test limits of field sizes
- **Test cases**:
  - Very long service endpoint URLs
  - Maximum length verification method IDs
  - Large number of aliases
- **Expected**: Correctly handles within protocol limits

---

## 9. State Consistency Tests

### 9.1 Verification Method Removal Cascade
- **Scenario**: Verify that removing a verification method removes all relationship references
- **Test data**: Remove `#key-1` that's in multiple relationships
- **Expected**: Method and all relationship references removed

### 9.2 Contract Version Field
- **Scenario**: Verify `contractVersion` field handling
- **Expected**: Version number correctly tracked (currently v1)

### 9.3 Operation Count Tracking
- **Scenario**: Verify internal operation counter
- **Expected**: Counter increments correctly (internal field, may not be exposed)

---

## 10. @context Field Tests

### 10.1 Standard Context
- **Scenario**: Verify correct @context URIs
- **Expected**:
  ```json
  [
    "https://www.w3.org/ns/did/v1",
    "https://w3c.github.io/vc-jws-2020/contexts/v1"
  ]
  ```

---

## Test Implementation Priorities

### High Priority (Core functionality):
- DID identifier tests (1.1, 1.2, 1.3)
- Verification method tests with both key types (2.1, 2.2, 2.3)
- Verification relationships (3.1, 3.2)
- Service endpoint variations (4.1, 4.2, 4.3, 4.4)
- Metadata tests (6.1, 6.2, 6.4, 6.5)
- Error cases (8.1, 8.2)

### Medium Priority (Edge cases and completeness):
- AlsoKnownAs tests (5.1-5.3)
- Multiple services (4.5)
- Complex combined scenarios (7.1, 7.2)
- ID format variations (2.4, 4.6)

### Lower Priority (Stress testing and limits):
- Maximum values tests (2.5, 5.4, 8.4)
- Progressive updates (7.3)
- State consistency (9.x)

---

## Test Data Requirements

For each test scenario, you'll need:
1. **Pre-computed contract state** (hex-encoded ledger state)
2. **Expected DID Document JSON** (for assertion)
3. **Expected metadata** (created, updated, versionId, deactivated)
4. **Test DID identifier** (valid Midnight DID)

The existing `serde_did_contract_v1.rs` test provides a good pattern to follow, using:
- Array of contract state hex strings
- Corresponding array of expected JSON outputs
- Deserialization and comparison logic

---

## Implementation Notes

### Contract State Structure (from did.compact)

The ledger state contains the following exported fields:
- `contractVersion`: Uint<32>
- `controllerPublicKey`: Bytes<32>
- `id`: ContractAddress
- `alsoKnownAs`: Set<Opaque<"string">>
- `version`: Counter
- `created`: Uint<64>
- `updated`: Uint<64>
- `deactivated`: Boolean
- `active`: Boolean
- `operationCount`: Counter
- `verificationMethods`: Map<Opaque<"string">, VerificationMethod>
- `authenticationRelation`: Set<Opaque<"string">>
- `assertionMethodRelation`: Set<Opaque<"string">>
- `keyAgreementRelation`: Set<Opaque<"string">>
- `capabilityInvocationRelation`: Set<Opaque<"string">>
- `capabilityDelegationRelation`: Set<Opaque<"string">>
- `services`: Map<Opaque<"string">, Service>

### Key Types Supported
1. **Ed25519**: OKP key type with Ed25519 curve (x parameter only)
2. **JubJub**: EC key type with Jubjub curve (x and y parameters)

### Service Endpoint Serialization
Services are stored with `serviceEndpoint` as a JSON string on-ledger, which must be rehydrated during resolution to support:
- Simple strings
- Arrays of strings
- Objects
- Arrays containing both strings and objects

---

## References

- **W3C DID Specification**: `tmp/midnight-did/w3c-spec/midnight-method.md`
- **Smart Contract**: `tmp/midnight-did/contract/src/did.compact`
- **Existing Tests**: `midnight-did-serde/tests/serde_did_contract_v1.rs`
- **Test Data**: `midnight-did-serde/tests/did-documents.json`

---

This test plan covers the resolver's responsibility: correctly deserializing contract state and reconstructing W3C-compliant DID Documents, with focus on field type variations and edge values rather than smart contract operation logic.
