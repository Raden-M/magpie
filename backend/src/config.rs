use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

/// Runtime configuration, populated from CLI flags or environment variables.
///
/// Every value has a safe local-first default so `magpie` runs with no
/// configuration at all. See `docs/08-deployment.md` for the remote story.
#[derive(Debug, Clone, Parser)]
#[command(name = "magpie", version, about = "Self-hosted highlight keeper")]
pub struct Config {
    /// Listen address. Binding to a non-loopback address requires a token.
    #[arg(long, env = "MAGPIE_BIND", default_value = "127.0.0.1:8787")]
    pub bind: SocketAddr,

    /// Directory holding the SQLite database, snapshots, and media.
    #[arg(long, env = "MAGPIE_DATA_DIR", default_value = "./data")]
    pub data_dir: PathBuf,

    /// Bearer token. Unset + loopback bind disables auth for local-only use.
    #[arg(long, env = "MAGPIE_TOKEN")]
    pub token: Option<String>,

    /// Comma-separated extension origins allowed via CORS.
    #[arg(long, env = "MAGPIE_ALLOWED_ORIGINS", value_delimiter = ',')]
    pub allowed_origins: Vec<String>,

    /// Comma-separated hostnames this instance answers to, beyond loopback and
    /// the bind address. Set this to the public name when behind a proxy.
    #[arg(long, env = "MAGPIE_ALLOWED_HOSTS", value_delimiter = ',')]
    pub allowed_hosts: Vec<String>,

    /// Per-image byte ceiling accepted at capture time.
    #[arg(long, env = "MAGPIE_MAX_IMAGE_BYTES", default_value_t = 5_000_000)]
    pub max_image_bytes: u64,

    /// Log level (`error`, `warn`, `info`, `debug`, `trace`).
    #[arg(long, env = "MAGPIE_LOG", default_value = "info")]
    pub log: String,
}

impl Config {
    /// Whether the configured bind address is a loopback interface.
    pub fn is_loopback(&self) -> bool {
        self.bind.ip().is_loopback()
    }

    /// Auth is enforced whenever a token is set, or the bind is non-loopback.
    pub fn auth_required(&self) -> bool {
        self.token.is_some() || !self.is_loopback()
    }
}
