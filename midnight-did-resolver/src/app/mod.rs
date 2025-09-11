use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use features::system_api;
use identus_did_resolver_http::DidResolverStateDyn;
use utoipa::OpenApi;

mod features;
mod urls;

mod oas_tags {
    pub const SYSTEM: &str = "System API";
    pub const OP_RESOLVER: &str = "Resolver API";
}

#[derive(Default)]
pub struct Routers {
    pub app_router: Router<()>,
    pub did_resolver_router: Router<DidResolverStateDyn>,
}

pub fn router() -> Routers {
    let api_router = system_api::router();

    let home_router = Router::new().route(
        urls::Home::AXUM_PATH,
        get(Redirect::temporary(&urls::Swagger::new_uri())),
    );

    Routers {
        app_router: api_router.app_router.merge(home_router),
        did_resolver_router: Router::new(),
    }
}

pub fn open_api() -> utoipa::openapi::OpenApi {
    #[derive(OpenApi)]
    #[openapi(servers(
        (url = "http://localhost:8080", description = "Local"),
    ))]
    struct BaseOpenApiDoc;

    BaseOpenApiDoc::openapi().merge_from(system_api::open_api())
}
