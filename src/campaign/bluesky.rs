// Portions ported from ArtiPub, Copyright (c) 2019 Crawlab Team.
// Licensed under the BSD-3-Clause License; see THIRD_PARTY_NOTICES.md.
use super::{
    BlueskyIntent, BodyError, approval::bluesky_handle, bounded_body, card, provider_client,
    timestamp,
};
use regex::Regex;
use reqwest::{Client, ClientBuilder, Proxy, Response, StatusCode, header};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::LazyLock;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use url::Url;

const PUBLIC_API: &str = "https://public.api.bsky.app";
const HEALTH: &str = "https://brittlegill.us-west.host.bsky.network/xrpc/_health";
const JSON_LIMIT: usize = 1024 * 1024;
const LINK: &str = "app.bsky.richtext.facet#link";

static LINKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:^|[\s(\[{<"'])((?i:https?)://[^\s<>"']+)"#).expect("static expression")
});

#[derive(Deserialize)]
struct Identity {
    did: String,
    handle: String,
}

#[derive(Deserialize)]
struct DidDocument {
    id: String,
    #[serde(rename = "alsoKnownAs")]
    also_known_as: Vec<String>,
    #[serde(default)]
    service: Vec<DidService>,
}

#[derive(Deserialize)]
struct DidService {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "serviceEndpoint")]
    endpoint: Value,
}

#[derive(Deserialize)]
struct Session {
    #[serde(flatten)]
    identity: Identity,
    #[serde(rename = "accessJwt")]
    access_jwt: String,
}

#[derive(Deserialize)]
pub(super) struct Receipt {
    pub(super) uri: String,
    pub(super) cid: String,
}

#[derive(Deserialize)]
struct RepoRecord {
    #[serde(flatten)]
    receipt: Receipt,
    value: Value,
}

#[derive(Deserialize)]
struct Author {
    did: String,
}

#[derive(Deserialize)]
struct PublicPost {
    #[serde(flatten)]
    receipt: Receipt,
    author: Author,
    record: Value,
}

#[derive(Deserialize)]
struct PublicPosts {
    posts: Vec<PublicPost>,
}

pub(super) struct Evidence {
    pub(super) receipt: Receipt,
    pub(super) url: String,
}

pub(super) enum Lookup {
    Missing,
    Conflict(String),
    Unverified(String),
    Pending(Evidence),
    UnverifiedView(Evidence, String),
    Public(Evidence),
}

pub(super) struct WriteError {
    pub(super) rejected: bool,
    pub(super) reason: String,
}

#[derive(Deserialize)]
struct XrpcError {
    error: String,
}

#[derive(Debug)]
enum ApiError {
    Transport(reqwest::Error),
    Body(StatusCode, BodyError),
    Json(StatusCode),
    Status(StatusCode, String),
    Invalid(StatusCode),
}

impl ApiError {
    fn reason(&self, step: &str, secrets: &[&str]) -> String {
        let (status, error, class, timeout) = match self {
            Self::Transport(error) => (
                None,
                None,
                if error.is_connect() {
                    "connect"
                } else {
                    "transport"
                },
                error.is_timeout(),
            ),
            Self::Body(status, BodyError::Request(error)) => {
                (Some(*status), None, "body", error.is_timeout())
            }
            Self::Body(status, BodyError::TooLarge) => {
                (Some(*status), None, "body_too_large", false)
            }
            Self::Json(status) => (Some(*status), None, "json", false),
            Self::Status(status, name) => {
                (Some(*status), safe_error_name(name, secrets), "xrpc", false)
            }
            Self::Invalid(status) => (Some(*status), None, "invalid_response", false),
        };
        let outcome = status.map_or_else(
            || "failed".to_owned(),
            |status| format!("returned HTTP {}", status.as_u16()),
        );
        let xrpc = error.map_or_else(String::new, |name| format!(", XRPC error={name}"));
        format!("Bluesky {step} {outcome} (class={class}, timeout={timeout}{xrpc})")
    }
}

fn safe_error_name(name: &str, secrets: &[&str]) -> Option<&'static str> {
    const KNOWN_NAMES: &[&str] = &[
        "AccountDeactivated",
        "AccountTakedown",
        "AuthRequired",
        "ExpiredToken",
        "InternalServerError",
        "InvalidRequest",
        "InvalidSwap",
        "InvalidToken",
        "MethodNotImplemented",
        "RecordNotFound",
        "RepoNotFound",
        "RateLimitExceeded",
        "UpstreamFailure",
    ];
    KNOWN_NAMES.iter().copied().find(|&known| {
        known == name
            && !secrets
                .iter()
                .any(|secret| !secret.is_empty() && known.contains(secret))
    })
}

pub struct Bluesky {
    client: Client,
    handle: Option<String>,
    password: Option<String>,
}

impl Bluesky {
    pub fn new(
        builder: ClientBuilder,
        proxy: Option<&str>,
        handle: Option<&str>,
        password: Option<String>,
    ) -> Result<Self, String> {
        let mut builder = provider_client(builder);
        if let Some(proxy) = proxy {
            builder =
                builder.proxy(Proxy::https(proxy).map_err(|_| "Invalid Bluesky HTTPS proxy")?);
        }
        Ok(Self {
            client: builder
                .build()
                .map_err(|_| "Could not configure Bluesky HTTPS client")?,
            handle: handle
                .map(|handle| handle.strip_prefix('@').unwrap_or(handle).to_lowercase())
                .filter(|handle| bluesky_handle(handle)),
            password,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        let proxy = std::env::var("HTTPS_PROXY")
            .or_else(|_| std::env::var("https_proxy"))
            .ok();
        let handle = std::env::var("BLUESKY_HANDLE").ok();
        Self::new(
            Client::builder(),
            proxy.as_deref(),
            handle.as_deref(),
            std::env::var("BLUESKY_APP_PASSWORD").ok(),
        )
    }

    pub async fn health(&self, url: Option<&str>) -> Result<u16, String> {
        let url = Url::parse(url.unwrap_or(HEALTH)).map_err(|_| "Invalid Bluesky health URL")?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err("Bluesky health requires an HTTPS URL without credentials".into());
        }
        self.client
            .get(url)
            .timeout(card::TIMEOUT)
            .send()
            .await
            .map(|response| response.status().as_u16())
            .map_err(|error| ApiError::Transport(error).reason("health", &[]))
    }

    async fn read<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<(StatusCode, T), ApiError> {
        let response = request.send().await.map_err(ApiError::Transport)?;
        read_response(response).await
    }

    fn xrpc(&self, origin: &str, method: &str) -> String {
        format!("{origin}/xrpc/{method}")
    }

    fn query(
        &self,
        origin: &str,
        method: &str,
        params: &[(&str, &str)],
    ) -> reqwest::RequestBuilder {
        self.client.get(self.xrpc(origin, method)).query(params)
    }

    fn diagnostic(&self, error: &ApiError, step: &str) -> String {
        error.reason(step, &[self.password.as_deref().unwrap_or("")])
    }

    pub(super) async fn resolve(
        &self,
        account: &str,
        text: &str,
        request_key: &str,
    ) -> Result<BlueskyIntent, String> {
        let (_, profile): (_, Identity) = self
            .read(self.query(
                PUBLIC_API,
                "app.bsky.actor.getProfile",
                &[("actor", account)],
            ))
            .await
            .map_err(|error| self.diagnostic(&error, "profile lookup"))?;
        if profile.handle != account || !BlueskyIntent::valid_did(&profile.did) {
            return Err("Bluesky profile did not confirm the approved handle and DID".into());
        }
        let did = profile.did;
        let document_url = match did.strip_prefix("did:web:") {
            Some(host) => format!("https://{host}/.well-known/did.json"),
            None => format!("https://plc.directory/{did}"),
        };
        let (_, document): (_, DidDocument) = self
            .read(self.client.get(document_url))
            .await
            .map_err(|error| self.diagnostic(&error, "DID lookup"))?;
        if document.id != did || !document.also_known_as.contains(&format!("at://{account}")) {
            return Err("Bluesky DID document does not confirm the handle".into());
        }
        let service_id = format!("{did}#atproto_pds");
        let service = document
            .service
            .iter()
            .find(|service| service.id == "#atproto_pds" || service.id == service_id);
        let pds = match service {
            Some(service) if service.kind == "AtprotoPersonalDataServer" => {
                let endpoint = service
                    .endpoint
                    .as_str()
                    .ok_or("Bluesky DID document advertises an invalid PDS")?;
                let origin = endpoint.strip_suffix('/').unwrap_or(endpoint);
                if !Url::parse(origin).is_ok_and(|url| {
                    url.scheme() == "https" && url.origin().ascii_serialization() == origin
                }) {
                    return Err("Bluesky DID document advertises an invalid PDS".into());
                }
                origin.to_owned()
            }
            Some(_) => return Err("Bluesky DID document advertises an invalid PDS".into()),
            None => "https://bsky.social".into(),
        };
        Ok(BlueskyIntent {
            account: account.into(),
            did,
            pds,
            rkey: record_key(request_key),
            text: text.into(),
            collection: Some(BlueskyIntent::COLLECTION.into()),
        })
    }

    pub(super) async fn session(&self, intent: &BlueskyIntent) -> Result<String, String> {
        let Some(password) = self
            .password
            .as_deref()
            .filter(|password| !password.is_empty())
            .filter(|_| self.handle.as_deref() == Some(intent.account.as_str()))
        else {
            return Err(
                "Set BLUESKY_HANDLE and BLUESKY_APP_PASSWORD for the approved handle".into(),
            );
        };
        let (status, session): (_, Session) = self
            .read(
                self.client
                    .post(self.xrpc(&intent.pds, "com.atproto.server.createSession"))
                    .json(&json!({ "identifier": intent.account, "password": password })),
            )
            .await
            .map_err(|error| self.diagnostic(&error, "session"))?;
        if session.identity.did != intent.did
            || session.identity.handle != intent.account
            || session.access_jwt.is_empty()
        {
            return Err(self.diagnostic(&ApiError::Invalid(status), "session"));
        }
        Ok(session.access_jwt)
    }

    // Only the repository's explicit XRPC error is absence evidence.
    async fn repository(&self, intent: &BlueskyIntent) -> Result<Option<RepoRecord>, ApiError> {
        let result = self
            .read(self.query(
                &intent.pds,
                "com.atproto.repo.getRecord",
                &[
                    ("repo", &intent.did),
                    ("collection", intent.collection()),
                    ("rkey", &intent.rkey),
                ],
            ))
            .await;
        match result {
            Ok((_, record)) => Ok(Some(record)),
            Err(ApiError::Status(StatusCode::BAD_REQUEST, name)) if name == "RecordNotFound" => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(super) async fn verify_absent(&self, intent: &BlueskyIntent) -> Result<(), String> {
        match self.repository(intent).await {
            Ok(None) => Ok(()),
            Ok(Some(_)) => Err("Bluesky PDS already has a record at the saved key".into()),
            Err(error) => Err(self.diagnostic(&error, "record lookup")),
        }
    }

    pub(super) async fn lookup(
        &self,
        intent: &BlueskyIntent,
        expected_cid: Option<&str>,
    ) -> Lookup {
        let repo = match self.repository(intent).await {
            Ok(None) => return Lookup::Missing,
            Ok(Some(record)) => record,
            Err(ApiError::Json(status)) if status.is_success() => {
                return Lookup::Conflict(
                    "Bluesky PDS returned an invalid record at the saved key".into(),
                );
            }
            Err(error) => return Lookup::Unverified(self.diagnostic(&error, "record lookup")),
        };
        let uri = at_uri(intent);
        let expected_facets = facets(&intent.text);
        if repo.receipt.uri != uri
            || repo.receipt.cid.is_empty()
            || expected_cid.is_some_and(|cid| cid != repo.receipt.cid)
            || !matches(&repo.value, &intent.text, &expected_facets)
        {
            return Lookup::Conflict(
                "Bluesky record differs from the approved post or saved receipt".into(),
            );
        }
        let evidence = Evidence {
            receipt: repo.receipt,
            url: format!(
                "https://bsky.app/profile/{}/post/{}",
                intent.did, intent.rkey
            ),
        };
        let view: Result<(_, PublicPosts), _> = self
            .read(self.query(PUBLIC_API, "app.bsky.feed.getPosts", &[("uris", &uri)]))
            .await;
        let view = match view {
            Ok((_, view)) => view,
            Err(error) => {
                return Lookup::UnverifiedView(evidence, self.diagnostic(&error, "public lookup"));
            }
        };
        let Some(post) = view.posts.iter().find(|post| post.receipt.uri == uri) else {
            return Lookup::Pending(evidence);
        };
        if post.receipt.cid != evidence.receipt.cid
            || post.author.did != intent.did
            || !matches(&post.record, &intent.text, &expected_facets)
        {
            return Lookup::UnverifiedView(
                evidence,
                "Public Bluesky post differs from the approved repository record".into(),
            );
        }
        Lookup::Public(evidence)
    }

    pub(super) async fn card(&self, text: &str) -> Option<card::Card> {
        let uri = links(text).find_map(|(_, _, url)| card::eligible(&url).then_some(url))?;
        card::fetch(&self.client, uri).await
    }

    pub(super) async fn upload_card(
        &self,
        intent: &BlueskyIntent,
        token: &str,
        card: card::Card,
    ) -> Option<Value> {
        #[derive(Deserialize)]
        struct Upload {
            blob: Value,
        }
        let mut embed = card.embed;
        if let Some((bytes, mime)) = card.thumb {
            let (_, upload): (_, Upload) = self
                .read(
                    self.client
                        .post(self.xrpc(&intent.pds, "com.atproto.repo.uploadBlob"))
                        .timeout(card::TIMEOUT)
                        .bearer_auth(token)
                        .header(header::CONTENT_TYPE, mime)
                        .body(bytes),
                )
                .await
                .ok()?;
            if !upload.blob.is_object() {
                return None;
            }
            embed["external"]["thumb"] = upload.blob;
        }
        Some(embed)
    }

    pub(super) async fn create(
        &self,
        intent: &BlueskyIntent,
        token: &str,
        embed: Option<Value>,
    ) -> Result<Receipt, WriteError> {
        let facets = facets(&intent.text);
        let mut record =
            json!({ "$type": intent.collection(), "text": intent.text, "createdAt": timestamp() });
        if !facets.is_empty() {
            record["facets"] = Value::Array(facets);
        }
        if let Some(embed) = embed {
            record["embed"] = embed;
        }
        let result = self
            .read(
                self.client
                    .post(self.xrpc(&intent.pds, "com.atproto.repo.createRecord"))
                    .bearer_auth(token)
                    .json(&json!({
                        "repo": intent.did,
                        "collection": intent.collection(),
                        "rkey": intent.rkey,
                        "record": record,
                    })),
            )
            .await;
        let error = match result {
            Ok((_, receipt)) if valid_receipt(&receipt, intent) => return Ok(receipt),
            Ok((status, _)) => ApiError::Invalid(status),
            Err(error) => error,
        };
        let rejected = matches!(&error, ApiError::Status(StatusCode::BAD_REQUEST, name) if name == "InvalidRequest");
        Err(WriteError {
            rejected,
            reason: error.reason(
                "createRecord",
                &[self.password.as_deref().unwrap_or(""), token],
            ),
        })
    }
}

async fn read_response<T: serde::de::DeserializeOwned>(
    response: Response,
) -> Result<(StatusCode, T), ApiError> {
    let status = response.status();
    let body = bounded_body(response, JSON_LIMIT)
        .await
        .map_err(|error| ApiError::Body(status, error))?;
    if !status.is_success() {
        let error =
            serde_json::from_slice::<XrpcError>(&body).map_err(|_| ApiError::Json(status))?;
        return Err(ApiError::Status(status, error.error));
    }
    let value = serde_json::from_slice(&body).map_err(|_| ApiError::Json(status))?;
    Ok((status, value))
}

fn at_uri(intent: &BlueskyIntent) -> String {
    format!(
        "at://{}/{}/{}",
        intent.did,
        intent.collection(),
        intent.rkey
    )
}

fn valid_receipt(receipt: &Receipt, intent: &BlueskyIntent) -> bool {
    !receipt.cid.is_empty() && receipt.uri == at_uri(intent)
}

fn record_key(request_key: &str) -> String {
    const ALPHABET: &[u8] = b"234567abcdefghijklmnopqrstuvwxyz";
    let mut value = u64::from_str_radix(&request_key[7..23], 16).expect("generated SHA-256 digest")
        & 0x7fff_ffff_ffff_ffff;
    let mut tid = [b'2'; 13];
    for digit in tid.iter_mut().rev() {
        *digit = ALPHABET[(value & 31) as usize];
        value >>= 5;
    }
    tid.into_iter().map(char::from).collect()
}

fn links(text: &str) -> impl Iterator<Item = (usize, usize, Url)> + '_ {
    LINKS.captures_iter(text).filter_map(|capture| {
        let link = capture.get(1)?;
        let mut value = link
            .as_str()
            .trim_end_matches(['.', ',', '!', '?', ';', ':']);
        for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
            if value.ends_with(close) && value.matches(close).count() > value.matches(open).count()
            {
                value = &value[..value.len() - 1];
            }
        }
        let url = Url::parse(value).ok()?;
        url.host()?;
        Some((link.start(), link.start() + value.len(), url))
    })
}

fn facets(text: &str) -> Vec<Value> {
    links(text)
        .map(|(start, end, url)| {
            json!({
                "index": { "byteStart": start, "byteEnd": end },
                "features": [{ "$type": LINK, "uri": url.as_str() }],
            })
        })
        .collect()
}

fn matches(record: &Value, text: &str, facets: &[Value]) -> bool {
    let Some(record) = record.as_object() else {
        return false;
    };
    record.keys().all(|field| {
        matches!(
            field.as_str(),
            "$type" | "text" | "createdAt" | "facets" | "embed"
        )
    }) && record.get("$type").and_then(Value::as_str) == Some(BlueskyIntent::COLLECTION)
        && record.get("text").and_then(Value::as_str) == Some(text)
        && record
            .get("createdAt")
            .and_then(Value::as_str)
            .is_some_and(|time| OffsetDateTime::parse(time, &Rfc3339).is_ok())
        && record.get("facets").map_or_else(
            || facets.is_empty(),
            |value| value.as_array().is_some_and(|actual| actual == facets),
        )
        && record.get("embed").is_none_or(valid_embed)
}

fn valid_embed(embed: &Value) -> bool {
    let Some(embed) = embed.as_object() else {
        return false;
    };
    embed.len() == 2
        && embed.get("$type").and_then(Value::as_str) == Some("app.bsky.embed.external")
        && embed
            .get("external")
            .and_then(Value::as_object)
            .is_some_and(|external| {
                external.keys().all(|field| {
                    matches!(field.as_str(), "uri" | "title" | "description" | "thumb")
                }) && external
                    .get("uri")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
                    && external.get("title").is_some_and(Value::is_string)
                    && external.get("description").is_some_and(Value::is_string)
            })
}
