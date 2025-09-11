use axum::Router;
use axum::routing::get;
use utoipa::OpenApi;

use crate::app::features::system_api::handlers::SystemOpenApiDoc;
use crate::app::{Routers, urls};

mod handlers;
mod models;

pub fn open_api() -> utoipa::openapi::OpenApi {
    SystemOpenApiDoc::openapi()
}

pub fn router() -> Routers {
    let app_router = Router::new()
        .route(urls::ApiHealth::AXUM_PATH, get(handlers::health))
        .route(urls::ApiAppMeta::AXUM_PATH, get(handlers::app_meta));

    Routers {
        app_router,
        ..Default::default()
    }
}
