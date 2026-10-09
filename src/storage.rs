//! Concrete SQLite/PostgreSQL implementations of the same small business store.
//! One workspace aggregate keeps related decisions and returns atomic. Immutable
//! business revisions are inside it; this is not an event sourcing framework.
use crate::model::Workspace;
use sqlx::{
    PgPool, SqlitePool,
    postgres::PgPoolOptions,
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
    #[error("DATABASE_URL must use sqlite:, postgres:, or postgresql:")]
    Unsupported,
}
pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone)]
pub struct SqliteBusinessStore {
    pool: SqlitePool,
}
#[derive(Clone)]
pub struct PostgresBusinessStore {
    pool: PgPool,
}
#[derive(Clone)]
pub enum BusinessStore {
    Sqlite(SqliteBusinessStore),
    Postgres(PostgresBusinessStore),
}

macro_rules! implement_store {
    ($store:ty, $select:literal, $insert:literal, $update:literal) => {
        impl $store {
            pub async fn load_workspace(&self, id: &str) -> Result<Workspace> {
                let initial = serde_json::to_string(&Workspace::empty(id))?;
                sqlx::query($insert)
                    .bind(id)
                    .bind(initial)
                    .execute(&self.pool)
                    .await?;
                let document: String = sqlx::query_scalar($select)
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await?;
                Ok(serde_json::from_str(&document)?)
            }
            pub async fn commit_workspace(
                &self,
                expected: i64,
                workspace: &Workspace,
            ) -> Result<()> {
                if workspace.revision != expected + 1 {
                    return Err(StorageError::StaleRevision);
                }
                let document = serde_json::to_string(workspace)?;
                let mut transaction = self.pool.begin().await?;
                let result = sqlx::query($update)
                    .bind(workspace.revision)
                    .bind(document)
                    .bind(&workspace.id)
                    .bind(expected)
                    .execute(&mut *transaction)
                    .await?;
                if result.rows_affected() != 1 {
                    return Err(StorageError::StaleRevision);
                }
                transaction.commit().await?;
                Ok(())
            }
            pub async fn close(&self) {
                self.pool.close().await;
            }
        }
    };
}
implement_store!(
    SqliteBusinessStore,
    "SELECT document FROM business_workspaces WHERE workspace_id = ?",
    "INSERT INTO business_workspaces (workspace_id, revision, document) VALUES (?, 0, ?) ON CONFLICT(workspace_id) DO NOTHING",
    "UPDATE business_workspaces SET revision = ?, document = ? WHERE workspace_id = ? AND revision = ?"
);
implement_store!(
    PostgresBusinessStore,
    "SELECT document FROM business_workspaces WHERE workspace_id = $1",
    "INSERT INTO business_workspaces (workspace_id, revision, document) VALUES ($1, 0, $2) ON CONFLICT(workspace_id) DO NOTHING",
    "UPDATE business_workspaces SET revision = $1, document = $2 WHERE workspace_id = $3 AND revision = $4"
);

impl BusinessStore {
    pub async fn connect(url: &str) -> Result<Self> {
        if url.starts_with("sqlite:") {
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
            Ok(Self::Sqlite(SqliteBusinessStore { pool }))
        } else if url.starts_with("postgres:") || url.starts_with("postgresql:") {
            let pool = PgPoolOptions::new().max_connections(4).connect(url).await?;
            sqlx::migrate!("./migrations/postgres").run(&pool).await?;
            Ok(Self::Postgres(PostgresBusinessStore { pool }))
        } else {
            Err(StorageError::Unsupported)
        }
    }
    pub async fn load_workspace(&self, id: &str) -> Result<Workspace> {
        match self {
            Self::Sqlite(s) => s.load_workspace(id).await,
            Self::Postgres(s) => s.load_workspace(id).await,
        }
    }
    pub async fn commit_workspace(&self, expected: i64, workspace: &Workspace) -> Result<()> {
        match self {
            Self::Sqlite(s) => s.commit_workspace(expected, workspace).await,
            Self::Postgres(s) => s.commit_workspace(expected, workspace).await,
        }
    }
    pub async fn close(&self) {
        match self {
            Self::Sqlite(s) => s.close().await,
            Self::Postgres(s) => s.close().await,
        }
    }
    pub fn backend(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "sqlite",
            Self::Postgres(_) => "postgres",
        }
    }
}
