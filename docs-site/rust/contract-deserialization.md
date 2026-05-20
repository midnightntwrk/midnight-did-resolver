# Contract Deserialization

Contract deserialization is a critical part of the Midnight DID Resolver, enabling the system to interpret and process DID contract states stored on-chain.

## Implementation

The resolver uses a native Rust implementation for contract deserialization, located in the `midnight-did-serde` crate.
This implementation ports logic from the `compact-runtime` to Rust, enabling direct reading of bytes and conversion to compact types.

Each version of the `compact-runtime` is represented in Rust code (e.g., `compact_v0_9`), ensuring the contract's serialization format is versioned and compatible with deployed contracts.

The deserializer is implemented in `midnight-did-serde/src/serde_rs/` and used by the resolver through the `DefaultContractStateDeserializer` trait implementation.

## DID Contract Definition in Rust

The DID contract is implemented in pure Rust using macros, building on top of compact type deserialization.
The contract definition must match the Rust code structure to ensure correct deserialization.

Example contract macro:

```rust
compact_ledger!(DidContract {
    contract_version: cell<CompactTypeUnsignedInteger> [0, 0],
    controller_public_key: cell<CompactTypeBytes> [0, 1],
    id: cell<CompactTypeBytes> [1, 0],
    also_known_as: set<CompactTypeOpaqueString> [1, 1],
    version: cell<CompactTypeUnsignedInteger> [1, 2],
    created: cell<CompactTypeUnsignedInteger> [1, 3],
    updated: cell<CompactTypeUnsignedInteger> [1, 4],
    deactivated: cell<CompactTypeBoolean> [1, 5],
    active: cell<CompactTypeBoolean> [1, 6],
    operation_count: cell<CompactTypeUnsignedInteger> [1, 7],
    verification_methods: map<CompactTypeOpaqueString, VerificationMethod> [1, 8],
    authentication_relation: set<CompactTypeOpaqueString> [1, 9],
    assertion_method_relation: set<CompactTypeOpaqueString> [1, 10],
    key_agreement_relation: set<CompactTypeOpaqueString> [1, 11],
    capability_invocation_relation: set<CompactTypeOpaqueString> [1, 12],
    capability_delegation_relation: set<CompactTypeOpaqueString> [1, 13],
    services: map<CompactTypeOpaqueString, Service> [1, 14]
});
```

__Field Path Mapping__

Each field in the contract is associated with a path (e.g., `[1, 2]`), which specifies how to query the `StateValue` from the `ContractState`.
These paths are derived from the compiled compact JS code, from which query operations are defined.

Example JS query operation:

```js
Contract._query(context,
                partialProofData,
                [
                 { dup: { n: 0 } },
                 { idx: { cached: false,
                          pushPath: false,
                          path: [
                                 { tag: 'value',
                                   value: { value: _descriptor_28.toValue(1n),
                                            alignment: _descriptor_28.alignment() } },
                                 { tag: 'value',
                                   value: { value: _descriptor_28.toValue(2n),
                                            alignment: _descriptor_28.alignment() } }] } },
                 { popeq: { cached: false,
                            result: undefined } }]).value
```

This operation accesses `state_value[1][2]`, corresponding to the field's path in the contract definition.
