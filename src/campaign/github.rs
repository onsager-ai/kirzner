// Portions ported from ArtiPub, Copyright (c) 2019 Crawlab Team.
// Licensed under the BSD-3-Clause License; see THIRD_PARTY_NOTICES.md.
use super::{ReleaseIntent, bounded_body, format::valid_hex, provider_client};
use percent_encoding::percent_decode_str;
use reqwest::{Client, ClientBuilder, Response, StatusCode, header};
use serde::Deserialize;
use serde_json::json;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use url::Url;

const READ_ERROR: &str = "GitHub release lookup could not be verified";

#[derive(Deserialize)]
struct Actor {
    login: String,
}

#[derive(Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    author: Actor,
    html_url: String,
}

pub(super) enum Lookup {
    Found { id: String, url: String },
    Absent,
    Blocked(String),
}

pub struct GitHubReleases {
    client: Client,
    token: Option<String>,
}

impl GitHubReleases {
    pub fn new(builder: ClientBuilder, token: Option<String>) -> Result<Self, String> {
        Ok(Self {
            client: provider_client(builder)
                .build()
                .map_err(|_| "Could not configure GitHub HTTPS client")?,
            token,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        Self::new(Client::builder(), std::env::var("GITHUB_TOKEN").ok())
    }

    fn url(segments: &[&str]) -> Url {
        let mut url = Url::parse("https://api.github.com/").expect("static API origin");
        url.path_segments_mut().expect("HTTPS URL").extend(segments);
        url
    }

    fn repo_url(intent: &ReleaseIntent, suffix: &[&str]) -> Url {
        let (owner, repo) = intent.repo.split_once('/').expect("validated repository");
        let mut url = Self::url(&["repos", owner, repo]);
        url.path_segments_mut().expect("HTTPS URL").extend(suffix);
        url
    }

    fn request(
        &self,
        url: Url,
        authenticated: bool,
        body: Option<serde_json::Value>,
    ) -> reqwest::RequestBuilder {
        let request = match body {
            Some(body) => self.client.post(url).json(&body),
            None => self.client.get(url),
        };
        let request = request
            .header(header::ACCEPT, "application/vnd.github+json")
            .header(header::USER_AGENT, "kirzner")
            .header("X-GitHub-Api-Version", "2022-11-28");
        match self.token.as_deref().filter(|_| authenticated) {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    pub(super) async fn lookup(&self, intent: &ReleaseIntent) -> Lookup {
        match self.lookup_release(intent).await {
            Ok(result) => result,
            Err(reason) => Lookup::Blocked(reason),
        }
    }

    async fn lookup_release(&self, intent: &ReleaseIntent) -> Result<Lookup, String> {
        let response = self
            .request(
                Self::repo_url(intent, &["releases", "tags", &intent.tag]),
                false,
                None,
            )
            .send()
            .await
            .map_err(|_| READ_ERROR)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Lookup::Absent);
        }
        if !response.status().is_success() {
            return Err(format!(
                "GitHub release lookup returned HTTP {}",
                response.status().as_u16()
            ));
        }
        let release: Release = read(response, 1024 * 1024).await.map_err(|_| READ_ERROR)?;
        let prefix = format!("/{}/releases/tag/", intent.repo);
        let url = Url::parse(&release.html_url).map_err(|_| READ_ERROR)?;
        let path = percent_decode_str(url.path())
            .decode_utf8()
            .map_err(|_| READ_ERROR)?;
        if release.id == 0
            || release.tag_name != intent.tag
            || release.name.as_deref() != Some(intent.title.as_str())
            || release.body.as_deref() != Some(intent.body.as_str())
            || release.draft
            || release.prerelease
            || !release
                .published_at
                .as_deref()
                .is_some_and(|stamp| OffsetDateTime::parse(stamp, &Rfc3339).is_ok())
            || !release.author.login.eq_ignore_ascii_case(&intent.account)
            || url.origin().ascii_serialization() != "https://github.com"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !path
                .get(..prefix.len())
                .is_some_and(|path| path.eq_ignore_ascii_case(&prefix))
            || path.get(prefix.len()..) != Some(intent.tag.as_str())
        {
            return Err("The release at this tag differs from the planned release".into());
        }
        if let Some(commit) = &intent.commit
            && self
                .tag_commit(intent)
                .await
                .map_err(|_| READ_ERROR)?
                .as_ref()
                != Some(commit)
        {
            return Err("The tag does not point to the expected commit".into());
        }
        Ok(Lookup::Found {
            id: release.id.to_string(),
            url: release.html_url,
        })
    }

    async fn tag_commit(&self, intent: &ReleaseIntent) -> Result<Option<String>, ()> {
        #[derive(Deserialize)]
        struct Commit {
            sha: String,
        }
        let tag = format!("tags/{}", intent.tag);
        let response = self
            .request(Self::repo_url(intent, &["commits", &tag]), false, None)
            .send()
            .await
            .map_err(|_| ())?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let commit: Commit = read(response, 1024 * 1024).await?;
        Ok(valid_hex(&commit.sha, 40).then_some(commit.sha))
    }

    pub(super) async fn preflight(&self, intent: &ReleaseIntent) -> Result<(), String> {
        const ERROR: &str = "GitHub account or tag could not be verified";
        if self.token.as_deref().is_none_or(str::is_empty) {
            return Err("Set GITHUB_TOKEN for the approved GitHub account".into());
        }
        let response = self
            .request(Self::url(&["user"]), true, None)
            .send()
            .await
            .map_err(|_| ERROR)?;
        if !response.status().is_success() {
            return Err(format!(
                "GitHub account lookup returned HTTP {}",
                response.status().as_u16()
            ));
        }
        let actor: Actor = read(response, 1024 * 1024).await.map_err(|_| ERROR)?;
        if !actor.login.eq_ignore_ascii_case(&intent.account) {
            return Err("GITHUB_TOKEN belongs to a different GitHub account".into());
        }
        let Some(commit) = self.tag_commit(intent).await.map_err(|_| ERROR)? else {
            return Err("The tag must already resolve to a commit in the public repository".into());
        };
        if intent
            .commit
            .as_ref()
            .is_some_and(|expected| *expected != commit)
        {
            return Err("The tag does not point to the expected commit".into());
        }

        #[derive(Deserialize)]
        struct ListedRelease {
            tag_name: String,
            draft: bool,
        }
        for page in 1u64.. {
            let mut url = Self::repo_url(intent, &["releases"]);
            url.query_pairs_mut()
                .append_pair("per_page", "100")
                .append_pair("page", &page.to_string());
            let response = self
                .request(url, true, None)
                .send()
                .await
                .map_err(|_| ERROR)?;
            if !response.status().is_success() {
                return Err(format!(
                    "GitHub release list returned HTTP {}",
                    response.status().as_u16()
                ));
            }
            let next = response
                .headers()
                .get(header::LINK)
                .and_then(|link| link.to_str().ok())
                .is_some_and(|link| link.contains("rel=\"next\""));
            let releases: Vec<ListedRelease> =
                read(response, 16 * 1024 * 1024).await.map_err(|_| ERROR)?;
            if releases
                .iter()
                .any(|release| release.draft && release.tag_name == intent.tag)
            {
                return Err(
                    "A draft release already exists for this tag. Review it in GitHub.".into(),
                );
            }
            if !next {
                break;
            }
        }
        Ok(())
    }

    // Both an accepted receipt and an ambiguous write require public readback.
    pub(super) async fn create(&self, intent: &ReleaseIntent) -> Result<(), String> {
        let result = self
            .request(
                Self::repo_url(intent, &["releases"]),
                true,
                Some(json!({
                    "tag_name": intent.tag,
                    "name": intent.title,
                    "body": intent.body,
                    "draft": false,
                    "prerelease": false,
                })),
            )
            .send()
            .await;
        if let Ok(response) = result
            && matches!(response.status().as_u16(), 400 | 401 | 403 | 404 | 429)
        {
            return Err(format!(
                "GitHub rejected release creation (HTTP {})",
                response.status().as_u16()
            ));
        }
        Ok(())
    }
}

async fn read<T: serde::de::DeserializeOwned>(response: Response, limit: usize) -> Result<T, ()> {
    let body = bounded_body(response, limit).await.map_err(|_| ())?;
    serde_json::from_slice(&body).map_err(|_| ())
}
