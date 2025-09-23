// TODO: make this dynamic
// For example: typed_uri!(ApiDid, "api" / "did" / (did: String))
macro_rules! typed_uri {
    ($name:ident, $path:literal) => {
        pub struct $name;
        impl $name {
            pub const AXUM_PATH: &str = $path;

            #[allow(unused)]
            pub fn new_uri() -> String {
                $path.to_string()
            }
        }
    };
}

typed_uri!(Home, "/");
typed_uri!(Swagger, "/swagger-ui");
typed_uri!(ApiHealth, "/api/_system/health");
typed_uri!(ApiAppMeta, "/api/_system/metadata");
typed_uri!(ApiDid, "/api/dids/{did}");
