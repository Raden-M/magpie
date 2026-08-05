//! Shared application state handed to every handler via `axum::extract::State`.

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::config::Config;
use crate::db::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    /// The value a browser session cookie must carry. Minted fresh at startup
    /// and never persisted, so restarting signs every browser out - and, unlike
    /// handing the browser the configured token, a stolen cookie expires with
    /// the process instead of granting the API forever.
    pub session: Arc<String>,
}

impl AppState {
    pub fn new(db: Db, config: Config) -> Self {
        AppState {
            db,
            config: Arc::new(config),
            session: Arc::new(new_session_key()),
        }
    }
}

/// 32 bytes from the OS, URL-safe so it needs no cookie quoting.
fn new_session_key() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS must provide randomness for session keys");
    URL_SAFE_NO_PAD.encode(bytes)
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
}
