//! Shared application state handed to every handler via `axum::extract::State`.

use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::config::Config;
use crate::db::Db;

/// Where the browser session key is kept, inside the data directory.
const SESSION_KEY_FILE: &str = "session.key";

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    /// The value a browser session cookie must carry - never the configured
    /// token, so nothing durable enough to reach the API is left in a browser.
    ///
    /// Persisted beside the collection, so restarting doesn't sign everyone out.
    pub session: Arc<String>,
}

impl AppState {
    pub fn new(db: Db, config: Config) -> Self {
        let session = load_or_create_session_key(&config.data_dir);
        AppState {
            db,
            config: Arc::new(config),
            session: Arc::new(session),
        }
    }
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
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    std::fs::write(path, contents)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
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

    #[test]
    fn an_unwritable_data_dir_still_yields_a_working_key() {
        // No such directory, so both the read and the write fail.
        let key = load_or_create_session_key(Path::new("/nonexistent/magpie-should-not-exist"));
        assert!(key.len() >= 43, "serving continues without persistence");
    }
}
