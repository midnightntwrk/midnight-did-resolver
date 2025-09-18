use midnight_ledger_v4::base_crypto::fab::Value;

use crate::serde_impl::compact_v08::*;

macro_rules! compact_enum {
    ($name:ident { $($fields:ident),+ }) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone)]
        pub enum $name {
            $($fields),+
        }

        impl CompactType for $name {
            fn from_value(value: &mut Value) -> Result<Self, CompactError> {
                let variants = [
                    $(Self::$fields),+
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
        $($fields:ident: $tys:ty),+
    }) => {
        pub struct $name {
            $($fields: $tys),+
        }

        impl CompactType for $name {
            fn from_value(value: &mut Value) -> Result<Self, CompactError> {
                $(let $fields = <$tys as CompactType>::from_value(value)?;)+
                Ok(Self {
                    $($fields),*
                })
            }
        }
    };
}

macro_rules! ledger {
    ($name:ident {
        $($fields:ident: $tys:ty [$($path:literal),+]),+
    }) => {
        paste::paste! {
            $(
                #[allow(non_camel_case_types)]
                struct [<$name _ $fields:camel>];

                impl StateValuePath for [<$name _ $fields:camel>]{
                    fn state_value_path() -> Vec<u8> {
                        vec![$($path),+]
                    }
                }
            )+

            pub struct $name {
                $($fields: LedgerTypeCell<$tys, [<$name _ $fields:camel>]>),+
            }
        }
    }
}

ledger!(DidContract2 {
    contract_version: CompactTypeUnsignedInteger [0, 0],
    version: CompactTypeUnsignedInteger [1, 0]
});

pub struct DidContract {
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
