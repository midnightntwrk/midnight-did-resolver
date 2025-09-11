use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use identus_did_resolver_http::DidResolverStateDyn;
use utoipa::OpenApi;

mod features;
mod urls;

pub use features::resolver_api::service::ResolverService;
use utoipa_swagger_ui::SwaggerUi;

mod oas_tags {
    pub const SYSTEM: &str = "System API";
    pub const RESOLVER: &str = "Resolver API";
}

/// Aggregator of Router from various features
#[derive(Default)]
pub struct Routers {
    pub app_router: Router<()>,
    pub did_resolver_router: Router<DidResolverStateDyn>,
}

impl Routers {
    pub fn merge(self, other: Routers) -> Self {
        Self {
            app_router: self.app_router.merge(other.app_router),
            did_resolver_router: self.did_resolver_router.merge(other.did_resolver_router),
        }
    }
}

pub fn router() -> Routers {
    let home_router = Routers {
        app_router: Router::new()
            .merge(SwaggerUi::new(urls::Swagger::AXUM_PATH).url("/api/openapi.json", open_api()))
            .route(
                urls::Home::AXUM_PATH,
                get(Redirect::temporary(&urls::Swagger::new_uri())),
            ),
        ..Default::default()
    };
    let system_api_router = features::system_api::router();
    let resolver_api_router = features::resolver_api::router();

    home_router.merge(system_api_router).merge(resolver_api_router)
}

pub fn open_api() -> utoipa::openapi::OpenApi {
    #[derive(OpenApi)]
    #[openapi(servers(
        (url = "http://localhost:8080", description = "Local"),
    ))]
    struct BaseOpenApiDoc;

    BaseOpenApiDoc::openapi()
        .merge_from(features::system_api::open_api())
        .merge_from(features::resolver_api::open_api())
}
