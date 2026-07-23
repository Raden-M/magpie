mod config;
mod db;
mod error;
mod util;

use axum::Router;
use axum::routing::get;
use clap::Parser;

use crate::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();
    init_tracing(&config.log);

    let app = Router::new().route("/healthz", get(healthz));

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(bind = %config.bind, "magpie listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

fn init_tracing(level: &str) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("magpie={level},tower_http={level}")));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
}
