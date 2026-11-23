//! SQLite business store for a small workspace aggregate. Immutable business
//! revisions are inside it; this is not an event sourcing framework.
use crate::model::Workspace;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{str::FromStr, time::Duration};

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
