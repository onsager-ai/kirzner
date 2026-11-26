//! SQLite business store for a small workspace aggregate. Immutable business
//! revisions are inside it; this is not an event sourcing framework.
use crate::model::Workspace;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    path::Path,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) const LOCAL_WORKSPACE_ID: &str = "local";

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("Stored business data is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Workspace changed. Refresh before submitting again.")]
    StaleRevision,
}
pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone)]
pub struct BusinessStore {
    pool: SqlitePool,
}

impl BusinessStore {
    pub async fn connect(url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(10));
        let pool = SqlitePoolOptions::new()
            .max_connections(if url.contains(":memory:") { 1 } else { 4 })
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations/sqlite").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn load_workspace(&self, id: &str) -> Result<Workspace> {
        let initial = serde_json::to_string(&Workspace::empty(id))?;
        sqlx::query("INSERT INTO business_workspaces (workspace_id, revision, document) VALUES (?, 0, ?) ON CONFLICT(workspace_id) DO NOTHING")
            .bind(id).bind(initial).execute(&self.pool).await?;
        let document: String =
            sqlx::query_scalar("SELECT document FROM business_workspaces WHERE workspace_id = ?")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(serde_json::from_str(&document)?)
    }

    pub async fn commit_workspace(&self, expected: i64, workspace: &Workspace) -> Result<()> {
        if workspace.revision != expected + 1 {
            return Err(StorageError::StaleRevision);
        }
        let document = serde_json::to_string(workspace)?;
        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query("UPDATE business_workspaces SET revision = ?, document = ? WHERE workspace_id = ? AND revision = ?")
            .bind(workspace.revision).bind(document).bind(&workspace.id).bind(expected)
            .execute(&mut *transaction).await?;
        if result.rows_affected() != 1 {
            return Err(StorageError::StaleRevision);
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub fn backend(&self) -> &'static str {
        "sqlite"
    }
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn put_content(data_dir: &Path, bytes: &[u8]) -> io::Result<String> {
    let digest = sha256(bytes);
    let directory = data_dir.join("files");
    fs::create_dir_all(&directory)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = directory.join(format!(".upload-{}-{nonce}", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    // An interrupted write cannot truncate already referenced content.
    let persist = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temporary, directory.join(&digest)));
    if persist.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    persist.map(|()| digest)
}
