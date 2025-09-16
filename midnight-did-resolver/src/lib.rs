use std::fs;
use std::sync::Arc;

use axum::Router;
use clap::Parser;
use cli::Cli;
use identus_did_resolver_http::DidResolverStateDyn;
use midnight_did_indexer_client::MidnightIndexerClient;
use midnight_did_serde::CliContractStateDeserializer;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app::ResolverService;
use crate::cli::ServeArgs;

mod app;
mod cli;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run_command() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        cli::Command::Serve(args) => run_serve_command(args).await?,
        cli::Command::GenerateOpenapi(args) => generate_openapi(args)?,
    };
    Ok(())
}

fn generate_openapi(args: crate::cli::GenerateOpenApiArgs) -> anyhow::Result<()> {
    let oas = crate::app::open_api();
    let openapi_json = oas.to_pretty_json()?;

    if let Some(path) = args.output {
        fs::write(path, &openapi_json)?;
    } else {
        println!("{openapi_json}");
    }
    Ok(())
}

async fn run_serve_command(args: ServeArgs) -> anyhow::Result<()> {
    let layer = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .option_layer(Some(CorsLayer::permissive()).filter(|_| args.server.cors_enabled));

    let resolver_service = ResolverService::new(
        MidnightIndexerClient::new(&args.indexer_url),
        Arc::new(CliContractStateDeserializer::new("midnight-did-serde-js")),
    );
    let did_resolver_state = DidResolverStateDyn {
        resolver: Arc::new(resolver_service),
    };

    let routers = app::router();
    let router = Router::new()
        .merge(routers.app_router)
        .merge(routers.did_resolver_router.with_state(did_resolver_state))
        .layer(layer);
    let bind_addr = format!("{}:{}", args.server.address, args.server.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Server is listening on {}", bind_addr);
    axum::serve(listener, router).await?;
    Ok(())
}
