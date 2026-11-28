use axum::{
    Json, Router,
    body::to_bytes,
    extract::Request,
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
    serve::Listener,
};
use kirzner::campaign::{
    ActionRecord, Bluesky, BlueskyIntent, CampaignResult, ExecutionEntry, GitHubReleases, Intent,
    Milestone, RecordStatus, ReleaseIntent, Step, approval_batch, campaign_approve, read_campaign,
    reconcile_absent, run_campaign, with_campaign,
};
use reqwest::{
    ClientBuilder,
    dns::{Name, Resolve, Resolving},
};
use serde_json::{Value, json};
use std::{
    fs, io,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig,
        pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
    },
    server::TlsStream,
};

const FILE: &str = "campaign.md";
const HANDLE: &str = "test.example";
const DID: &str = "did:plc:abcdefghijklmnopqrstuvwx";
const PASSWORD: &str = "fake-app-pass-word";
const JWT: &str = "mock-secret-jwt";
const TOKEN: &str = "mock-github-token";
const COLLECTION: &str = "app.bsky.feed.post";
const SAVED_KEY: &str = "3222222222222";
const CID: &str = "bafyreicampaign";
const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const LINK_TEXT: &str = "世界 🌎 (HTTPS://card.test/page), unchanged";
const HOSTS: &[&str] = &[
    "public.api.bsky.app",
    "plc.directory",
    "pds.test",
    "saved.test",
    "card.test",
    "api.github.com",
    "health.test",
];

struct Workspace(PathBuf);

impl Workspace {
    fn new(text: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "kirzner-runners-{}-{}-{}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join(FILE), text).unwrap();
        Self(path)
    }

    fn approved(text: &str) -> Self {
        let workspace = Self::new(text);
        let (_, digest, _) = approval_batch(&read_campaign(&workspace.0, FILE).unwrap()).unwrap();
        campaign_approve(&workspace.0, FILE, &digest).unwrap();
        workspace
    }

    fn text(&self) -> String {
        fs::read_to_string(self.0.join(FILE)).unwrap()
    }

    fn write(&self, text: &str) {
        fs::write(self.0.join(FILE), text).unwrap();
    }

    fn records(&self) -> Vec<ActionRecord> {
        read_campaign(&self.0, FILE).unwrap().records
    }

    fn append(&self, record: ActionRecord) {
        with_campaign(&self.0, FILE, |store| {
            store.append(&ExecutionEntry {
                record: Some(record),
                ..ExecutionEntry::default()
            })
        })
        .unwrap();
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bluesky_plan(text: &str) -> String {
    format!(
        "---\nschema: 1\ncampaign: demo\nitems:\n  - id: d1\n    content: post\n    target: bluesky/{HANDLE}\n    steps: [publish]\n---\n\n## post\n\n{text}\n"
    )
}

fn release_plan() -> String {
    format!(
        "---\nschema: 1\ncampaign: release\nitems:\n  - id: d1\n    content: release\n    target: github/Org/Repo\n    account: release-bot\n    steps: [publish]\n    tag: v1/rc\n    commit: {PIN}\n---\n\n## release\n\nTitle: Test release\n\nBody\n"
    )
}

#[derive(Clone)]
struct Call {
    method: Method,
    host: String,
    path: String,
    query: String,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Call {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }

    fn auth(&self) -> Option<&str> {
        self.headers
            .get(header::AUTHORIZATION)
            .map(|value| value.to_str().unwrap())
    }

    fn param(&self, key: &str) -> String {
        url::form_urlencoded::parse(self.query.as_bytes())
            .find_map(|(name, value)| (name == key).then(|| value.into_owned()))
            .unwrap()
    }
}

struct TlsListener(TcpListener, TlsAcceptor);

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (socket, address) = self.0.accept().await.unwrap();
            if let Ok(stream) = self.1.accept(socket).await {
                return (stream, address);
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.0.local_addr()
    }
}

struct NoExternalDns;

impl Resolve for NoExternalDns {
    fn resolve(&self, _: Name) -> Resolving {
        Box::pin(async { Err(io::Error::other("No external DNS in campaign runner tests").into()) })
    }
}

struct Mock {
    calls: Arc<Mutex<Vec<Call>>>,
    addr: SocketAddr,
    cert: reqwest::Certificate,
    task: JoinHandle<()>,
}

impl Mock {
    async fn new(handler: impl Fn(Call) -> Response + Send + Sync + 'static) -> Self {
        let rcgen::CertifiedKey { cert, signing_key } = rcgen::generate_simple_self_signed(
            HOSTS
                .iter()
                .map(|host| (*host).to_owned())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let tls = ServerConfig::builder_with_provider(Arc::new(
            tokio_rustls::rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(signing_key.serialize_der())),
        )
        .unwrap();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(tls));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let requests = calls.clone();
        let handler = Arc::new(handler);
        let router = Router::new().fallback(move |request: Request| {
            let requests = requests.clone();
            let handler = handler.clone();
            async move {
                let (parts, body) = request.into_parts();
                let call = Call {
                    method: parts.method,
                    host: parts.headers[header::HOST].to_str().unwrap().into(),
                    path: parts.uri.path().into(),
                    query: parts.uri.query().unwrap_or("").into(),
                    headers: parts.headers,
                    body: to_bytes(body, 2 * 1024 * 1024).await.unwrap().to_vec(),
                };
                requests.lock().unwrap().push(call.clone());
                handler(call)
            }
        });
        let task = tokio::spawn(async move {
            axum::serve(TlsListener(listener, acceptor), router)
                .await
                .unwrap()
        });
        Self {
            calls,
            addr,
            cert: reqwest::Certificate::from_der(cert.der()).unwrap(),
            task,
        }
    }

    fn builder(&self) -> ClientBuilder {
        HOSTS.iter().fold(
            reqwest::Client::builder()
                .add_root_certificate(self.cert.clone())
                .dns_resolver2(NoExternalDns),
            |builder, host| builder.resolve(host, self.addr),
        )
    }

    fn bluesky(&self) -> Bluesky {
        Bluesky::new(self.builder(), None, Some(HANDLE), Some(PASSWORD.into())).unwrap()
    }

    fn github(&self) -> GitHubReleases {
        GitHubReleases::new(self.builder(), Some(TOKEN.into())).unwrap()
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    async fn run(&self, workspace: &Workspace) -> Vec<CampaignResult> {
        run_campaign(&workspace.0, FILE, &self.github(), &self.bluesky())
            .await
            .unwrap()
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn response(code: u16, value: Value) -> Response {
    (StatusCode::from_u16(code).unwrap(), Json(value)).into_response()
}

fn receipt(key: &str) -> Value {
    json!({"uri": format!("at://{DID}/{COLLECTION}/{key}"), "cid": CID})
}

fn post(text: &str, key: &str) -> Value {
    let mut post = receipt(key);
    post["value"] = json!({"$type": COLLECTION, "text": text, "createdAt": "2026-11-28T12:00:00Z"});
    post
}

#[derive(Default)]
struct Bsky {
    public: bool,
    different_author: bool,
    record: Option<Value>,
    lookup_error: Option<(u16, &'static str)>,
    create_error: Option<(u16, Value)>,
    image_len: Option<usize>,
    image_redirect: bool,
    upload_error: bool,
    page_redirects: u8,
    reserve_in: Option<PathBuf>,
    pause_on_upload: Option<PathBuf>,
}

impl Bsky {
    fn handle(&mut self, call: Call) -> Response {
        match call.path.as_str() {
            "/xrpc/app.bsky.actor.getProfile" => {
                assert_eq!(call.host, "public.api.bsky.app");
                assert_eq!(call.param("actor"), HANDLE);
                response(200, json!({"did": DID, "handle": HANDLE}))
            }
            path if path.strip_prefix('/') == Some(DID) => {
                assert_eq!(call.host, "plc.directory");
                response(
                    200,
                    json!({
                        "id": DID,
                        "alsoKnownAs": [format!("at://{HANDLE}")],
                        "service": [{"id": "#atproto_pds", "type": "AtprotoPersonalDataServer", "serviceEndpoint": "https://pds.test/"}],
                    }),
                )
            }
            "/xrpc/com.atproto.server.createSession" => {
                assert_eq!(call.method, Method::POST);
                assert_eq!(call.auth(), None);
                assert_eq!(
                    call.json(),
                    json!({"identifier": HANDLE, "password": PASSWORD})
                );
                response(200, json!({"did": DID, "handle": HANDLE, "accessJwt": JWT}))
            }
            "/xrpc/com.atproto.repo.getRecord" => {
                assert_eq!(call.method, Method::GET);
                assert_eq!(call.auth(), None);
                assert_eq!(call.param("repo"), DID);
                assert_eq!(call.param("collection"), COLLECTION);
                if let Some((status, body)) = self.lookup_error {
                    return (StatusCode::from_u16(status).unwrap(), body).into_response();
                }
                match &self.record {
                    Some(record) => response(200, record.clone()),
                    None => response(400, json!({"error": "RecordNotFound"})),
                }
            }
            "/xrpc/app.bsky.feed.getPosts" => {
                assert_eq!(call.auth(), None);
                if self.public {
                    let record = self.record.as_ref().unwrap();
                    assert_eq!(call.param("uris"), record["uri"].as_str().unwrap());
                    return response(
                        200,
                        json!({"posts": [{
                            "author": {"did": if self.different_author { "did:plc:other" } else { DID }},
                            "uri": record["uri"], "cid": record["cid"], "record": record["value"]
                        }]}),
                    );
                }
                response(200, json!({"posts": []}))
            }
            "/xrpc/com.atproto.repo.uploadBlob" => {
                assert_eq!(call.auth(), Some("Bearer mock-secret-jwt"));
                assert_eq!(call.headers[header::CONTENT_TYPE], "image/png");
                assert_eq!(call.body.len(), 32);
                if let Some(path) = &self.pause_on_upload {
                    fs::write(path.join(".kirzner.pause"), "").unwrap();
                }
                if self.upload_error {
                    return response(500, json!({"error": "InternalServerError"}));
                }
                response(
                    200,
                    json!({"blob": {"$type": "blob", "ref": {"$link": "mock-blob"}, "mimeType": "image/png", "size": 32}}),
                )
            }
            "/xrpc/com.atproto.repo.createRecord" => {
                assert_eq!(call.auth(), Some("Bearer mock-secret-jwt"));
                if let Some(path) = &self.reserve_in {
                    assert!(path.join(".kirzner.lock").is_dir());
                    let records = read_campaign(path, FILE).unwrap().records;
                    assert_eq!(records.last().unwrap().status, Some(RecordStatus::Reserved));
                }
                if let Some((status, body)) = &self.create_error {
                    return response(*status, body.clone());
                }
                let request = call.json();
                assert_eq!(request["repo"], DID);
                assert_eq!(request["collection"], COLLECTION);
                let ack = receipt(request["rkey"].as_str().unwrap());
                let mut record = ack.clone();
                record["value"] = request["record"].clone();
                self.record = Some(record);
                response(200, ack)
            }
            "/page" => {
                assert_eq!(call.host, "card.test");
                assert_eq!(call.auth(), None);
                (StatusCode::FOUND, [(header::LOCATION, "/landing/0")]).into_response()
            }
            path if path.starts_with("/landing/") => {
                assert_eq!(call.auth(), None);
                let index = path.rsplit('/').next().unwrap().parse::<u8>().unwrap();
                if index < self.page_redirects {
                    return (
                        StatusCode::FOUND,
                        [(header::LOCATION, format!("/landing/{}", index + 1))],
                    )
                        .into_response();
                }
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    "<title>Fallback &amp; title</title><!-- <meta property=og:title content=wrong> --><meta content='A &amp; B &gt; C' PROPERTY=og:title><meta name=description content=\"Description &#x1F30E;\"><meta property=og:image content=../assets/og.png>",
                ).into_response()
            }
            "/assets/og.png" => {
                assert_eq!(call.auth(), None);
                if self.image_redirect {
                    return (StatusCode::FOUND, [(header::LOCATION, "/not-allowed")])
                        .into_response();
                }
                (
                    [(header::CONTENT_TYPE, "image/png")],
                    vec![0; self.image_len.unwrap_or(32)],
                )
                    .into_response()
            }
            "/health" => {
                assert_eq!(call.auth(), None);
                StatusCode::OK.into_response()
            }
            "/unhealthy" => StatusCode::SERVICE_UNAVAILABLE.into_response(),
            _ => panic!("Unexpected mock Bluesky path: {}", call.path),
        }
    }
}

async fn bsky_mock(state: Bsky) -> (Mock, Arc<Mutex<Bsky>>) {
    let state = Arc::new(Mutex::new(state));
    let server_state = state.clone();
    (
        Mock::new(move |call| server_state.lock().unwrap().handle(call)).await,
        state,
    )
}

fn assert_awk_fences(workspace: &Workspace) {
    let result = Command::new("awk")
        .arg(
            r#"
            /^record:$/ { record = 1; seen++; next }
            /^```$/ { record = 0; next }
            record && !/^($|    |  [a-z_]+:)/ { exit 2 }
            END { if (!seen) exit 3 }
        "#,
        )
        .arg(workspace.0.join(FILE))
        .status()
        .unwrap();
    assert!(result.success());
}

#[tokio::test]
async fn bluesky_uses_tid_durable_reservation_and_two_public_reads() {
    let workspace = Workspace::approved(&bluesky_plan("hello 🌎"));
    let (mock, state) = bsky_mock(Bsky {
        reserve_in: Some(workspace.0.clone()),
        ..Bsky::default()
    })
    .await;
    let result = mock.run(&workspace).await;
    assert_eq!(result[0].status, "pending");
    let records = workspace.records();
    assert_eq!(records.len(), 2);
    let Some(Intent::Bluesky(intent)) = &records[0].intent else {
        panic!("Missing Bluesky reservation")
    };
    assert_eq!(intent.rkey, "5j7nsnpreeqnn");
    assert_eq!(records[1].status, Some(RecordStatus::Pending));
    assert_eq!(result[0].cid.as_deref(), Some(CID));
    state.lock().unwrap().public = true;
    fs::write(workspace.0.join(".kirzner.pause"), "").unwrap();
    assert_eq!(mock.run(&workspace).await[0].status, "public");
    assert_eq!(
        workspace.records().last().unwrap().milestone,
        Milestone::Public
    );
    let count = mock.calls().len();
    assert_eq!(mock.run(&workspace).await[0].status, "public");
    assert_eq!(mock.calls().len(), count);
    assert_awk_fences(&workspace);
}

#[tokio::test]
async fn bluesky_card_preserves_facets_and_applies_fetch_limits() {
    let (mock, state) = bsky_mock(Bsky {
        public: true,
        ..Bsky::default()
    })
    .await;
    for (index, expected_embed, expected_thumb) in [
        (0, true, true),
        (1, true, false),
        (2, false, false),
        (3, true, false),
        (4, false, false),
        (5, true, true),
        (6, false, false),
    ] {
        {
            let mut state = state.lock().unwrap();
            state.record = None;
            state.image_len = match index {
                1 => Some(1_200_000),
                6 => Some(5 * 1024 * 1024 + 1),
                _ => None,
            };
            state.upload_error = index == 2;
            state.image_redirect = index == 3;
            state.page_redirects = match index {
                4 => 3,
                5 => 2,
                _ => 0,
            };
        }
        let workspace = Workspace::approved(&bluesky_plan(LINK_TEXT));
        let count = mock.calls().len();
        assert_eq!(mock.run(&workspace).await[0].status, "public");
        let calls = mock.calls();
        let calls = &calls[count..];
        let call = calls
            .iter()
            .find(|call| call.path.ends_with(".createRecord"))
            .unwrap();
        let body = call.json();
        let record = &body["record"];
        assert_eq!(record["text"], LINK_TEXT);
        let start = LINK_TEXT.find("HTTPS").unwrap();
        assert_eq!(
            record["facets"],
            json!([{
                "index": {"byteStart": start, "byteEnd": start + "HTTPS://card.test/page".len()},
                "features": [{"$type": "app.bsky.richtext.facet#link", "uri": "https://card.test/page"}]
            }])
        );
        assert_eq!(record.get("embed").is_some(), expected_embed);
        if expected_embed {
            let external = &record["embed"]["external"];
            assert_eq!(external["title"], "A & B > C");
            assert_eq!(external["description"], "Description 🌎");
            assert_eq!(external.get("thumb").is_some(), expected_thumb);
        }
        assert!(
            !calls
                .iter()
                .any(|call| call.path == "/not-allowed" || call.path == "/landing/3")
        );
        assert!(
            calls
                .iter()
                .filter(|call| call.host == "card.test")
                .all(|call| call.auth().is_none() && call.body.is_empty())
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.path.ends_with(".uploadBlob"))
                .count(),
            usize::from(matches!(index, 0 | 2 | 5))
        );
    }
}

#[tokio::test]
async fn pause_after_blob_upload_stops_post_dispatch() {
    let workspace = Workspace::approved(&bluesky_plan(LINK_TEXT));
    let (mock, _) = bsky_mock(Bsky {
        pause_on_upload: Some(workspace.0.clone()),
        ..Bsky::default()
    })
    .await;
    assert_eq!(mock.run(&workspace).await[0].status, "paused");
    assert!(
        !mock
            .calls()
            .iter()
            .any(|call| call.path.ends_with(".createRecord"))
    );
    assert_eq!(
        workspace.records().last().unwrap().status,
        Some(RecordStatus::Paused)
    );
}

#[tokio::test]
async fn bluesky_rejects_definite_errors_and_redacts_ambiguous_ones() {
    let (mock, state) = bsky_mock(Bsky::default()).await;
    for (status, name, expected, ledger) in [
        (400, "InvalidRequest", "blocked", RecordStatus::Rejected),
        (500, "InternalServerError", "unknown", RecordStatus::Unknown),
        (400, JWT, "unknown", RecordStatus::Unknown),
    ] {
        let workspace = Workspace::approved(&bluesky_plan("example post"));
        state.lock().unwrap().create_error = Some((
            status,
            json!({"error": name, "message": format!("{PASSWORD} {JWT}")}),
        ));
        let result = mock.run(&workspace).await;
        assert_eq!(result[0].status, expected);
        let reason = result[0].reason.as_ref().unwrap();
        assert!(reason.contains(&format!("createRecord returned HTTP {status}")));
        assert!(reason.contains("timeout=false"));
        if name != JWT {
            assert!(reason.contains(name));
        }
        assert!(!reason.contains(PASSWORD));
        assert!(!reason.contains(JWT));
        assert!(!workspace.text().contains(PASSWORD));
        assert!(!workspace.text().contains(JWT));
        assert_eq!(workspace.records().last().unwrap().status, Some(ledger));
        if name == "InvalidRequest" {
            let calls = mock.calls().len();
            let entries = workspace.records().len();
            let second = mock.run(&workspace).await;
            assert_eq!(second[0].status, "blocked");
            assert_eq!(mock.calls().len(), calls);
            assert_eq!(workspace.records().len(), entries);
        }
    }
}

fn unknown(workspace: &Workspace, key: &str, pds: &str) -> ActionRecord {
    let (batch, _, _) = approval_batch(&read_campaign(&workspace.0, FILE).unwrap()).unwrap();
    let action = &batch[0];
    ActionRecord {
        item: action.item.clone(),
        milestone: Milestone::Prepared,
        version: Some(action.version.clone()),
        step: Some(Step::Publish),
        request_key: Some(action.request_key.clone()),
        remote_key: Some(action.remote_key.clone()),
        intent: Some(Intent::Bluesky(BlueskyIntent {
            account: HANDLE.into(),
            did: DID.into(),
            pds: pds.into(),
            rkey: key.into(),
            text: action.text.clone().unwrap(),
            collection: None,
        })),
        attempted_at: Some("2026-11-28T12:00:00Z".into()),
        status: Some(RecordStatus::Unknown),
        cid: None,
        native_id: None,
        url: None,
        observed_at: None,
        source: None,
        reason: Some("Earlier write could not be verified".into()),
        extra: Default::default(),
    }
}

#[tokio::test]
async fn verified_absence_reuses_the_saved_key_for_adoption_and_create() {
    for late in [true, false] {
        let workspace = Workspace::approved(&bluesky_plan("example post"));
        let original = unknown(&workspace, SAVED_KEY, "https://saved.test");
        workspace.append(original.clone());
        let before = workspace.text();
        let (mock, state) = bsky_mock(Bsky {
            public: true,
            reserve_in: Some(workspace.0.clone()),
            ..Bsky::default()
        })
        .await;
        let absent = reconcile_absent(&workspace.0, FILE, "d1", &mock.bluesky())
            .await
            .unwrap();
        assert!(workspace.text().starts_with(&before));
        assert_eq!(mock.calls().len(), 1);
        assert_eq!(absent.request_key, original.request_key);
        assert_eq!(absent.version, original.version);
        assert_eq!(absent.status, Some(RecordStatus::Absent));
        assert_eq!(absent.source.as_deref(), Some("bluesky_pds_get_record"));
        assert!(absent.observed_at.is_some());
        let Some(Intent::Bluesky(intent)) = absent.intent else {
            panic!("Missing saved identity")
        };
        assert_eq!(intent.collection.as_deref(), Some(COLLECTION));
        assert_eq!(intent.pds, "https://saved.test");
        assert_eq!(intent.rkey, SAVED_KEY);
        let untouched = workspace.text();
        assert!(
            reconcile_absent(&workspace.0, FILE, "d1", &mock.bluesky())
                .await
                .is_err()
        );
        assert_eq!(workspace.text(), untouched);
        if late {
            state.lock().unwrap().record = Some(post("example post", SAVED_KEY));
        }
        assert_eq!(mock.run(&workspace).await[0].status, "public");
        let calls = mock.calls();
        assert!(!calls.iter().any(|call| call.path.ends_with(".getProfile")));
        let create = calls
            .iter()
            .find(|call| call.path.ends_with(".createRecord"));
        if late {
            assert!(create.is_none());
        } else {
            let create = create.unwrap();
            assert_eq!(create.host, "saved.test");
            assert_eq!(create.json()["rkey"], SAVED_KEY);
        }
        assert!(
            calls
                .iter()
                .filter(|call| call.path.ends_with(".getRecord"))
                .all(|call| call.host == "saved.test" && call.param("rkey") == SAVED_KEY)
        );
        assert_eq!(workspace.records()[0].status, Some(RecordStatus::Unknown));
        assert_awk_fences(&workspace);
    }
}

#[tokio::test]
async fn verified_absence_accepts_saved_tid_starting_with_a() {
    let workspace = Workspace::approved(&bluesky_plan("example post"));
    let tid = "a222222222222";
    workspace.append(unknown(&workspace, tid, "https://saved.test"));
    let (mock, _) = bsky_mock(Bsky {
        public: true,
        ..Bsky::default()
    })
    .await;
    reconcile_absent(&workspace.0, FILE, "d1", &mock.bluesky())
        .await
        .unwrap();

    assert_eq!(mock.run(&workspace).await[0].status, "public");
    let create = mock
        .calls()
        .into_iter()
        .find(|call| call.path.ends_with(".createRecord"))
        .expect("saved TID should be published");
    assert_eq!(create.json()["rkey"], tid);
}

#[tokio::test]
async fn reconcile_refusals_preserve_the_file_and_legacy_keys_are_readable() {
    let (mock, state) = bsky_mock(Bsky::default()).await;
    for (status, body) in [
        (404, r#"{"error":"RecordNotFound"}"#),
        (401, r#"{"error":"AuthRequired"}"#),
        (400, r#"{"error":"InvalidRequest"}"#),
        (400, "not JSON"),
        (200, r#"{"error":"RecordNotFound"}"#),
    ] {
        let workspace = Workspace::approved(&bluesky_plan("example post"));
        workspace.append(unknown(&workspace, SAVED_KEY, "https://saved.test"));
        state.lock().unwrap().lookup_error = Some((status, body));
        assert_refuses(&mock, &workspace).await;
    }
    state.lock().unwrap().lookup_error = None;
    let workspace = Workspace::approved(&bluesky_plan("example post"));
    workspace.append(unknown(&workspace, SAVED_KEY, "https://unresolved.test"));
    assert_refuses(&mock, &workspace).await;

    let workspace = Workspace::approved(&bluesky_plan("example post"));
    let mut release = unknown(&workspace, SAVED_KEY, "https://saved.test");
    release.intent = Some(Intent::Release(ReleaseIntent {
        repo: "org/repo".into(),
        account: "release-bot".into(),
        tag: "v1".into(),
        commit: None,
        title: "release".into(),
        body: "body".into(),
    }));
    workspace.append(release);
    assert_refuses(&mock, &workspace).await;
    let mut second = unknown(&workspace, SAVED_KEY, "https://saved.test");
    second.request_key = Some(format!("sha256:{}", "e".repeat(64)));
    workspace.append(second);
    assert_refuses(&mock, &workspace).await;

    let legacy = format!("kirzner-{}", "f".repeat(64));
    let workspace = Workspace::approved(&bluesky_plan("example post"));
    workspace.append(unknown(&workspace, &legacy, "https://saved.test"));
    for text in ["conflicting text", "example post"] {
        state.lock().unwrap().record = Some(post(text, &legacy));
        assert_refuses(&mock, &workspace).await;
    }
    assert_eq!(mock.run(&workspace).await[0].status, "pending");
    state.lock().unwrap().public = true;
    assert_eq!(mock.run(&workspace).await[0].status, "public");

    let retry = Workspace::approved(&bluesky_plan("example post"));
    retry.append(unknown(&retry, &legacy, "https://saved.test"));
    state.lock().unwrap().record = None;
    reconcile_absent(&retry.0, FILE, "d1", &mock.bluesky())
        .await
        .unwrap();
    let result = mock.run(&retry).await;
    assert_eq!(result[0].status, "blocked");
    assert!(
        result[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("legacy non-TID")
    );
    assert!(
        !mock
            .calls()
            .iter()
            .any(|call| call.path.ends_with(".createRecord"))
    );
}

async fn assert_refuses(mock: &Mock, workspace: &Workspace) {
    let before = workspace.text();
    assert!(
        reconcile_absent(&workspace.0, FILE, "d1", &mock.bluesky())
            .await
            .is_err()
    );
    assert_eq!(workspace.text(), before);
}

#[tokio::test]
async fn a_late_conflict_holds_an_absent_key_and_an_amended_item() {
    let workspace = Workspace::approved(&bluesky_plan("example post"));
    workspace.append(unknown(&workspace, SAVED_KEY, "https://saved.test"));
    let (mock, state) = bsky_mock(Bsky::default()).await;
    reconcile_absent(&workspace.0, FILE, "d1", &mock.bluesky())
        .await
        .unwrap();
    state.lock().unwrap().record = Some(post("different post", SAVED_KEY));
    assert_eq!(mock.run(&workspace).await[0].status, "unknown");
    workspace.write(
        &workspace
            .text()
            .replace("\n\nexample post\n", "\n\namended post\n"),
    );
    let result = mock.run(&workspace).await;
    assert_eq!(
        result
            .iter()
            .map(|item| item.status.as_str())
            .collect::<Vec<_>>(),
        ["unknown", "hold"]
    );
    assert!(
        !mock
            .calls()
            .iter()
            .any(|call| call.path.ends_with(".createRecord"))
    );
}

#[tokio::test]
async fn appview_author_mismatch_stays_unknown_even_with_a_matching_repo_record() {
    let workspace = Workspace::approved(&bluesky_plan("example post"));
    let (mock, _) = bsky_mock(Bsky {
        public: true,
        different_author: true,
        ..Bsky::default()
    })
    .await;
    let result = mock.run(&workspace).await;
    assert_eq!(result[0].status, "unknown");
    assert!(result[0].reason.as_ref().unwrap().contains("differs"));
    assert_eq!(
        workspace.records().last().unwrap().status,
        Some(RecordStatus::Unknown)
    );
}

fn approve(workspace: &Workspace) {
    let (_, digest, _) = approval_batch(&read_campaign(&workspace.0, FILE).unwrap()).unwrap();
    campaign_approve(&workspace.0, FILE, &digest).unwrap();
}

fn second_action(plan: &str, after: &str) -> String {
    format!(
        "{}\n## additional\n\nsecond message\n",
        plan.replacen(
            "\n---\n",
            &format!("\n  - id: d2\n    content: additional\n    target: bluesky/{HANDLE}\n    steps: [publish]\n{after}---\n"),
            1,
        )
    )
}

#[tokio::test]
async fn approval_is_per_item_and_dependencies_require_the_current_public_version() {
    let (mock, _) = bsky_mock(Bsky {
        public: true,
        ..Bsky::default()
    })
    .await;
    let workspace = Workspace::approved(&second_action(&bluesky_plan("first message"), ""));
    workspace.write(
        &workspace
            .text()
            .replace("second message", "edited second message"),
    );
    assert_eq!(
        mock.run(&workspace)
            .await
            .iter()
            .map(|item| item.status.as_str())
            .collect::<Vec<_>>(),
        ["public", "unapproved"]
    );

    let workspace = Workspace::approved(&second_action(
        &bluesky_plan("another message"),
        "    after: d1\n",
    ));
    let mut prior = unknown(&workspace, SAVED_KEY, "https://saved.test");
    prior.milestone = Milestone::Public;
    prior.status = None;
    prior.intent = None;
    workspace.append(prior);
    workspace.write(&workspace.text().replacen(
        "    steps: [publish]\n",
        "    steps: [publish]\n    at: '2020-01-01T00:00:00Z'\n",
        1,
    ));
    let count = mock.calls().len();
    let results = mock.run(&workspace).await;
    assert_eq!(results[0].status, "unapproved");
    assert_eq!(results[1].status, "hold");
    assert!(
        results[1]
            .reason
            .as_ref()
            .unwrap()
            .contains("Waiting for d1")
    );
    assert_eq!(mock.calls().len(), count);
}

#[tokio::test]
async fn a_missing_previously_public_post_is_never_republished() {
    let workspace = Workspace::approved(&bluesky_plan("example post"));
    let mut public = unknown(&workspace, SAVED_KEY, "https://saved.test");
    public.milestone = Milestone::Public;
    public.status = None;
    public.intent = None;
    workspace.append(public);
    let (mock, _) = bsky_mock(Bsky::default()).await;
    assert_eq!(mock.run(&workspace).await[0].status, "public");
    assert!(mock.calls().is_empty());
    workspace.write(&workspace.text().replacen(
        "    steps: [publish]\n",
        "    steps: [publish]\n    at: '2020-01-01T00:00:00Z'\n",
        1,
    ));
    approve(&workspace);
    assert_eq!(mock.run(&workspace).await[0].status, "hold");
    assert!(
        !mock
            .calls()
            .iter()
            .any(|call| call.path.ends_with(".createRecord"))
    );
}

struct Github {
    by_tag: Option<Value>,
    pin: &'static str,
    draft: bool,
    readback: bool,
    create_status: u16,
}

impl Default for Github {
    fn default() -> Self {
        Self {
            by_tag: None,
            pin: PIN,
            draft: false,
            readback: true,
            create_status: 201,
        }
    }
}

fn published_release() -> Value {
    json!({
        "id": 42, "tag_name": "v1/rc", "name": "Test release", "body": "Body",
        "draft": false, "prerelease": false, "published_at": "2026-11-28T12:00:00Z",
        "author": {"login": "Release-Bot"}, "html_url": "https://github.com/Org/Repo/releases/tag/v1%2Frc",
    })
}

impl Github {
    fn handle(&mut self, call: Call) -> Response {
        assert_eq!(call.host, "api.github.com");
        match (call.method.clone(), call.path.as_str()) {
            (Method::GET, "/repos/org/repo/releases/tags/v1%2Frc") => {
                assert_eq!(call.auth(), None);
                match &self.by_tag {
                    Some(release) => response(200, release.clone()),
                    None => StatusCode::NOT_FOUND.into_response(),
                }
            }
            (Method::GET, "/repos/org/repo/commits/tags%2Fv1%2Frc") => {
                assert_eq!(call.auth(), None);
                response(200, json!({"sha": self.pin}))
            }
            (Method::GET, "/user") => {
                assert_eq!(call.auth(), Some("Bearer mock-github-token"));
                response(200, json!({"login": "Release-Bot"}))
            }
            (Method::GET, "/repos/org/repo/releases") => {
                assert_eq!(call.auth(), Some("Bearer mock-github-token"));
                assert_eq!(call.param("per_page"), "100");
                match call.param("page").as_str() {
                    "1" => (
                        [(header::LINK, r#"<https://ignored.test>; rel="next""#)],
                        Json(json!([{"tag_name": "other", "draft": true}])),
                    )
                        .into_response(),
                    "2" => response(200, json!([{"tag_name": "v1/rc", "draft": self.draft}])),
                    page => panic!("Unexpected release page {page}"),
                }
            }
            (Method::POST, "/repos/org/repo/releases") => {
                assert_eq!(call.auth(), Some("Bearer mock-github-token"));
                assert_eq!(
                    call.json(),
                    json!({
                        "tag_name": "v1/rc", "name": "Test release", "body": "Body",
                        "draft": false, "prerelease": false,
                    })
                );
                if self.readback {
                    self.by_tag = Some(published_release());
                }
                response(self.create_status, json!({"id": 42}))
            }
            _ => panic!("Unexpected GitHub path: {}", call.path),
        }
    }
}

async fn github_mock(state: Github) -> (Mock, Arc<Mutex<Github>>) {
    let state = Arc::new(Mutex::new(state));
    let server_state = state.clone();
    (
        Mock::new(move |call| server_state.lock().unwrap().handle(call)).await,
        state,
    )
}

#[tokio::test]
async fn github_tag_match_pin_paginated_draft_and_authenticated_create() {
    let (mock, state) = github_mock(Github {
        by_tag: Some(published_release()),
        ..Github::default()
    })
    .await;
    let workspace = Workspace::approved(&release_plan());
    assert_eq!(mock.run(&workspace).await[0].status, "public");
    assert_eq!(mock.calls().len(), 2);

    let workspace = Workspace::approved(&release_plan());
    state.lock().unwrap().pin = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    assert!(
        mock.run(&workspace).await[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("expected commit")
    );
    assert!(workspace.records().is_empty());

    {
        let mut state = state.lock().unwrap();
        state.pin = PIN;
        state.by_tag = None;
        state.draft = true;
    }
    let before = mock.calls().len();
    assert!(
        mock.run(&workspace).await[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("draft release")
    );
    let calls = mock.calls();
    assert!(
        calls[before..]
            .iter()
            .any(|call| call.query.contains("page=2"))
    );
    assert!(calls.iter().all(|call| call.method == Method::GET));

    state.lock().unwrap().draft = false;
    let result = mock.run(&workspace).await;
    assert_eq!(result[0].status, "public");
    assert_eq!(
        result[0].url.as_deref(),
        Some("https://github.com/Org/Repo/releases/tag/v1%2Frc")
    );
    assert_eq!(workspace.records()[0].status, Some(RecordStatus::Reserved));
    assert_eq!(workspace.records()[1].native_id.as_deref(), Some("42"));
    assert_eq!(mock.calls().last().unwrap().auth(), None);
    assert_awk_fences(&workspace);
}

#[tokio::test]
async fn github_201_and_422_both_need_public_readback() {
    for code in [201, 422] {
        let workspace = Workspace::approved(&release_plan());
        let (mock, state) = github_mock(Github {
            create_status: code,
            readback: false,
            ..Github::default()
        })
        .await;
        assert_eq!(mock.run(&workspace).await[0].status, "unknown");
        assert_eq!(
            workspace.records().last().unwrap().status,
            Some(RecordStatus::Unknown)
        );
        state.lock().unwrap().by_tag = Some(published_release());
        assert_eq!(mock.run(&workspace).await[0].status, "public");
        assert_eq!(
            mock.calls()
                .iter()
                .filter(|call| call.method == Method::POST)
                .count(),
            1
        );
    }
}

async fn socks_probe(listener: TcpListener) -> (u8, Vec<u8>, u16) {
    let (mut socket, _) = listener.accept().await.unwrap();
    assert_eq!(socket.read_u8().await.unwrap(), 5);
    let count = socket.read_u8().await.unwrap();
    let mut methods = vec![0; usize::from(count)];
    socket.read_exact(&mut methods).await.unwrap();
    assert!(methods.contains(&0));
    socket.write_all(&[5, 0]).await.unwrap();
    let mut header = [0; 4];
    socket.read_exact(&mut header).await.unwrap();
    assert_eq!(header[..3], [5, 1, 0]);
    let length = match header[3] {
        1 => 4,
        3 => usize::from(socket.read_u8().await.unwrap()),
        4 => 16,
        kind => panic!("Unexpected SOCKS5 address kind {kind}"),
    };
    let mut target = vec![0; length];
    socket.read_exact(&mut target).await.unwrap();
    let port = socket.read_u16().await.unwrap();
    socket
        .write_all(&[5, 4, 0, 1, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    (header[3], target, port)
}

#[tokio::test]
async fn bluesky_probe_uses_socks5_local_dns_and_socks5h_remote_dns() {
    let (mock, _) = bsky_mock(Bsky::default()).await;
    for (scheme, kind, target) in [
        ("socks5", 1, [127, 0, 0, 1].to_vec()),
        ("socks5h", 3, b"health.test".to_vec()),
    ] {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let proxy = format!("{scheme}://{}", listener.local_addr().unwrap());
        let probe = tokio::spawn(socks_probe(listener));
        let bluesky = Bluesky::new(
            mock.builder(),
            Some(&proxy),
            Some(HANDLE),
            Some(PASSWORD.into()),
        )
        .unwrap();
        let url = format!("https://health.test:{}/health", mock.addr.port());
        assert!(bluesky.health(Some(&url)).await.is_err());
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(2), probe)
                .await
                .unwrap()
                .unwrap(),
            (kind, target, mock.addr.port()),
        );
    }
    assert_eq!(
        mock.bluesky()
            .health(Some("https://health.test/health"))
            .await
            .unwrap(),
        200
    );
    assert_eq!(
        mock.bluesky()
            .health(Some("https://health.test/unhealthy"))
            .await
            .unwrap(),
        503
    );
    assert_eq!(mock.calls().len(), 2);
}

#[tokio::test]
async fn github_explicitly_bypasses_ambient_https_proxy() {
    const CHILD: &str = "KIRZNER_TEST_CAMPAIGN_AMBIENT_PROXY";
    if std::env::var_os(CHILD).is_none() {
        let proxy = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let proxy_url = format!("socks5h://{}", proxy.local_addr().unwrap());
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "github_explicitly_bypasses_ambient_https_proxy",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("HTTPS_PROXY", proxy_url)
            .env_remove("https_proxy")
            .env_remove("NO_PROXY")
            .env_remove("no_proxy")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        proxy.set_nonblocking(true).unwrap();
        assert_eq!(
            proxy.accept().unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        return;
    }
    let workspace = Workspace::approved(&release_plan());
    let (mock, _) = github_mock(Github {
        by_tag: Some(published_release()),
        ..Github::default()
    })
    .await;
    assert_eq!(mock.run(&workspace).await[0].status, "public");
    assert_eq!(mock.calls().len(), 2);
}

#[test]
fn cli_run_returns_status_without_opening_the_business_store() {
    let workspace = Workspace::new(&bluesky_plan("example post"));
    let execute = || {
        Command::new(env!("CARGO_BIN_EXE_kirzner"))
            .args(["campaign", "run", FILE])
            .env_remove("HTTPS_PROXY")
            .env_remove("https_proxy")
            .env("KIRZNER_DATA_DIR", workspace.0.join("never-open"))
            .current_dir(&workspace.0)
            .output()
            .unwrap()
    };
    let output = execute();
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["status"], "unapproved");

    workspace.write(&bluesky_plan("example post").replace(&format!("bluesky/{HANDLE}"), "manual"));
    let output = execute();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!([])
    );
    assert!(!workspace.0.join("never-open").exists());
}
