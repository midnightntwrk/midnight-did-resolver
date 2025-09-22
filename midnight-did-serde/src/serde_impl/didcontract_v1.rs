use midnight_ledger_v4::base_crypto::fab::Value;

use crate::serde_impl::compact_v0_8::*;

compact_ledger!(DidContract {
    contract_version: cell<CompactTypeUnsignedInteger> [0, 0],
    version: cell<CompactTypeUnsignedInteger> [1, 2],
    created_at: cell<CompactTypeBytes<64>> [1, 3],
    updated_at: cell<CompactTypeBytes<64>> [1, 4],
    deactivated_at: cell<CompactTypeBytes<64>> [1, 5],
    active: cell<CompactTypeBoolean> [1, 6],
    operation_count: cell<CompactTypeUnsignedInteger> [1, 7],
    verification_method: map<CompactTypeOpaqueString, VerificationMethod> [1, 8],
    authentication: set<CompactTypeOpaqueString> [1, 9],
    assertion_method: set<CompactTypeOpaqueString> [1, 10],
    key_agreement: set<CompactTypeOpaqueString> [1, 11],
    capability_invocation: set<CompactTypeOpaqueString> [1, 12],
    capability_delegation: set<CompactTypeOpaqueString> [1, 13],
    service: map<CompactTypeOpaqueString, AdtTypeSet<CompactTypeOpaqueString>> [1, 14]
});

compact_enum!(VerificationMethodType { Undefined, JsonWebKey });

compact_enum!(VerificationMethodRelation {
    Undefined,
    Authentication,
    AssertionMethod,
    KeyAgreement,
    CapabilityInvocation,
    CapabilityDelegation
});

compact_enum!(KeyType { EC, RSA, oct });

compact_enum!(CurveType { Ed25519, Jubjub });

compact_struct!(PublicKeyJwk {
    kty: KeyType,
    crv: CurveType,
    x: CompactTypeField,
    y: CompactTypeField
});

compact_struct!(VerificationMethod {
    id: CompactTypeOpaqueString,
    r#type: VerificationMethodType,
    public_key_jwk: PublicKeyJwk
});

compact_struct!(Service {
    id: CompactTypeOpaqueString,
    r#type: CompactTypeOpaqueString,
    service_endpoint: CompactTypeVector<4, CompactTypeOpaqueString>
});
