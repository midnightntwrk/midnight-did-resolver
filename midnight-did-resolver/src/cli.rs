use std::net::Ipv4Addr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the midnight resolver server.
    Serve(ServeArgs),
    /// Generate OpenAPI specification for the API.
    GenerateOpenapi(GenerateOpenApiArgs),
}

#[derive(Args)]
pub struct ServeArgs {
    #[clap(flatten)]
    pub server: ServerArgs,
    /// URL for the Midnight Indexer API (e.g. http://localhost:8088/api/v1/graphql)
    #[arg(long, env = "MIDNIGHT_INDEXER_URL")]
    pub indexer_url: String,
}

#[derive(Args)]
pub struct GenerateOpenApiArgs {
    /// Output file for the OpenAPI spec (stdout if not provided)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct ServerArgs {
    /// HTTP server binding address
    #[arg(long, env = "SERVER_ADDRESS", default_value = "0.0.0.0")]
    pub address: Ipv4Addr,
    /// HTTP server listening port
    #[arg(long, short, env = "SERVER_PORT", default_value_t = 8080)]
    pub port: u16,
    /// Enable permissive CORS (https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.permissive)
    #[arg(long, env = "SERVER_CORS_ENABLED")]
    pub cors_enabled: bool,
}
