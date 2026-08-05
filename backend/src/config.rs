use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

/// Runtime configuration, populated from CLI flags or environment variables.
///
/// Every value has a safe local-first default so `magpie` runs with no
/// configuration at all. See `docs/usage.md` for the remote story.
#[derive(Debug, Clone, Parser)]
#[command(name = "magpie", version, about = "Self-hosted highlight keeper")]
pub struct Config {
    /// Listen address. Binding to a non-loopback address requires a token.
    #[arg(long, env = "MAGPIE_BIND", default_value = "127.0.0.1:8787")]
    pub bind: SocketAddr,

    /// Directory holding the SQLite database, snapshots, and media.
    #[arg(long, env = "MAGPIE_DATA_DIR", default_value_os_t = default_data_dir())]
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

    /// Print the extension pairing string and exit, instead of serving.
    #[arg(long)]
    pub connect: bool,
}

/// Where the collection lives when nobody says otherwise.
///
/// Deliberately absolute. A relative default is resolved against the working
/// directory, so the same command run from two places would open two different
/// collections - and under a service manager, whose working directory is the
/// home directory, it would silently start an empty one. Falls back to `./data`
/// only if the platform has no data directory at all.
fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .map(|dir| dir.join("magpie"))
        .unwrap_or_else(|| PathBuf::from("./data"))
}

impl Config {
    /// Whether the configured bind address is a loopback interface.
    pub fn is_loopback(&self) -> bool {
        self.bind.ip().is_loopback()
    }

    /// The one-paste pairing string for the extension: the address to reach us
    /// at, with any token carried in the fragment. A fragment is never sent to
    /// a server, so pasting it back into a URL bar cannot leak the token.
    pub fn connect_string(&self) -> String {
        // Magpie never terminates TLS, so it only answers to https when
        // something sits in front of it - which is exactly the case where a
        // public hostname has been configured.
        let base = match self.public_host() {
            Some(host) => format!("https://{host}"),
            // A wildcard bind names no reachable address, and printing
            // `0.0.0.0` helps nobody. A container publishes to the host's
            // loopback, which is where whoever reads this will actually look.
            None if self.bind.ip().is_unspecified() => {
                format!("http://127.0.0.1:{}", self.bind.port())
            }
            None => format!("http://{}", self.bind),
        };

        match self.token.as_deref() {
            Some(token) => format!("{base}#{token}"),
            None => base,
        }
    }

    /// The first configured public hostname, if there is one.
    fn public_host(&self) -> Option<&str> {
        self.allowed_hosts
            .iter()
            .map(|h| h.trim())
            .find(|h| !h.is_empty())
    }

    /// Auth is enforced whenever a token is set, or the bind is non-loopback.
    pub fn auth_required(&self) -> bool {
        self.token.is_some() || !self.is_loopback()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(bind: &str) -> Config {
        Config {
            bind: bind.parse().unwrap(),
            data_dir: PathBuf::from("/tmp/magpie-test"),
            token: Some("s3cret".into()),
            allowed_origins: vec![],
            allowed_hosts: vec![],
            max_image_bytes: 5_000_000,
            log: "warn".into(),
            connect: false,
        }
    }

    #[test]
    fn a_local_instance_pairs_on_its_bind_address() {
        assert_eq!(
            config("127.0.0.1:8787").connect_string(),
            "http://127.0.0.1:8787#s3cret"
        );
    }

    #[test]
    fn a_wildcard_bind_pairs_on_loopback_over_plain_http() {
        // What a container sets. `0.0.0.0` reaches nothing, and Magpie never
        // terminates TLS, so neither that host nor https would be usable here.
        assert_eq!(
            config("0.0.0.0:8787").connect_string(),
            "http://127.0.0.1:8787#s3cret"
        );
    }

    #[test]
    fn a_proxied_instance_pairs_on_its_public_name_over_https() {
        let mut c = config("0.0.0.0:8787");
        c.allowed_hosts = vec!["magpie.example.com".into()];
        assert_eq!(c.connect_string(), "https://magpie.example.com#s3cret");
    }

    #[test]
    fn without_a_token_the_address_alone_is_the_pairing_string() {
        let mut c = config("127.0.0.1:8787");
        c.token = None;
        assert_eq!(c.connect_string(), "http://127.0.0.1:8787");
    }
}
