//! Local publication handoffs. Adapters transform content for a founder to publish.
mod article;
mod bundle;
mod platforms;

pub use article::{Article, ArticleRecord, Asset};
pub use bundle::Bundle;

use crate::{
    marketing,
    model::{Action, Attempt, BusinessError, Command, Workspace, next_id, now},
    storage::{BusinessStore, LOCAL_WORKSPACE_ID, StorageError, put_content, sha256},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    ffi::OsStr,
    fs, io,
    net::Ipv6Addr,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    Business(#[from] BusinessError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Could not resolve image {path}: {source}")]
    Image { path: String, source: io::Error },
}

pub type Result<T> = std::result::Result<T, PublishError>;

fn invalid(message: impl Into<String>) -> PublishError {
    BusinessError::Invalid(message.into()).into()
}

/// Target-specific metadata and Markdown transformations for a manual handoff.
pub trait PublisherAdapter {
    fn id(&self) -> &str;
    fn creator_url(&self) -> Option<&str>;
    fn normalize(&self, article: &Article) -> Result<PlatformArticle>;

    fn tags_are_topic_suggestions(&self) -> bool {
        false
    }

    fn checklist_steps(&self) -> &[&str] {
        &[]
    }
}

#[derive(Debug)]
pub struct PlatformArticle {
    pub title: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub markdown: String,
}

pub struct GenericAdapter;

impl PublisherAdapter for GenericAdapter {
    fn id(&self) -> &str {
        "generic"
    }

    fn creator_url(&self) -> Option<&str> {
        None
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        let record = article.metadata();
        Ok(PlatformArticle {
            title: record.title.clone(),
            summary: record.summary.clone(),
            tags: record.tags.clone(),
            category: record.category.clone(),
            markdown: article.markdown().into(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackageReference {
    pub article_id: String,
    pub platform: String,
    pub digest: String,
}

pub struct PackageRequest<'a> {
    pub experiment_id: &'a str,
    pub version: u32,
    pub article: &'a OsStr,
    pub platform: &'a str,
    pub out: &'a Path,
}

pub struct Packaged {
    pub attempt_id: String,
    pub digest: String,
    pub path: PathBuf,
}

pub async fn import_article(
    store: &BusinessStore,
    data_dir: &Path,
    path: &Path,
) -> Result<ArticleRecord> {
    let article = Article::read(path)?;
    let mut workspace = store.load_workspace(LOCAL_WORKSPACE_ID).await?;
    if save_article(&mut workspace, data_dir, &article)? {
        commit(store, &mut workspace).await?;
    }
    Ok(article.record)
}

pub async fn package(
    store: &BusinessStore,
    data_dir: &Path,
    request: PackageRequest<'_>,
) -> Result<Packaged> {
    let adapter = platforms::lookup(request.platform)?;
    let mut workspace = store.load_workspace(LOCAL_WORKSPACE_ID).await?;
    let experiment =
        marketing::authorized_preparation(&workspace, request.experiment_id, request.version)?;
    let article = match workspace
        .articles
        .iter()
        .find(|a| request.article == OsStr::new(&a.id))
    {
        Some(record) => read_article(data_dir, record)?,
        None => Article::read(Path::new(request.article))?,
    };
    let bundle = Bundle::build(&article, adapter)?;
    // Stage the export before CAS, then make it visible only after approval and
    // the matching attempt are durable. Repeating the command reuses that attempt.
    let export = bundle.stage(request.out)?;
    let existing = package_attempt(
        &workspace,
        request.experiment_id,
        request.version,
        bundle.digest(),
    );
    let attempt_id = if let Some(attempt) = existing {
        attempt.id.clone()
    } else {
        let proposal = experiment.proposals.last().expect("approved proposal");
        let snapshot = json!({"experiment_id": experiment.id, "proposal": proposal});
        let id = next_id("attempt", workspace.attempts.len());
        let attempt = Attempt {
            id: id.clone(),
            kind: "publish_package".into(),
            experiment_id: Some(request.experiment_id.into()),
            proposal_version: Some(request.version),
            snapshot,
            brief_markdown: "Publication package prepared for the founder.\n".into(),
            execution_refs: vec![],
            returns: vec![],
            reviews: vec![],
            reconciliations: vec![],
            publish_package: Some(PackageReference {
                article_id: article.record.id.clone(),
                platform: request.platform.into(),
                digest: bundle.digest().into(),
            }),
            created_at: now(),
        };
        save_article(&mut workspace, data_dir, &article)?;
        put_content(data_dir, &bundle.canonical)?;
        workspace.attempts.push(attempt);
        commit(store, &mut workspace).await?;
        id
    };
    Ok(Packaged {
        attempt_id,
        digest: bundle.digest().into(),
        path: export.finish()?,
    })
}

pub async fn record(
    store: &BusinessStore,
    experiment_id: &str,
    version: u32,
    package_digest: &str,
    url: &str,
) -> Result<Action> {
    if !https_url(url) {
        return Err(invalid("Publication requires an absolute HTTPS URL"));
    }
    if !is_digest(package_digest) {
        return Err(invalid(
            "Package digest must be a lowercase SHA-256 hex string",
        ));
    }
    let mut workspace = store.load_workspace(LOCAL_WORKSPACE_ID).await?;
    let attempt_id = package_attempt(&workspace, experiment_id, version, package_digest)
        .ok_or_else(|| BusinessError::NotFound("No package for this proposal version".into()))?
        .id
        .clone();
    let index = workspace
        .experiments
        .iter()
        .position(|experiment| experiment.id == experiment_id)
        .ok_or_else(|| BusinessError::NotFound("Experiment not found".into()))?;
    if let Some(action) = workspace.experiments[index]
        .actions
        .iter()
        .find(|action| action.publish_attempt_id.as_deref() == Some(attempt_id.as_str()))
    {
        if action.reference == url {
            return Ok(action.clone());
        }
        return Err(BusinessError::Conflict("Package already has a recorded URL".into()).into());
    }
    marketing::apply(
        &mut workspace,
        Command::RecordAction {
            experiment_id: experiment_id.into(),
            version,
            reference: url.into(),
            note: format!("Founder-supplied publication URL for package SHA-256 {package_digest}"),
            self_prepared: true,
        },
    )?;
    let action = workspace.experiments[index]
        .actions
        .last_mut()
        .expect("recorded action");
    action.publish_attempt_id = Some(attempt_id);
    let action = action.clone();
    commit(store, &mut workspace).await?;
    Ok(action)
}

fn package_attempt<'a>(
    workspace: &'a Workspace,
    experiment_id: &str,
    version: u32,
    digest: &str,
) -> Option<&'a Attempt> {
    workspace.attempts.iter().find(|attempt| {
        attempt.kind == "publish_package"
            && attempt.experiment_id.as_deref() == Some(experiment_id)
            && attempt.proposal_version == Some(version)
            && attempt
                .publish_package
                .as_ref()
                .is_some_and(|package| package.digest == digest)
    })
}

fn save_article(workspace: &mut Workspace, data_dir: &Path, article: &Article) -> Result<bool> {
    if workspace.articles.iter().any(|a| a.id == article.record.id) {
        return Ok(false);
    }
    put_content(data_dir, &article.source)?;
    for bytes in &article.asset_bytes {
        put_content(data_dir, bytes)?;
    }
    workspace.articles.push(article.record.clone());
    Ok(true)
}

fn read_article(data_dir: &Path, record: &ArticleRecord) -> Result<Article> {
    let source = read_content(data_dir, &record.source_sha256)?;
    let article = Article::parse(source, |_, index| {
        let asset = record
            .assets
            .get(index)
            .ok_or_else(|| invalid("Imported article image metadata is incomplete"))?;
        read_content(data_dir, &asset.sha256)
    })?;
    if article.metadata() != record {
        return Err(invalid(
            "Imported article content does not match its metadata",
        ));
    }
    Ok(article)
}

fn read_content(data_dir: &Path, digest: &str) -> Result<Vec<u8>> {
    if !is_digest(digest) {
        return Err(invalid("Invalid stored content digest"));
    }
    let bytes = fs::read(data_dir.join("files").join(digest))?;
    if sha256(&bytes) != digest {
        return Err(invalid("Stored file does not match its SHA-256 digest"));
    }
    Ok(bytes)
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

async fn commit(store: &BusinessStore, workspace: &mut Workspace) -> Result<()> {
    let expected = workspace.revision;
    workspace.revision += 1;
    store.commit_workspace(expected, workspace).await?;
    Ok(())
}

fn https_url(url: &str) -> bool {
    if !url.starts_with("https://")
        || !url
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~:/?#[]@!$&'()*+,;=%".contains(&b))
        || url.matches('#').count() > 1
        || url.split('%').skip(1).any(|rest| {
            !rest
                .as_bytes()
                .get(..2)
                .is_some_and(|escape| escape.iter().all(u8::is_ascii_hexdigit))
        })
    {
        return false;
    }
    let (target, _) = url.split_once('#').unwrap_or((url, ""));
    let Ok(uri) = target.parse::<axum::http::Uri>() else {
        return false;
    };
    if uri.scheme_str() != Some("https") {
        return false;
    }
    let Some(authority) = uri.authority() else {
        return false;
    };
    let host = authority.host();
    let Some(port) = authority.as_str().strip_prefix(host) else {
        return false;
    };
    if !port.is_empty()
        && !port.strip_prefix(':').is_some_and(|port| {
            port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u16>().is_ok()
        })
    {
        return false;
    }
    if let Some(address) = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
    {
        return address.parse::<Ipv6Addr>().is_ok();
    }
    host.strip_suffix('.')
        .unwrap_or(host)
        .split('.')
        .all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::https_url;

    #[test]
    fn https_url_accepts_ipv6_and_percent_encoded_paths() {
        assert!(https_url("https://[2001:db8::1]:443/a?x=1#title"));
        assert!(https_url("https://example.com/%E4%B8%AD"));
        assert!(!https_url("https://[broken]/article"));
        assert!(!https_url("https://example.com:65536/article"));
    }
}
