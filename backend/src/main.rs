use clap::Parser;

use magpie::config::Config;
use magpie::db::Db;
use magpie::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();
    init_tracing(&config.log);
    check_startup_safety(&config)?;

    let db = Db::open(&config.data_dir)?;
    let state = AppState::new(db, config.clone());

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(bind = %config.bind, data_dir = %config.data_dir.display(), "magpie listening");
    if let Some(connect) = config.connect_string() {
        // Pairing is the one step that needs a value from over here, so print it
        // in the form the options page accepts rather than making people
        // assemble a URL and a token by hand.
        tracing::info!("pair the extension by pasting this into its options: {connect}");
    }
    axum::serve(listener, magpie::app(state)).await?;
    Ok(())
}

/// Fail fast on foot-guns before opening the database (docs/08-deployment.md).
fn check_startup_safety(config: &Config) -> anyhow::Result<()> {
    if !config.is_loopback() && config.token.is_none() {
        anyhow::bail!(
            "refusing to bind to non-loopback address {} without MAGPIE_TOKEN set - \
             a token is required whenever Magpie is reachable off this machine",
            config.bind
        );
    }

    std::fs::create_dir_all(&config.data_dir).map_err(|e| {
        anyhow::anyhow!(
            "data directory {} is not usable: {e}",
            config.data_dir.display()
        )
    })?;
    let probe = config.data_dir.join(".magpie-write-test");
    std::fs::write(&probe, b"ok").map_err(|e| {
        anyhow::anyhow!(
            "data directory {} is not writable: {e}",
            config.data_dir.display()
        )
    })?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

fn init_tracing(level: &str) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("magpie={level},tower_http={level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
