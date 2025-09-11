use identus_did_resolver_http::{HttpBindingOptions, did_resolver_http_binding};

use crate::app::{Routers, oas_tags, urls};

pub mod service;

pub fn open_api() -> utoipa::openapi::OpenApi {
    did_resolver_http_binding(
        urls::ApiDid::AXUM_PATH,
        HttpBindingOptions {
            openapi_tags: Some(vec![oas_tags::RESOLVER.to_string()]),
        },
    )
    .openapi
}

pub fn router() -> Routers {
    let did_resolver_router = did_resolver_http_binding(urls::ApiDid::AXUM_PATH, Default::default()).router;
    Routers {
        did_resolver_router,
        ..Default::default()
    }
}
