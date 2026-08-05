use clap::Parser;

use magpie::config::Config;
use magpie::db::Db;
use magpie::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();

    // Answered before anything is opened or bound, so it works against an
    // instance that is already running - which is the case that matters, since
    // the pairing string is usually wanted long after startup scrolled away.
    if config.connect {
        println!("{}", config.connect_string());
        return Ok(());
    }

    init_tracing(&config.log);
    check_startup_safety(&config)?;

    let db = Db::open(&config.data_dir)?;
    let state = AppState::new(db, config.clone());

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(bind = %config.bind, data_dir = %config.data_dir.display(), "magpie listening");
    if config.token.is_some() {
        // Pairing is the one step that needs a value from over here, so print it
        // in the form the options page accepts rather than making people
        // assemble a URL and a token by hand. Without a token there is nothing
        // to pair beyond the address already logged above.
        tracing::info!(
            "pair the extension by pasting this into its options: {} (or run `magpie --connect`)",
            config.connect_string()
        );
    }
    axum::serve(listener, magpie::app(state)).await?;
    Ok(())
}

/// Fail fast on foot-guns before opening the database (see `docs/usage.md`).
fn check_startup_safety(config: &Config) -> anyhow::Result<()> {
    config.validate()?;

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
