use identus_did_core::{Did, DidDocument};
use midnight_did::did::MidnightDid;
use midnight_did::dlt::ContractStateDeserializer;
use midnight_ledger_v4::base_crypto::fab::Value;
use midnight_ledger_v4::onchain_runtime::state::ContractState;
use midnight_ledger_v4::serialize::{NetworkId, deserialize};
use midnight_ledger_v4::storage::DefaultDB;

use crate::serde_rs::compact_v0_8::*;

compact_ledger!(DidContract {
    contract_version: cell<CompactTypeUnsignedInteger> [0, 0],
    version: cell<CompactTypeUnsignedInteger> [1, 2],
    created_at: cell<CompactTypeBytes> [1, 3],
    updated_at: cell<CompactTypeBytes> [1, 4],
    deactivated_at: cell<CompactTypeBytes> [1, 5],
    active: cell<CompactTypeBoolean> [1, 6],
    operation_count: cell<CompactTypeUnsignedInteger> [1, 7],
    verification_method: map<CompactTypeOpaqueString, VerificationMethod> [1, 8],
    authentication: set<CompactTypeOpaqueString> [1, 9],
    assertion_method: set<CompactTypeOpaqueString> [1, 10],
    key_agreement: set<CompactTypeOpaqueString> [1, 11],
    capability_invocation: set<CompactTypeOpaqueString> [1, 12],
    capability_delegation: set<CompactTypeOpaqueString> [1, 13],
    service: map<CompactTypeOpaqueString, Service> [1, 14]
});

compact_enum!(VerificationMethodType {
    Undefined,
    JsonWebKey2020
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

impl From<PublicKeyJwk> for identus_apollo::jwk::Jwk {
    fn from(value: PublicKeyJwk) -> Self {
        Self {
            kty: value.kty.to_string(),
            crv: value.crv.to_string(),
            x: Some(value.x.0.0.into()),
            y: Some(value.y.0.0.into()),
        }
    }
}

impl VerificationMethod {
    fn to_did_core(self, controller: &Did) -> identus_did_core::VerificationMethod {
        identus_did_core::VerificationMethod {
            id: format!("{}#{}", controller, self.id.0),
            r#type: self.r#type.to_string(),
            controller: controller.to_string(),
            public_key_jwk: Some(self.public_key_jwk.into()),
        }
    }
}

impl Service {
    fn to_did_core(self, controller: &Did) -> identus_did_core::Service {
        let service_endpoints = self
            .service_endpoint
            .0
            .into_iter()
            .map(|i| identus_did_core::StringOrMap::Str(i.0))
            .collect();
        identus_did_core::Service {
            id: format!("{}#{}", controller, self.id.0),
            r#type: identus_did_core::ServiceType::Str(self.r#type.0),
            service_endpoint: identus_did_core::ServiceEndpoint::List(service_endpoints),
        }
    }
}

pub struct DidContractDeserializer;

impl ContractStateDeserializer for DidContractDeserializer {
    fn deserialize(
        &self,
        did: &MidnightDid,
        state: &midnight_did::dlt::ContractState,
    ) -> Result<DidDocument, Box<dyn std::error::Error + Send + Sync>> {
        let network_id = match did.network() {
            midnight_did::did::MidnightNetwork::Undeployed => NetworkId::Undeployed,
            midnight_did::did::MidnightNetwork::Devnet => NetworkId::DevNet,
            midnight_did::did::MidnightNetwork::Testnet => NetworkId::TestNet,
            midnight_did::did::MidnightNetwork::Mainnet => NetworkId::MainNet,
        };
        let bytes = state.inner().to_bytes();
        let contract_state: ContractState<DefaultDB> = deserialize(bytes.as_slice(), network_id)?;
        let state_value = contract_state.data;
        let did_contract = DidContract::from_state_value(&state_value)?;

        let did = did.to_did();
        let verification_relation = |keys: Vec<CompactTypeOpaqueString>| {
            Some(
                keys.into_iter()
                    .map(|pk| identus_did_core::VerificationMethodOrRef::Ref(format!("{}#{}", did, pk.0)))
                    .collect::<Vec<_>>(),
            )
        };

        Ok(DidDocument {
            context: vec![],
            id: did.clone(),
            verification_method: did_contract
                .verification_method
                .0
                .into_iter()
                .map(|(_, v)| v.to_did_core(&did))
                .collect(),
            authentication: verification_relation(did_contract.authentication.0),
            assertion_method: verification_relation(did_contract.assertion_method.0),
            key_agreement: verification_relation(did_contract.key_agreement.0),
            capability_invocation: verification_relation(did_contract.capability_invocation.0),
            capability_delegation: verification_relation(did_contract.capability_delegation.0),
            service: Some(
                did_contract
                    .service
                    .0
                    .into_iter()
                    .map(|(_, v)| v.to_did_core(&did))
                    .collect(),
            ),
        })
    }
}
