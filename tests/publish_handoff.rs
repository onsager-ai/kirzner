use kirzner::{
    BusinessStore, marketing,
    model::{Brief, Command, ProposalInput},
    publish::{Article, Bundle, GenericAdapter},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command as Process, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "kirzner-publish-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn source(dir: &Path, article: &str) -> PathBuf {
    let path = dir.join("fixture.md");
    fs::write(&path, article).unwrap();
    path
}

#[test]
fn imports_frontmatter_and_requires_a_title() {
    let dir = TestDirectory::new();
    for (input, tags) in [
        ("tags: guide", vec!["guide"]),
        (
            "tags: [guide, 'owner''s guide, part two']",
            vec!["guide", "owner's guide, part two"],
        ),
        ("tags:\n  - guide\n  - release", vec!["guide", "release"]),
    ] {
        let path = source(
            &dir.0,
            &format!(
                "---\ntitle: 'Guide: local'\nsummary: A guide\ncategory: Work\n{input}\n---\nBody\n"
            ),
        );
        let article = Article::read(&path).unwrap();
        assert_eq!(article.metadata().title, "Guide: local");
        assert_eq!(article.metadata().summary.as_deref(), Some("A guide"));
        assert_eq!(article.metadata().category.as_deref(), Some("Work"));
        assert_eq!(article.metadata().tags, tags);
        assert_eq!(article.markdown(), "Body\n");
    }
    for frontmatter in ["summary: Missing title", "title: ' '"] {
        let path = source(&dir.0, &format!("---\n{frontmatter}\n---\nBody"));
        assert!(Article::read(&path).is_err());
    }
}

#[test]
fn orders_images_and_writes_identical_bundles_in_separate_directories() {
    let first = TestDirectory::new();
    let second = TestDirectory::new();
    let markdown = concat!(
        "---\r\ntitle: Fixture\r\ntags: [a, b]\r\ncover: cover.JPG\r\n---\r\n",
        "![Second alphabetically](z.png) ![First alphabetically](<a space.webp> \"Title\")\r\n",
        "![Repeated](z.png)\r\n",
        "`![Code](missing.png)`\r\n```md\r\n![Fence](missing.png)\r\n```\r\n"
    );
    let fixture = |dir: &Path| {
        fs::write(dir.join("z.png"), b"source bytes").unwrap();
        fs::write(dir.join("a space.webp"), b"other source bytes").unwrap();
        fs::write(dir.join("cover.JPG"), b"cover bytes").unwrap();
        Article::read(&source(dir, markdown)).unwrap()
    };
    let article = fixture(&first.0);
    let copy = fixture(&second.0);
    assert_eq!(article.metadata(), copy.metadata());
    assert_eq!(
        article.metadata().source_sha256,
        format!("{:x}", Sha256::digest(markdown))
    );
    let a = Bundle::build(&article, &GenericAdapter).unwrap();
    let b = Bundle::build(&copy, &GenericAdapter).unwrap();
    assert_eq!(a.digest(), b.digest());
    let a_out = a.write(&first.0.join("out")).unwrap();
    let b_out = b.write(&second.0.join("out")).unwrap();
    assert_eq!(a.write(&first.0.join("out")).unwrap(), a_out);
    for file in [
        "article.md",
        "manifest.json",
        "checklist.md",
        "assets/002-z.png",
        "assets/003-a-space.webp",
        "assets/001-cover.jpg",
    ] {
        assert_eq!(
            fs::read(a_out.join(file)).unwrap(),
            fs::read(b_out.join(file)).unwrap()
        );
    }
    let rewritten = fs::read_to_string(a_out.join("article.md")).unwrap();
    assert!(!rewritten.contains('\r'));
    assert!(rewritten.contains("![First alphabetically](<assets/003-a-space.webp> \"Title\")"));
    assert_eq!(rewritten.matches("assets/002-z.png").count(), 2);
    assert!(rewritten.contains("```md\n![Fence](missing.png)\n```"));
    assert_eq!(
        fs::read(a_out.join("assets/002-z.png")).unwrap(),
        b"source bytes"
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(a_out.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["article_id"], article.metadata().id);
    assert_eq!(manifest["cover"], "assets/001-cover.jpg");
    assert_eq!(manifest["platform"], "generic");
    assert_eq!(manifest["tags"], serde_json::json!(["a", "b"]));
    let checklist = fs::read_to_string(a_out.join("checklist.md")).unwrap();
    assert!(checklist.contains("- [ ] Paste the live public URL"));
    assert!(checklist.contains("- Cover: assets/001-cover.jpg"));
    assert_eq!(manifest["assets"][1]["media_type"], "image/png");
    assert_eq!(manifest["assets"][1]["bytes"], 12);
    assert_eq!(manifest["assets"][1]["order"], 2);
    assert_eq!(
        manifest["assets"][1]["sha256"],
        format!("{:x}", Sha256::digest(b"source bytes"))
    );
    assert_eq!(
        manifest["normalized_body_sha256"],
        format!("{:x}", Sha256::digest(rewritten))
    );
    fs::write(a_out.join("article.md"), "local edit").unwrap();
    assert!(a.write(&first.0.join("out")).is_err());
    assert_eq!(
        fs::read_to_string(a_out.join("article.md")).unwrap(),
        "local edit"
    );
}

#[test]
fn rejects_nonlocal_missing_and_unsupported_images() {
    let dir = TestDirectory::new();
    for destination in [
        "../secret.png",
        "sub/../../secret.png",
        "%2e%2e/secret.png",
        "/absolute.png",
        "C:/absolute.png",
        "//example.com/image.png",
        "https://example.com/image.png",
        "http://example.com/image.png",
        "data:image/png;base64,abc",
        "missing.png",
        "notes.txt",
    ] {
        let input = format!("---\ntitle: Fixture\n---\n![x]({destination})");
        let path = source(&dir.0, &input);
        assert!(Article::read(&path).is_err(), "Accepted {destination}");
    }
    let path = source(&dir.0, "---\ntitle: Fixture\ncover: missing.png\n---\nBody");
    assert!(Article::read(&path).is_err());
}

#[cfg(unix)]
#[test]
fn rejects_an_existing_symlink_to_an_image_outside_the_article_directory() {
    let dir = TestDirectory::new();
    let other = TestDirectory::new();
    let target = other.0.join("secret.png");
    fs::write(&target, b"outside article directory").unwrap();
    std::os::unix::fs::symlink(target, dir.0.join("escape.png")).unwrap();
    let path = source(&dir.0, "---\ntitle: Fixture\n---\n![x](escape.png)");
    assert!(Article::read(&path).is_err());
}

struct Fixture {
    dir: TestDirectory,
    store: BusinessStore,
}

impl Fixture {
    async fn new() -> Self {
        let dir = TestDirectory::new();
        let sqlite = dir.0.join("kirzner.sqlite3");
        let store = BusinessStore::connect(&format!("sqlite://{}", sqlite.display()))
            .await
            .unwrap();
        let fixture = Self { dir, store };
        fixture
            .apply(Command::SaveBrief {
                brief: Brief {
                    product: "Fixture product".into(),
                    objective: "Test publication handoff".into(),
                    constraints: "Synthetic only".into(),
                    ..Brief::default()
                },
            })
            .await;
        fixture
            .apply(Command::CreateExperiment {
                opportunity_id: None,
                proposal: proposal("Draft one"),
            })
            .await;
        fixture
    }

    async fn apply(&self, command: Command) {
        let mut workspace = self.store.load_workspace("local").await.unwrap();
        let expected = workspace.revision;
        assert!(marketing::apply(&mut workspace, command).unwrap().changed);
        workspace.revision += 1;
        self.store
            .commit_workspace(expected, &workspace)
            .await
            .unwrap();
    }

    async fn decide(&self, version: u32, approved: bool) {
        self.apply(Command::DecideProposal {
            experiment_id: "experiment-1".into(),
            version,
            approved,
            reason: "Synthetic decision".into(),
        })
        .await;
    }

    fn cli(&self, args: &[&OsStr]) -> Output {
        Process::new(env!("CARGO_BIN_EXE_kirzner"))
            .env("KIRZNER_DATA_DIR", &self.dir.0)
            .args(args)
            .output()
            .unwrap()
    }

    fn package(&self, version: &str, article: &OsStr, out: &Path) -> Output {
        self.cli(&[
            OsStr::new("publish"),
            OsStr::new("package"),
            OsStr::new("--experiment"),
            OsStr::new("experiment-1"),
            OsStr::new("--version"),
            OsStr::new(version),
            OsStr::new("--article"),
            article,
            OsStr::new("--platform"),
            OsStr::new("generic"),
            OsStr::new("--out"),
            out.as_os_str(),
        ])
    }

    fn record(&self, version: &str, digest: &str, url: &str) -> Output {
        let args = [
            "publish",
            "record",
            "--experiment",
            "experiment-1",
            "--version",
            version,
            "--package-digest",
            digest,
            "--url",
            url,
        ];
        self.cli(&args.map(OsStr::new))
    }
}

fn proposal(action: &str) -> ProposalInput {
    ProposalInput {
        audience: "Synthetic audience".into(),
        action: action.into(),
        expected_deliverables: "Local article package".into(),
        observation_window: "One week".into(),
        success_criteria: "A founder records publication".into(),
        failure_criteria: "The founder declines publication".into(),
        inconclusive_criteria: "No evidence".into(),
        resource_limits: "No spend".into(),
        authorization_scope: "Founder publishes manually".into(),
    }
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
async fn cli_packages_only_approved_current_versions_and_records_matching_urls() {
    let fixture = Fixture::new().await;
    let source = source(
        &fixture.dir.0,
        "---\ntitle: Fixture\n---\nArticle body\n![Fixture](feature.png)\n",
    );
    let image = fixture.dir.0.join("feature.png");
    fs::write(&image, b"synthetic asset").unwrap();
    let out = fixture.dir.0.join("out");
    assert!(
        !fixture
            .package("1", source.as_os_str(), &out)
            .status
            .success()
    );
    assert!(!out.exists());
    fixture.decide(1, true).await;
    fixture.decide(1, false).await;
    assert!(
        !fixture
            .package("1", source.as_os_str(), &out)
            .status
            .success()
    );
    fixture.decide(1, true).await;
    fixture
        .apply(Command::ReviseProposal {
            experiment_id: "experiment-1".into(),
            proposal: proposal("Draft two"),
        })
        .await;
    assert!(
        !fixture
            .package("1", source.as_os_str(), &out)
            .status
            .success()
    );
    assert!(!out.exists());
    fixture.decide(2, true).await;
    let imported = stdout(fixture.cli(&[
        OsStr::new("article"),
        OsStr::new("import"),
        source.as_os_str(),
    ]));
    let article_id = imported.split_whitespace().next().unwrap();
    let packaged = stdout(fixture.package("2", source.as_os_str(), &out));
    let mut fields = packaged.split_whitespace();
    let attempt_id = fields.next().unwrap();
    let digest = fields.next().unwrap();
    let workspace = fixture.store.load_workspace("local").await.unwrap();
    assert_eq!(workspace.articles.len(), 1);
    assert_eq!(workspace.articles[0].id, article_id);
    assert!(workspace.experiments[0].actions.is_empty());
    assert_eq!(workspace.attempts.len(), 1);
    let attempt = &workspace.attempts[0];
    assert_eq!(attempt.kind, "publish_package");
    assert_eq!(attempt.id, attempt_id);
    assert_eq!(attempt.proposal_version, Some(2));
    assert_eq!(
        attempt.snapshot["proposal"]["content"]["action"],
        "Draft two"
    );
    assert_eq!(attempt.publish_package.as_ref().unwrap().digest, digest);
    assert!(attempt.returns.is_empty());
    let stored = fs::read(fixture.dir.0.join("files").join(digest)).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(stored)), digest);
    fs::remove_file(source).unwrap();
    fs::remove_file(image).unwrap();
    let retry = stdout(fixture.package("2", OsStr::new(article_id), &out));
    assert_eq!(retry, packaged);
    assert_eq!(
        fixture.store.load_workspace("local").await.unwrap(),
        workspace
    );
    for url in [
        "http://example.com/article",
        "https:///article",
        "https://",
        "https://example.com:bad/article",
        "https://example.com/a b",
        "https://user@example.com/article",
        "https://example.com/%xy",
    ] {
        assert!(
            !fixture.record("2", digest, url).status.success(),
            "Accepted {url}"
        );
    }
    let url = "https://example.com:443/article?x=1#body";
    assert!(!fixture.record("1", digest, url).status.success());
    assert!(!fixture.record("2", &"0".repeat(64), url).status.success());
    assert_eq!(
        fixture.store.load_workspace("local").await.unwrap(),
        workspace
    );
    let recorded = stdout(fixture.record("2", digest, url));
    assert_eq!(recorded, format!("action-1 {url}\n"));
    assert_eq!(stdout(fixture.record("2", digest, url)), recorded);
    assert!(
        !fixture
            .record("2", digest, "https://example.com/different")
            .status
            .success()
    );
    let workspace = fixture.store.load_workspace("local").await.unwrap();
    assert_eq!(workspace.experiments[0].actions.len(), 1);
    let action = &workspace.experiments[0].actions[0];
    assert_eq!(action.version, 2);
    assert_eq!(action.reference, url);
    assert_eq!(action.publish_attempt_id.as_deref(), Some(attempt_id));
    assert!(action.note.contains(digest));
    assert!(workspace.experiments[0].observations.is_empty());
}
