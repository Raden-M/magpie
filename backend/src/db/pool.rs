//! Connection pool and the `Db` handle shared across the app. rusqlite is
//! blocking, so every query runs inside `spawn_blocking` via [`Db::call`].

use std::path::{Path, PathBuf};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub type SqlitePool = Pool<SqliteConnectionManager>;

/// The application's data handle: a SQLite pool plus the data directory that
/// holds snapshot HTML and media blobs.
#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
    data_dir: PathBuf,
}

impl Db {
    /// Open the pool against `<data_dir>/magpie.db`, applying migrations. Creates
    /// the data directory and its `snapshots`/`media` subdirectories if missing;
    /// safe to call on an existing directory (idempotent init).
    pub fn open(data_dir: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(data_dir.join("snapshots"))?;
        std::fs::create_dir_all(data_dir.join("media"))?;

        let manager = SqliteConnectionManager::file(data_dir.join("magpie.db"))
            .with_init(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode = WAL;
                     PRAGMA foreign_keys = ON;
                     PRAGMA busy_timeout = 5000;
                     PRAGMA synchronous = NORMAL;",
                )
            });
        let pool = Pool::builder()
            .build(manager)
            .map_err(|e| AppError::Internal(e.into()))?;

        let mut conn = pool.get()?;
        super::migrations()
            .to_latest(&mut conn)
            .map_err(|e| AppError::Internal(e.into()))?;

        Ok(Db { pool, data_dir: data_dir.to_path_buf() })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn snapshot_path(&self, snapshot_id: &str) -> PathBuf {
        self.data_dir.join("snapshots").join(format!("{snapshot_id}.html"))
    }

    pub fn media_path(&self, hash: &str) -> PathBuf {
        self.data_dir.join("media").join(hash)
    }

    /// Run a blocking closure with a pooled connection on the blocking pool.
    pub async fn call<T, F>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&mut Connection) -> AppResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            f(&mut conn)
        })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db task panicked: {e}")))?
    }
}
