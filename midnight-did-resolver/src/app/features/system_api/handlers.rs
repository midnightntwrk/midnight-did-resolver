use axum::Json;
use utoipa::OpenApi;

use crate::VERSION;
use crate::app::features::system_api::models::AppMeta;
use crate::app::{oas_tags, urls};

#[derive(OpenApi)]
#[openapi(paths(health, app_meta))]
pub struct SystemOpenApiDoc;

#[utoipa::path(
    get,
    path = urls::ApiHealth::AXUM_PATH,
    tags = [oas_tags::SYSTEM],
    responses(
        (status = OK, description = "Healthy", body = String, example = "ok"),
    )
)]
pub async fn health() -> &'static str {
    "ok"
}

#[utoipa::path(
    get,
    path = urls::ApiAppMeta::AXUM_PATH,
    tags = [oas_tags::SYSTEM],
    responses((status = OK, description = "Healthy", body = AppMeta))
)]
pub async fn app_meta() -> Json<AppMeta> {
    Json(AppMeta {
        version: VERSION.to_string(),
    })
}
