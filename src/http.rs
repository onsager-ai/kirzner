use crate::{
    handoffs, marketing,
    model::{BusinessError, Mutation, Workspace},
    semon::SemonConnection,
    storage::{BusinessStore, StorageError},
};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, Request, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub store: BusinessStore,
    pub workspace_id: String,
    pub data_dir: PathBuf,
    pub semon: SemonConnection,
    pub frontend_dir: PathBuf,
}
impl AppState {
    pub fn new(store: BusinessStore, data_dir: PathBuf) -> Self {
        Self {
            store,
            workspace_id: "local".into(),
            semon: SemonConnection::from_env(&data_dir),
            data_dir,
            frontend_dir: "frontend/dist".into(),
        }
    }
}

#[derive(Debug)]
struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}
impl From<BusinessError> for ApiError {
    fn from(error: BusinessError) -> Self {
        let status = match error {
            BusinessError::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            BusinessError::Conflict(_) => StatusCode::CONFLICT,
            BusinessError::NotFound(_) => StatusCode::NOT_FOUND,
        };
        Self(status, error.to_string())
    }
}
impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::StaleRevision => Self(StatusCode::CONFLICT, error.to_string()),
            // Database errors may contain a connection string or query data.
            _ => Self(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database operation failed; business data has not been overwritten".into(),
            ),
        }
    }
}

pub fn router(mut state: AppState, frontend: PathBuf) -> Router {
    state.frontend_dir = frontend;
    Router::new()
        .route("/api/workspace", get(workspace).post(mutate))
        .route("/api/health", get(health))
        .route("/api/semon", get(semon_status))
        .route("/api/handoffs/{id}", get(handoff))
        .route("/api/handoffs/{id}/continuation", get(continuation))
        .route("/api/files", post(upload))
        .route("/files/{id}", get(download))
        .route("/assets/{file}", get(asset))
        .route("/", get(index))
        .route("/opportunities", get(index))
        .route("/experiments", get(index))
        .route("/handoffs", get(index))
        .route("/results", get(index))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024))
        .layer(middleware::from_fn(local_boundary))
        .with_state(Arc::new(state))
}

async fn index(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    let bytes = std::fs::read(state.frontend_dir.join("index.html")).map_err(|_| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "Build the frontend with npm --prefix frontend run build".into(),
        )
    })?;
    Ok(([(header::CONTENT_TYPE, "text/html; charset=utf-8")], bytes).into_response())
}
async fn asset(
    State(state): State<Arc<AppState>>,
    Path(file): Path<String>,
) -> Result<Response, ApiError> {
    if file.starts_with('.')
        || !file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "Asset not found".into()));
    }
    let media_type = if file.ends_with(".js") {
        "application/javascript"
    } else if file.ends_with(".css") {
        "text/css"
    } else {
        "application/octet-stream"
    };
    let bytes = std::fs::read(state.frontend_dir.join("assets").join(file))
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "Asset not found".into()))?;
    Ok(([(header::CONTENT_TYPE, media_type)], bytes).into_response())
}

async fn local_boundary(request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let local = host
        .split(':')
        .next()
        .is_some_and(|h| h == "127.0.0.1" || h == "localhost");
    if !local {
        return ApiError(
            StatusCode::FORBIDDEN,
            "Use the loopback address shown by the local application".into(),
        )
        .into_response();
    }
    if let Some(origin) = request.headers().get(header::ORIGIN) {
        let allowed = origin.to_str().is_ok_and(|o| o == format!("http://{host}"));
        if !allowed {
            return ApiError(
                StatusCode::FORBIDDEN,
                "Cross-origin requests are not permitted".into(),
            )
            .into_response();
        }
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    headers.insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'".parse().unwrap());
    response
}

async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({"application": "kirzner", "backend": state.store.backend()}))
}
async fn workspace(State(state): State<Arc<AppState>>) -> Result<Json<Workspace>, ApiError> {
    Ok(Json(state.store.load_workspace(&state.workspace_id).await?))
}
async fn semon_status(State(state): State<Arc<AppState>>) -> Json<crate::semon::SemonStatus> {
    Json(state.semon.status().await)
}

async fn mutate(
    State(state): State<Arc<AppState>>,
    input: Result<Json<Mutation>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(input) = input.map_err(|error| ApiError(error.status(), error.body_text()))?;
    let mut workspace = state.store.load_workspace(&state.workspace_id).await?;
    if workspace.revision != input.expected_revision {
        return Err(StorageError::StaleRevision.into());
    }
    let result = marketing::apply(&mut workspace, input.command)?;
    if result.changed {
        workspace.revision += 1;
        state
            .store
            .commit_workspace(input.expected_revision, &workspace)
            .await?;
    }
    let status = if result.conflict {
        StatusCode::CONFLICT
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(
            json!({"workspace": workspace, "message": result.message, "conflict": result.conflict}),
        ),
    )
        .into_response())
}

async fn handoff(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let workspace = state.store.load_workspace(&state.workspace_id).await?;
    let attempt = workspace
        .attempts
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Handoff not found".into()))?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=kirzner-handoff.md",
            ),
        ],
        attempt.brief_markdown.clone(),
    )
        .into_response())
}
async fn continuation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let workspace = state.store.load_workspace(&state.workspace_id).await?;
    let attempt = workspace
        .attempts
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Handoff not found".into()))?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=kirzner-continuation.md",
            ),
        ],
        handoffs::continuation(attempt),
    )
        .into_response())
}

#[derive(Deserialize)]
struct UploadQuery {
    name: String,
}
async fn upload(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    if query.name.trim().is_empty() || bytes.is_empty() {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Choose a named, nonempty file (up to 5 MB)".into(),
        ));
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let directory = state.data_dir.join("files");
    std::fs::create_dir_all(&directory).map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not create file directory".into(),
        )
    })?;
    // An interrupted upload cannot truncate an already referenced content file.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = directory.join(format!(".upload-{}-{nonce}", std::process::id()));
    let persist = || -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, directory.join(&digest))?;
        Ok(())
    };
    if persist().is_err() {
        let _ = std::fs::remove_file(&temporary);
        return Err(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not persist file".into(),
        ));
    }
    let media_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("application/octet-stream");
    Ok(Json(
        json!({"title": query.name, "reference": format!("files/{digest}"), "media_type": media_type, "note": format!("SHA-256 {digest}; {} bytes", bytes.len())}),
    ))
}
async fn download(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    if id.len() != 64 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError(StatusCode::NOT_FOUND, "File not found".into()));
    }
    let bytes = std::fs::read(state.data_dir.join("files").join(&id)).map_err(|_| {
        ApiError(
            StatusCode::NOT_FOUND,
            "File not found; restore referenced files from your backup".into(),
        )
    })?;
    if format!("{:x}", Sha256::digest(&bytes)) != id {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "File content does not match its reference; restore it from backup".into(),
        ));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, "attachment"),
        ],
        bytes,
    )
        .into_response())
}
