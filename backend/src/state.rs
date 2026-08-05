//! Shared application state handed to every handler via `axum::extract::State`.

use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::config::Config;
use crate::db::Db;
use crate::util::hmac_sha256;

/// Where the browser session key is kept, inside the data directory.
const SESSION_KEY_FILE: &str = "session.key";

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    /// The value a browser session cookie must carry - never the configured
    /// token, so nothing durable enough to reach the API is left in a browser.
    ///
    /// Derived from the stored key *and* the token, so the two credentials are
    /// genuinely one secret: rotating `MAGPIE_TOKEN` changes this value and
    /// every outstanding cookie stops matching. Restarting on an unchanged
    /// token reproduces it exactly, so browsers stay signed in.
    pub session: Arc<String>,
}

impl AppState {
    pub fn new(db: Db, config: Config) -> Self {
        let key = load_or_create_session_key(&config.data_dir);
        let session = derive_session_value(&key, config.token.as_deref());
        AppState {
            db,
            config: Arc::new(config),
            session: Arc::new(session),
        }
    }
}

/// The cookie value for a given key and token.
///
/// Keyed on the stored secret and covering the token, so it cannot be computed
/// without the file and cannot survive the token changing. A missing token
/// still derives a value - auth is off in that case, so nothing consults it.
fn derive_session_value(key: &str, token: Option<&str>) -> String {
    let mac = hmac_sha256(key.as_bytes(), token.unwrap_or_default().as_bytes());
    URL_SAFE_NO_PAD.encode(mac)
}

/// 32 bytes from the OS, URL-safe so it needs no cookie quoting.
fn new_session_key() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS must provide randomness for session keys");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Read the session key, creating it on first run.
///
/// Kept next to the collection it guards, which is what makes persisting it
/// reasonable: anyone who can read this file can already read every highlight
/// sitting beside it, so it gives away nothing that wasn't already lost. It is
/// still written owner-only.
///
/// Any failure to read or write degrades to an in-memory key rather than
/// refusing to start - the collection is still perfectly servable, browsers
/// just get signed out on restart, and the warning says so.
fn load_or_create_session_key(data_dir: &Path) -> String {
    let path = data_dir.join(SESSION_KEY_FILE);

    match std::fs::read_to_string(&path) {
        Ok(stored) => {
            let key = stored.trim();
            if !key.is_empty() {
                return key.to_string();
            }
            tracing::warn!(path = %path.display(), "session key file is empty; writing a new one");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "cannot read the session key - browsers will be signed out on restart"
            );
            return new_session_key();
        }
    }

    let key = new_session_key();
    if let Err(e) = write_private(&path, &key) {
        tracing::warn!(
            error = %e,
            path = %path.display(),
            "cannot store the session key - browsers will be signed out on restart"
        );
    }
    key
}

/// Write owner-only where the platform has a notion of it.
///
/// Replaces rather than truncates, because only a *newly created* file can be
/// opened with its mode already set. Writing into an existing one would leave
/// the key readable at whatever the umask allows - usually world-readable -
/// for the moment between the write and the chmod.
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path)?;
    std::io::Write::write_all(&mut file, contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_keys_are_unpredictable() {
        let (a, b) = (new_session_key(), new_session_key());
        assert_ne!(a, b, "each start mints a distinct key");
        assert!(a.len() >= 43, "32 bytes of entropy survive the encoding");
    }

    #[test]
    fn the_session_key_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let first = load_or_create_session_key(dir.path());
        let second = load_or_create_session_key(dir.path());
        assert_eq!(first, second, "a restart keeps browsers signed in");

        // A different collection is a different session.
        let elsewhere = tempfile::tempdir().unwrap();
        assert_ne!(first, load_or_create_session_key(elsewhere.path()));
    }

    #[test]
    fn rotating_the_token_invalidates_outstanding_cookies() {
        let key = new_session_key();
        let before = derive_session_value(&key, Some("s3cret"));

        assert_eq!(
            before,
            derive_session_value(&key, Some("s3cret")),
            "an unchanged token keeps browsers signed in across a restart"
        );
        assert_ne!(
            before,
            derive_session_value(&key, Some("rotated")),
            "changing the token signs every browser out"
        );
        assert_ne!(
            before,
            derive_session_value(&new_session_key(), Some("s3cret")),
            "the stored key is still required to compute it"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_session_key_is_written_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        load_or_create_session_key(dir.path());
        let mode = std::fs::metadata(dir.path().join(SESSION_KEY_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn replacing_a_world_readable_key_file_tightens_it() {
        use std::os::unix::fs::PermissionsExt;

        // What an older Magpie, or a stray `echo >`, could have left behind.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SESSION_KEY_FILE);
        std::fs::write(&path, "").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        load_or_create_session_key(dir.path());

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "the empty file is replaced, not filled"
        );
    }

    #[test]
    fn an_unwritable_data_dir_still_yields_a_working_key() {
        // No such directory, so both the read and the write fail.
        let key = load_or_create_session_key(Path::new("/nonexistent/magpie-should-not-exist"));
        assert!(key.len() >= 43, "serving continues without persistence");
    }
}
