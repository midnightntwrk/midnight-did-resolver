use std::str::FromStr;

use chrono::{DateTime, Timelike, Utc};
use identus_did_core::{Did, DidDocument, DidDocumentMetadata, Uri};
use midnight_did::did::MidnightDid;
use midnight_did::dlt::ContractStateDeserializer;
use midnight_ledger_v4::base_crypto::fab::Value;
use midnight_ledger_v4::onchain_runtime::state::ContractState;
use midnight_ledger_v4::serialize::{NetworkId, deserialize};
use midnight_ledger_v4::storage::DefaultDB;

use crate::serde_rs::compact_v0_9::*;

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

compact_enum!(VerificationMethodType { Undefined, JsonWebKey });

compact_enum!(KeyType { EC, RSA, oct, OKP });

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
    service_endpoint: CompactTypeOpaqueString
});

impl From<PublicKeyJwk> for identus_apollo::jwk::Jwk {
    fn from(value: PublicKeyJwk) -> Self {
        let to_base64 = |mut bytes: CompactTypeField| {
            bytes.0.reverse();
            if bytes.0.is_empty() { None } else { Some(bytes.0.into()) }
        };
        Self {
            kty: value.kty.to_string(),
            crv: value.crv.to_string(),
            x: to_base64(value.x),
            y: to_base64(value.y),
        }
    }
}

impl VerificationMethod {
    fn to_did_core(self, controller: &Did) -> identus_did_core::VerificationMethod {
        let id = format!("{}#{}", controller, self.id.0);
        identus_did_core::VerificationMethod {
            id,
            r#type: self.r#type.to_string(),
            controller: controller.to_string(),
            public_key_jwk: Some(self.public_key_jwk.into()),
        }
    }
}

impl Service {
    fn to_did_core(self, controller: &Did) -> identus_did_core::Service {
        identus_did_core::Service {
            id: format!("{}#{}", controller, self.id.0),
            r#type: parse_service_type(&self.r#type.0),
            service_endpoint: parse_service_endpoint(&self.service_endpoint.0),
        }
    }
}

pub struct DidContractDeserializer;

impl ContractStateDeserializer for DidContractDeserializer {
    fn deserialize(
        &self,
        did: &MidnightDid,
        state: &midnight_did::dlt::ContractState,
    ) -> Result<(DidDocumentMetadata, DidDocument), Box<dyn std::error::Error + Send + Sync>> {
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
                    .map(|pk| identus_did_core::VerificationMethodOrRef::Ref(format!("{}#{}", &did, &pk.0)))
                    .collect::<Vec<_>>(),
            )
        };

        let did_doc_metadata = DidDocumentMetadata {
            created: le_bytes_to_datetime(&did_contract.created.0.0),
            updated: le_bytes_to_datetime(&did_contract.updated.0.0),
            version_id: Some(le_bytes_to_u64(&did_contract.version.0.0).to_string()),
            deactivated: Some(did_contract.deactivated.0.0),
            ..Default::default()
        };

        let did_doc = DidDocument {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3c.github.io/vc-jws-2020/contexts/v1".to_string(),
            ],
            id: did.clone(),
            also_known_as: Some(
                did_contract
                    .also_known_as
                    .0
                    .into_iter()
                    .flat_map(|s| Uri::from_str(&s.0).ok())
                    .collect::<Vec<_>>(),
            ),
            verification_method: did_contract
                .verification_methods
                .0
                .into_iter()
                .map(|(_, v)| v.to_did_core(&did))
                .collect(),
            authentication: verification_relation(did_contract.authentication_relation.0),
            assertion_method: verification_relation(did_contract.assertion_method_relation.0),
            key_agreement: verification_relation(did_contract.key_agreement_relation.0),
            capability_invocation: verification_relation(did_contract.capability_invocation_relation.0),
            capability_delegation: verification_relation(did_contract.capability_delegation_relation.0),
            service: Some(
                did_contract
                    .services
                    .0
                    .into_iter()
                    .map(|(_, v)| v.to_did_core(&did))
                    .collect(),
            ),
        };

        Ok((did_doc_metadata, did_doc))
    }
}

fn le_bytes_to_u64(bytes: &[u8]) -> u64 {
    // Pad with zeros if bytes are less than 8
    let mut padded = [0u8; 8];
    let len = bytes.len().min(8);
    padded[..len].copy_from_slice(&bytes[..len]);
    u64::from_le_bytes(padded)
}

fn le_bytes_to_datetime(bytes: &[u8]) -> Option<DateTime<Utc>> {
    // Pad with zeros if bytes are less than 8
    let mut padded = [0u8; 8];
    let len = bytes.len().min(8);
    padded[..len].copy_from_slice(&bytes[..len]);
    let timestamp_millis = i64::from_le_bytes(padded);
    DateTime::from_timestamp_millis(timestamp_millis).and_then(|dt| dt.with_nanosecond(0))
}

fn parse_service_type(type_str: &str) -> identus_did_core::ServiceType {
    use identus_did_core::ServiceType;

    // Try parsing as JSON array first (most specific)
    if let Ok(list) = serde_json::from_str::<Vec<String>>(type_str) {
        return ServiceType::List(list);
    }

    // Fallback: treat as single string type
    ServiceType::Str(type_str.to_string())
}

fn parse_service_endpoint(endpoint_str: &str) -> identus_did_core::ServiceEndpoint {
    use identus_did_core::{ServiceEndpoint, StringOrMap};

    // Try parsing as JSON array first (most specific)
    if let Ok(array) = serde_json::from_str::<Vec<serde_json::Value>>(endpoint_str) {
        let parsed_items: Vec<StringOrMap> = array
            .into_iter()
            .map(|value| match value {
                serde_json::Value::String(s) => StringOrMap::Str(s),
                serde_json::Value::Object(map) => StringOrMap::Map(map),
                _ => {
                    // Fallback: convert any other type to string representation
                    StringOrMap::Str(value.to_string())
                }
            })
            .collect();
        return ServiceEndpoint::List(parsed_items);
    }

    // Try parsing as JSON object (medium specific)
    if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(endpoint_str) {
        return ServiceEndpoint::StrOrMap(StringOrMap::Map(map));
    }

    // Fallback: treat as plain string
    let endpoint = serde_json::from_str::<String>(endpoint_str).unwrap_or_default();
    ServiceEndpoint::StrOrMap(StringOrMap::Str(endpoint))
}
