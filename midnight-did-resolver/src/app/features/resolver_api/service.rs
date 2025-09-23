use std::str::FromStr;
use std::sync::Arc;

use identus_did_core::{
    Did, DidDocument, DidDocumentMetadata, DidResolutionError, DidResolutionErrorCode, DidResolutionMetadata,
    DidResolver, ResolutionOptions, ResolutionResult,
};
use midnight_did::did::MidnightDid;
use midnight_did::dlt::ContractStateDeserializer;
use midnight_did_indexer_client::{Error as IndexerClientError, MidnightIndexerClient};

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
enum ResolutionError {
    #[from]
    #[display("invalid did input")]
    InvalidDid { source: midnight_did::error::Error },
    #[display("did is not found")]
    NotFound,
    #[from]
    #[display("unexpected server error")]
    InternalError { source: anyhow::Error },
}

impl From<ResolutionError> for ResolutionResult {
    fn from(err: ResolutionError) -> Self {
        let error = match err {
            ResolutionError::InvalidDid { .. } => DidResolutionError {
                r#type: DidResolutionErrorCode::InvalidDid,
                title: Some("Invalid DID".to_string()),
                detail: Some(err.to_string()),
            },
            ResolutionError::NotFound => DidResolutionError {
                r#type: DidResolutionErrorCode::NotFound,
                title: Some("DID Not Found".to_string()),
                detail: Some(err.to_string()),
            },
            ResolutionError::InternalError { .. } => DidResolutionError {
                r#type: DidResolutionErrorCode::InternalError,
                title: Some("Internal Error".to_string()),
                detail: Some(err.to_string()),
            },
        };

        ResolutionResult {
            did_resolution_metadata: DidResolutionMetadata {
                error: Some(error),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct ResolverService {
    indexer_client: MidnightIndexerClient,
    state_deserializer: Arc<dyn ContractStateDeserializer + Send + Sync + 'static>,
}

impl ResolverService {
    pub fn new(
        indexer_client: MidnightIndexerClient,
        state_deserializer: Arc<dyn ContractStateDeserializer + Send + Sync + 'static>,
    ) -> Self {
        Self {
            indexer_client,
            state_deserializer,
        }
    }

    async fn resolution_logic(&self, did: &Did) -> Result<(DidDocumentMetadata, DidDocument), ResolutionError> {
        let did = match MidnightDid::from_str(&did.to_string()) {
            Ok(did) => did,
            Err(e) => Err(ResolutionError::InvalidDid { source: e })?,
        };
        let contract_state = match self.indexer_client.get_contract_state(&did).await {
            Ok(state) => state,
            Err(IndexerClientError::MissingDataFields { .. }) => Err(ResolutionError::NotFound)?,
            Err(e) => Err(anyhow::Error::from(e))?,
        };
        self.state_deserializer
            .deserialize(&did, &contract_state)
            .map_err(|e| ResolutionError::InternalError {
                source: anyhow::Error::from_boxed(e),
            })
    }
}

#[async_trait::async_trait]
impl DidResolver for ResolverService {
    async fn resolve(&self, did: &Did, _options: &ResolutionOptions) -> ResolutionResult {
        match self.resolution_logic(did).await {
            Ok((did_doc_metadata, did_doc)) => {
                let mut result = ResolutionResult::success(did_doc);
                result.did_document_metadata = did_doc_metadata;
                result
            }
            Err(e) => e.into(),
        }
    }
}
