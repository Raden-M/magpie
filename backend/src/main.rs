mod api;
mod config;
mod cursor;
mod db;
mod error;
mod reader;
mod state;
mod url_canon;
mod util;
mod web;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use clap::Parser;

use crate::config::Config;
use crate::db::Db;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();
    init_tracing(&config.log);

    let db = Db::open(&config.data_dir)?;
    let state = AppState { db, config: Arc::new(config.clone()) };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .merge(api::router(state));

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(bind = %config.bind, data_dir = %config.data_dir.display(), "magpie listening");
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
