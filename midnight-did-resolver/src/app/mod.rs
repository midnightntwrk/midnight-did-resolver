use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use identus_did_resolver_http::DidResolverStateDyn;
use utoipa::OpenApi;
use utoipa::openapi::Server;

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

pub fn router<H: IntoIterator<Item = S>, S: AsRef<str>>(hosts: H) -> Routers {
    let home_router = Routers {
        app_router: Router::new()
            .merge(SwaggerUi::new(urls::Swagger::AXUM_PATH).url("/api/openapi.json", open_api_custom_host(hosts)))
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

pub fn open_api_custom_host<H: IntoIterator<Item = S>, S: AsRef<str>>(hosts: H) -> utoipa::openapi::OpenApi {
    let mut servers = Vec::new();
    for host in hosts {
        let mut s = Server::default();
        s.url = host.as_ref().to_string();
        servers.push(s);
    }

    #[derive(OpenApi)]
    struct BaseOpenApiDoc;

    let mut openapi = BaseOpenApiDoc::openapi()
        .merge_from(features::system_api::open_api())
        .merge_from(features::resolver_api::open_api());

    openapi.servers = Some(servers);
    openapi
}

pub fn open_api() -> utoipa::openapi::OpenApi {
    let a = ["http://localhost:8080"];
    open_api_custom_host(&a)
}
