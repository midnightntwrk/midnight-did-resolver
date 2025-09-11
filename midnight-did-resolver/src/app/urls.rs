use lazybe::macros::typed_uri;

typed_uri!(Home, "");
typed_uri!(Swagger, "swagger-ui");
typed_uri!(ApiHealth, "api" / "_system" / "health");
typed_uri!(ApiAppMeta, "api" / "_system" / "metadata");
typed_uri!(ApiDid, "api" / "dids" / (did: String));
