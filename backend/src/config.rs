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

    /// Comma-separated hostnames this instance answers to, beyond loopback,
    /// any IP address, and the host of `--public-url`.
    #[arg(long, env = "MAGPIE_ALLOWED_HOSTS", value_delimiter = ',')]
    pub allowed_hosts: Vec<String>,

    /// The URL this instance is reached at from outside, e.g.
    /// `https://magpie.example.com`. Only needed behind a reverse proxy: it is
    /// what the extension is told to pair with, and its host is allowed
    /// automatically.
    #[arg(long, env = "MAGPIE_PUBLIC_URL")]
    pub public_url: Option<String>,

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
        // The scheme has to be told to us, never guessed. Magpie only ever
        // speaks plain HTTP, so whether the outside world reaches it over TLS
        // is a fact about the proxy in front - and a pairing string with the
        // wrong scheme fails in the one place it is hardest to debug.
        let base = match self.public_base() {
            Some(base) => base,
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

    /// `public_url` with any trailing slash removed, if one is set.
    fn public_base(&self) -> Option<String> {
        let raw = self.public_url.as_deref()?.trim().trim_end_matches('/');
        (!raw.is_empty()).then(|| raw.to_string())
    }

    /// The hostname of `public_url`, in the form a `Host` header carries it -
    /// IPv6 bracketed, no port. This is the single place that name is derived,
    /// so the allowlist and the pairing string cannot disagree about it.
    pub fn public_url_host(&self) -> Option<String> {
        let parsed = url::Url::parse(self.public_base()?.as_str()).ok()?;
        match parsed.host()? {
            url::Host::Ipv6(addr) => Some(format!("[{addr}]")),
            other => Some(other.to_string()),
        }
    }

    /// Reject configuration that would only fail later, and further away.
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(raw) = self.public_url.as_deref().map(str::trim)
            && !raw.is_empty()
            && self.public_url_host().is_none()
        {
            anyhow::bail!(
                "MAGPIE_PUBLIC_URL is not a URL with a hostname: {raw:?} - \
                 it should look like https://magpie.example.com"
            );
        }
        Ok(())
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
            public_url: None,
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
    fn a_proxied_instance_pairs_on_the_public_url_it_was_given() {
        let mut c = config("0.0.0.0:8787");
        c.public_url = Some("https://magpie.example.com/".into());
        assert_eq!(c.connect_string(), "https://magpie.example.com#s3cret");
    }

    #[test]
    fn the_public_url_keeps_its_own_scheme_and_port() {
        // A plain-HTTP proxy on the LAN, which guessing https would have broken.
        let mut c = config("0.0.0.0:8787");
        c.public_url = Some("http://magpie.lan:9000".into());
        assert_eq!(c.connect_string(), "http://magpie.lan:9000#s3cret");
    }

    #[test]
    fn the_allowlist_alone_never_decides_the_pairing_scheme() {
        let mut c = config("0.0.0.0:8787");
        c.allowed_hosts = vec!["magpie.example.com".into()];
        assert_eq!(
            c.connect_string(),
            "http://127.0.0.1:8787#s3cret",
            "an allowed name is not evidence of how to reach it"
        );
    }

    #[test]
    fn the_public_host_is_derived_once_for_both_uses() {
        let mut c = config("0.0.0.0:8787");

        c.public_url = Some("https://magpie.example.com:8443/".into());
        assert_eq!(c.public_url_host().as_deref(), Some("magpie.example.com"));

        c.public_url = Some("http://[2001:db8::1]:8787".into());
        assert_eq!(c.public_url_host().as_deref(), Some("[2001:db8::1]"));
    }

    #[test]
    fn a_public_url_that_is_not_a_url_fails_at_startup() {
        let mut c = config("0.0.0.0:8787");

        c.public_url = Some("magpie.example.com".into());
        assert!(c.validate().is_err(), "a bare hostname names no scheme");

        c.public_url = Some("https://magpie.example.com".into());
        assert!(c.validate().is_ok());

        c.public_url = None;
        assert!(c.validate().is_ok(), "unset is the ordinary local case");
    }

    #[test]
    fn without_a_token_the_address_alone_is_the_pairing_string() {
        let mut c = config("127.0.0.1:8787");
        c.token = None;
        assert_eq!(c.connect_string(), "http://127.0.0.1:8787");
    }
}
