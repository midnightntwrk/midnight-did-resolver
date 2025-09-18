use midnight_ledger_v4::base_crypto::fab::Value;

use crate::serde_impl::compact_v081::*;

macro_rules! compact_enum {
    ($name:ident { $field1:ident $(, $fields:ident)* }) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone)]
        pub enum $name {
            $field1,
            $($fields,)*
        }

        impl CompactType for $name {
            fn from_value(value: &mut Value) -> Result<Self, CompactError> {
                let variants = [
                    Self::$field1,
                    $(Self::$fields,)*
                ];
                let idx = CompactTypeEnum::from_value(value)?.0;
                variants.get(idx as usize).cloned().ok_or(CompactError(
                    format!("exptected Enum[<=?] for type {}", stringify!($name)),
                ))
            }
        }
    };
}

macro_rules! compact_struct {
    ($name:ident {
        $field1:ident: $ty1:ty
        $(, $fields:ident: $tys:ty)*
    }) => {
        pub struct $name {
            $field1: $ty1
            $(, $fields: $tys)*
        }

        impl CompactType for $name {
            fn from_value(value: &mut Value) -> Result<Self, CompactError> {
                let $field1 = <$ty1 as CompactType>::from_value(value)?;
                $(let $fields = <$tys as CompactType>::from_value(value)?;)*
                Ok(Self {
                    $field1
                    $(, $fields)*
                })
            }
        }
    };
}

pub struct DidContract {
    id: CompactTypeBytes<32>,
    version: CompactTypeUnsignedInteger,
    active: CompactTypeBoolean,
    // authenticationRelation: Set<Opaque<"string">>;
    // assertionMethodRelation: Set<Opaque<"string">>;
    // keyAgreementRelation: Set<Opaque<"string">>;
    // capabilityInvocationRelation: Set<Opaque<"string">>;
    // capabilityDelegationRelation: Set<Opaque<"string">>;
    // services: Map<Opaque<"string">, Service>;
}

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
    publicKeyJwk: PublicKeyJwk
});

compact_struct!(Service {
    id: CompactTypeOpaqueString,
    r#type: CompactTypeOpaqueString,
    serviceEndpoint: CompactTypeVector<4, CompactTypeOpaqueString>
});
