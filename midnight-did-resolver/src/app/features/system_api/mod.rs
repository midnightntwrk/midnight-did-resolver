use axum::Router;
use axum::routing::get;
use identus_did_resolver_http::{HttpBindingOptions, did_resolver_http_binding};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::app::features::system_api::handlers::SystemOpenApiDoc;
use crate::app::{Routers, oas_tags, urls};

mod handlers;
mod models;

pub fn open_api() -> utoipa::openapi::OpenApi {
    SystemOpenApiDoc::openapi()
}

pub fn router() -> Routers {
    let app_router = Router::new()
        .merge(SwaggerUi::new(urls::Swagger::AXUM_PATH).url("/api/openapi.json", open_api()))
        .route(urls::ApiHealth::AXUM_PATH, get(handlers::health))
        .route(urls::ApiAppMeta::AXUM_PATH, get(handlers::app_meta));

    let did_resolver_router = did_resolver_http_binding(urls::ApiDid::AXUM_PATH, Default::default()).router;

    Routers {
        app_router,
        did_resolver_router,
        ..Default::default()
    }
}
