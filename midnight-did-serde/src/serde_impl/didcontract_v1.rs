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
        $($field:ident: $adt:tt <$ty:ty $(, $ty2:ty)?> [$($path:literal),*]),+
    }) => {
        paste::paste! {
            $(
                #[allow(non_camel_case_types)]
                struct [<$name _ $field:camel>];

                impl StateValuePath for [<$name _ $field:camel>]{
                    fn state_value_path() -> &'static [u8] {
                        &[$($path),*]
                    }
                }
            )+

            pub struct $name {
                $($field: ledger!(@internal $adt <$ty $(, $ty2)?>, [<$name _ $field:camel>])),+
            }
        }
    };
    (@internal cell <$ty:ty>, $pty:ident) => {
        AdtTypeCell<$ty, $pty>
    };
    (@internal set <$ty:ty>, $pty:ident) => {
        AdtTypeSet<$ty, $pty>
    };
    (@internal map <$ty:ty, $ty2:ty>, $pty:ident) => {
        AdtTypeMap<$ty, $ty2, $pty>
    }
}

ledger!(DidContract {
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
