use kirzner::campaign::{
    Approval, ExecutionEntry, Intent, append_execution, approval_batch, approve_campaign, approved,
    campaign_approve, campaign_plan, campaign_status, in_window, parse_campaign, read_campaign,
    with_campaign,
};
use std::{
    fs,
    panic::catch_unwind,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const CAMPAIGN: &str = "---\nschema: 1\ncampaign: week48\nitems:\n  - id: d1\n    content: post\n    target: bluesky/test.example\n    steps: [publish]\n    at: '2026-11-28T10:00:00Z'\n---\n\n## post\n\nhello 🌎\n";
const NAME: &str = "campaign.md";
const RELEASE: &str = "---\nschema: 1\ncampaign: release-demo\nitems:\n  - id: d0\n    content: release\n    target: manual\n    steps: [save_draft, publish]\n  - id: d1\n    content: release\n    target: github/Org/My_Repo.Test\n    account: Release-BOT[BOT]\n    steps: [publish]\n    tag: v1.2.3\n---\n\n## release\n\nTitle:\t First release  \n\n   \n\nChanges\n\n\n- Ships 🌎\n";

struct Workspace(PathBuf);

impl Workspace {
    fn new(text: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "kirzner-campaign-{}-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let workspace = Self(dir);
        fs::write(workspace.0.join(NAME), text).unwrap();
        workspace
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn execution(body: &str) -> String {
    format!("{CAMPAIGN}\n## Execution\n\n```yaml\n{body}\n```\n")
}

#[test]
fn bluesky_hash_matches_artipub_vector_and_preserves_timestamp() {
    let campaign = parse_campaign(CAMPAIGN).unwrap();
    let (actions, digest, _) = approval_batch(&campaign).unwrap();
    assert_eq!(
        actions[0].request_key,
        "sha256:3796789d6ea55a73c6e8215c91139781f346fdb1cc8eb0b665071ea0ee9546d3"
    );
    assert_eq!(
        actions[0].version,
        "sha256:a5c9b64d1a47d9a55cbd825264202599c0d5dab19d01d4c75949ab1b4b1a4c5e"
    );
    assert_eq!(
        digest,
        "sha256:c52418126c51db325ed791486fb6df4cd93779cfe7ddc4bf90a1378ced2026bd"
    );
}

#[test]
fn rejects_yaml_aliases_and_anchors_at_event_level() {
    for yaml in [
        "metrics: [&a, *a]",
        "metrics: {a: &a [1], b: *a}",
        "metrics:\n  b:\n    *a",
        "goal: !!str &a x",
        "metrics: &a {test: 1}",
    ] {
        let source = CAMPAIGN.replacen("items:", &format!("{yaml}\nitems:"), 1);
        assert!(
            parse_campaign(&source)
                .unwrap_err()
                .contains("aliases and anchors"),
            "{source}"
        );
    }
    assert!(
        parse_campaign(&execution("observations: [*a]"))
            .unwrap_err()
            .contains("aliases and anchors")
    );
}

#[test]
fn yaml_literals_and_indented_front_matter_delimiter_are_preserved() {
    let source = execution("record:\n  item: d1\n  milestone: public\n  url: https://example.com/a?b=c\n  reason: \"foo: *bar and: &baz\"")
        .replacen("items:", "goal: |-\n  ---\n  : *literal\n  : &literal\nitems:", 1);
    let campaign = parse_campaign(&source).unwrap();
    assert_eq!(
        campaign.plan.goal.as_deref(),
        Some("---\n: *literal\n: &literal")
    );
    assert_eq!(
        campaign.records[0].reason.as_deref(),
        Some("foo: *bar and: &baz")
    );
    assert_eq!(
        campaign.records[0].url.as_deref(),
        Some("https://example.com/a?b=c")
    );
}

#[test]
fn markdown_headings_and_fences_match_artipub() {
    let payload = "hello\n    ## post\n    ```yaml\n```inline```\n   ```md\n## post\n   ````\n";
    let source = CAMPAIGN.replace(
        "## post\n\nhello 🌎\n",
        &format!("   ##\tpost ###  \n\n{payload}\n# Next\n\nignored\n## post#\nother\n"),
    );
    let campaign = parse_campaign(&source).unwrap();
    assert_eq!(campaign.payloads["post"], payload.trim_end());
    assert!(parse_campaign(&CAMPAIGN.replace("## post", "## post#")).is_err());
    let source = format!(
        "{}# End\n```yaml\ninvalid: true\n```\n",
        execution("observations: []")
    );
    parse_campaign(&source).unwrap();
}

#[test]
fn reads_legacy_records_arrays_and_postiz_intents() {
    let text = format!(
        "{CAMPAIGN}\n## Execution\n\n```yaml\nrecords:\n  - item: d1\n    milestone: prepared\n    intent:\n      provider: postiz\n      account: legacy\n      text: old text\n      integration_id: i1\n      actor_id: a1\n      instance: 'sha256:{}'\n```\n",
        "0".repeat(64)
    );
    let parsed = parse_campaign(&text).unwrap();
    assert!(
        matches!(&parsed.records[0].intent, Some(Intent::Retired(intent)) if intent.provider == "postiz")
    );
    for key in [
        "2222222222222".into(),
        format!("kirzner-{}", "a".repeat(64)),
    ] {
        let body = format!(
            "record:\n  item: d1\n  milestone: public\n  intent:\n    account: test.example\n    did: did:plc:abc234\n    pds: https://example.com\n    rkey: {key}\n    text: old text"
        );
        // A digits-only TID is a string in the execution schema.
        let body = body.replace(&format!("rkey: {key}"), &format!("rkey: '{key}'"));
        let parsed = parse_campaign(&execution(&body)).unwrap();
        assert!(
            matches!(&parsed.records[0].intent, Some(Intent::Bluesky(intent)) if intent.rkey == key)
        );
    }
}

#[test]
fn rejects_invalid_plan_and_execution_fields() {
    for (from, to) in [
        ("content: post", "content: ../post"),
        ("[publish]", "[publish, publish]"),
        ("items:", "metrics: [one, two]\nitems:"),
        (
            "steps:",
            "assets: [{path: post.md, sha256: abc, extra: false}]\n    steps:",
        ),
        ("steps:", "assets: [{path: '', sha256: abc}]\n    steps:"),
    ] {
        assert!(parse_campaign(&CAMPAIGN.replace(from, to)).is_err(), "{to}");
    }
    for (field, value) in [
        ("item", "-d1"),
        ("milestone", "deleted"),
        ("status", "succeeded"),
        ("step", "deploy"),
        ("version", "sha256:abc"),
        ("request_key", "abc"),
        ("url", "./post"),
        ("url", "null"),
        ("cid", "''"),
        ("attempted_at", "2026-11-28T10:00:00+00:00"),
        ("attempted_at", "2026-11-28t10:00:00Z"),
        ("observed_at", "2026-11-28T10:00:00z"),
        (
            "intent",
            "{repo: invalid, account: user, tag: v1, title: Release, body: ''}",
        ),
        (
            "intent",
            "{account: test.example, did: did:plc:abcd, pds: https://example.com/, rkey: abcdefghijklm, text: old}",
        ),
        (
            "intent",
            "{account: test.example, did: did:plc:abcd, pds: https://example.com, rkey: zbcdefghijklm, text: old}",
        ),
        (
            "intent",
            "{provider: postiz, account: old, text: old, integration_id: i, actor_id: a, instance: bad}",
        ),
    ] {
        let mut body = "record:\n  item: d1\n  milestone: public".to_owned();
        match field {
            "item" => body = body.replace("item: d1", &format!("item: {value}")),
            "milestone" => body = body.replace("milestone: public", &format!("milestone: {value}")),
            _ => body.push_str(&format!("\n  {field}: {value}")),
        }
        assert!(parse_campaign(&execution(&body)).is_err(), "{body}");
    }
    for observations in ["[no]", "[[]]", "[{false: 1}]"] {
        assert!(parse_campaign(&execution(&format!("observations: {observations}"))).is_err());
    }
    let campaign = parse_campaign(CAMPAIGN).unwrap();
    let digest = format!("sha256:{}", "a".repeat(64));
    for item in ["", "-bad"] {
        let entry = ExecutionEntry {
            approval: Some(Approval {
                items: vec![item.into()],
                hashes: vec![digest.clone()],
                digest: digest.clone(),
                expires_at: None,
            }),
            ..Default::default()
        };
        assert!(append_execution(&campaign, &entry).is_err());
        let yaml = serde_yaml::to_string(&entry).unwrap();
        assert!(parse_campaign(&execution(&yaml)).is_err());
    }
    let invalid: ExecutionEntry =
        serde_yaml::from_str("record: {item: d1, milestone: public, version: abc}").unwrap();
    assert!(append_execution(&campaign, &invalid).is_err());
}

#[test]
fn github_release_hash_matches_artipub_vectors() {
    let extended = RELEASE.replacen("    tag: v1.2.3",
        "    tag: v1.2.3\n    commit: abcdef0123456789abcdef0123456789abcdef01\n    after: d0\n    at: '2027-01-01T13:00:30.123Z'", 1);
    for (source, version, request, batch) in [
        (
            RELEASE,
            "sha256:c198ecfce237a2aedb7460a9d6015406cd125bc4643ed7cbc3a435d81e21aaab",
            "sha256:dc8f20dc95ffafe543fe7b8979e98a873256445bafa20f6befea124fb1f1ec6c",
            "sha256:dacb73037b6034c5acd00d1df45a8ad196e998a3fd12e4fe64aa6a0bbec464c6",
        ),
        (
            &extended,
            "sha256:ad7aa1686cebb581208ec9fba789ab80bc13842aec3737365399fe5a4aec5638",
            "sha256:41207f3e79e99ed6eae7432091b51dbe6c3a4d54c8d1cf871dbe1f5a01af47b5",
            "sha256:6325b8f9c39f93edd778429aaf579bf2437b82179e35dafc60c27ac6f6654230",
        ),
    ] {
        let (actions, digest, _) = approval_batch(&parse_campaign(source).unwrap()).unwrap();
        let [action] = actions.as_slice() else {
            panic!("Expected one release action")
        };
        assert_eq!(action.title.as_deref(), Some("First release"));
        assert_eq!(action.body.as_deref(), Some("Changes\n\n\n- Ships 🌎"));
        assert_eq!(action.account.as_deref(), Some("release-bot[bot]"));
        assert_eq!(action.remote_key, "github/org/my_repo.test@v1.2.3");
        assert_eq!(action.version, version);
        assert_eq!(action.request_key, request);
        assert_eq!(digest, batch);
    }
    for (before, after) in [
        ("Org/My_Repo.Test", "Org/.."),
        ("Org/My_Repo.Test", "Org/Repo/Extra"),
        ("Release-BOT[BOT]", "-bad"),
        ("Release-BOT[BOT]", "invalid_account"),
        ("v1.2.3", "v1..2"),
        ("v1.2.3", "v1/test.lock"),
    ] {
        let campaign = parse_campaign(&RELEASE.replace(before, after)).unwrap();
        assert!(approval_batch(&campaign).is_err(), "{after}");
    }
}

#[test]
fn unholding_an_item_keeps_approval_for_previously_reviewed_hashes() {
    let source = CAMPAIGN.replace("items:", "window_utc: ['2026-01-01T00:00Z', '2099-01-01T00:00:00Z']\nitems:")
        .replacen("\n---\n", "\n  - id: d2\n    content: next\n    target: bluesky/test.example\n    steps: [publish]\n    status: hold\n---\n", 1);
    let source = format!("{source}\n## next\nA held post\n");
    let held = parse_campaign(&source).unwrap();
    let (held_actions, held_digest, _) = approval_batch(&held).unwrap();
    assert_eq!(held_actions.len(), 1);
    let approval = approve_campaign(&held, &held_digest).unwrap();
    let text = append_execution(
        &held,
        &ExecutionEntry {
            approval: Some(approval),
            ..Default::default()
        },
    )
    .unwrap();
    let unheld = parse_campaign(&text.replace("    status: hold\n", "")).unwrap();
    let (actions, digest, expiry) = approval_batch(&unheld).unwrap();
    assert_ne!(held_digest, digest);
    assert!(approved(&unheld, expiry.as_deref(), &actions[0]));
    assert!(!approved(&unheld, expiry.as_deref(), &actions[1]));
    assert!(!approved(&unheld, None, &actions[0]));
    for (now, expected) in [
        ("2025-12-31T23:59:59Z", false),
        ("2026-01-01T00:00:00Z", true),
        ("2099-01-01T00:00:00Z", false),
    ] {
        assert_eq!(
            in_window(&unheld, OffsetDateTime::parse(now, &Rfc3339).unwrap()),
            expected
        );
    }
}

#[test]
fn inserts_indented_approval_fence_and_preserves_crlf() {
    let source = CAMPAIGN.replace('\n', "\r\n");
    let campaign = parse_campaign(&source).unwrap();
    let (_, digest, _) = approval_batch(&campaign).unwrap();
    let approval = approve_campaign(&campaign, &digest).unwrap();
    let output = append_execution(
        &campaign,
        &ExecutionEntry {
            approval: Some(approval),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        output.contains("## Execution\r\n\r\n```yaml\r\napproval:\r\n  items:\r\n    - d1\r\n")
    );
    assert!(!output.replace("\r\n", "").contains('\n'));
    assert_eq!(
        parse_campaign(&output).unwrap().approval.unwrap().digest,
        digest
    );
}

#[test]
fn lock_is_exclusive_but_reads_work_and_errors_and_panics_release_it() {
    let workspace = Workspace::new(CAMPAIGN);
    let lock = workspace.0.join(".kirzner.lock");

    let result = with_campaign::<()>(&workspace.0, NAME, |_| {
        assert!(lock.is_dir());
        let err = with_campaign(&workspace.0, NAME, |_| Ok(())).unwrap_err();
        assert!(err.contains("Workspace is locked"));
        assert!(campaign_plan(&workspace.0, NAME).is_ok());
        assert!(campaign_status(&workspace.0, NAME).is_ok());
        Err("operation failed".into())
    });
    assert_eq!(result.unwrap_err(), "operation failed");
    assert!(!lock.exists());
    assert!(with_campaign(&workspace.0, "missing.md", |_| Ok(())).is_err());
    assert!(!lock.exists());

    let panic = catch_unwind(|| {
        let _ = with_campaign::<()>(&workspace.0, NAME, |_| panic!("operator panicked"));
    });
    assert!(panic.is_err());
    assert!(!lock.exists());
}

#[test]
fn stale_append_preserves_external_changes_and_removes_temp_file() {
    let workspace = Workspace::new(CAMPAIGN);
    let path = workspace.0.join(NAME);
    let changed = format!("{CAMPAIGN}\nExternal edit\n");
    let error = with_campaign(&workspace.0, NAME, |store| {
        fs::write(&path, &changed).unwrap();
        store.append(&ExecutionEntry::default())
    })
    .unwrap_err();
    assert!(error.contains("Campaign changed during execution"));
    assert_eq!(fs::read_to_string(path).unwrap(), changed);
    assert_eq!(fs::read_dir(&workspace.0).unwrap().count(), 1);
}

#[test]
fn pauses_gate_store_but_approval_remains_available() {
    let workspace = Workspace::new(CAMPAIGN);
    let pending = |store: &mut kirzner::campaign::CampaignStore| store.paused();
    assert!(!with_campaign(&workspace.0, NAME, pending).unwrap());

    let global = workspace.0.join(".kirzner.pause");
    fs::write(&global, "").unwrap();
    assert!(with_campaign(&workspace.0, NAME, pending).unwrap());
    fs::remove_file(global).unwrap();
    let local = workspace.0.join(format!("{NAME}.pause"));
    #[cfg(unix)]
    std::os::unix::fs::symlink("missing-pause-target", &local).unwrap();
    #[cfg(not(unix))]
    fs::write(&local, "").unwrap();
    assert!(with_campaign(&workspace.0, NAME, pending).unwrap());
    let (_, digest, _) = approval_batch(&parse_campaign(CAMPAIGN).unwrap()).unwrap();
    campaign_approve(&workspace.0, NAME, &digest).unwrap();
    let campaign = read_campaign(&workspace.0, NAME).unwrap();
    assert_eq!(campaign.approval.unwrap().digest, digest);
    assert!(fs::symlink_metadata(local).is_ok());
}

#[test]
fn campaign_paths_must_be_regular_workspace_markdown_files() {
    let workspace = Workspace::new(CAMPAIGN);
    for name in ["../foo.md", "/tmp/foo.md", "foo.txt", "-foo.md"] {
        assert!(
            read_campaign(&workspace.0, name)
                .unwrap_err()
                .contains("Markdown filename")
        );
        assert!(with_campaign(&workspace.0, name, |_| Ok(())).is_err());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(NAME, workspace.0.join("linked.md")).unwrap();
        assert!(
            read_campaign(&workspace.0, "linked.md")
                .unwrap_err()
                .contains("regular file")
        );
        assert!(with_campaign(&workspace.0, "linked.md", |_| Ok(())).is_err());
    }
}

#[cfg(unix)]
#[test]
fn append_updates_store_and_preserves_file_mode() {
    use std::os::unix::fs::PermissionsExt;
    let workspace = Workspace::new(CAMPAIGN);
    let path = workspace.0.join(NAME);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let entry: ExecutionEntry =
        serde_yaml::from_str("record: {item: d1, milestone: public}").unwrap();
    with_campaign(&workspace.0, NAME, |store| {
        let lock_mode = fs::metadata(workspace.0.join(".kirzner.lock"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(lock_mode & 0o077, 0);
        store.append(&entry)?;
        store.append(&entry)?;
        assert_eq!(store.campaign().records.len(), 2);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(read_campaign(&workspace.0, NAME).unwrap().records.len(), 2);
}

#[test]
fn status_reports_the_earliest_instant_instead_of_string_order() {
    let source = CAMPAIGN
        .replacen("2026-11-28T10:00:00Z", "2099-01-01T10:00:00.1Z", 1)
        .replacen(
            "\n---\n",
            "\n  - id: d2\n    target: manual\n    at: '2099-01-01T10:00:00Z'\n---\n",
            1,
        );
    let workspace = Workspace::new(&source);
    assert_eq!(
        campaign_status(&workspace.0, NAME).unwrap()["next_due_at"],
        "2099-01-01T10:00:00Z"
    );
}

#[cfg(unix)]
#[test]
fn approval_and_record_pass_release_script_awk_fence_validation() {
    // Execution-fence validation from release-next-hold.sh, applied to synthetic data.
    const VALIDATOR: &str = r###"
        in_fence {
          if ($0 == "```") {
            if (kind == "record:") {
              if (item == "" || milestone == "") bad_execution = 1
              latest[item] = milestone
            } else if (kind != "approval:") bad_execution = 1
            in_fence = 0
            next
          }
          if (!kind) { kind = $0; next }
          if (kind == "record:") {
            if (/^  item: d[1-4]$/ && item == "") { item = $2; next }
            if (/^  milestone: [a-z_]+$/ && milestone == "") { milestone = $2; next }
            if (/^  (item|milestone):/ || !/^($|    |  [a-z_]+:)/) bad_execution = 1
          } else if (kind == "approval:" && !/^($|    |  [a-z_]+:)/) bad_execution = 1
          next
        }
        $0 == "## Execution" {
          if (++execution_count != 1) bad_execution = 1
          execution = 1
          next
        }
        execution && $0 == "```yaml" {
          in_fence = 1; kind = item = milestone = ""
          next
        }
        /^##? / { execution = 0; next }
        /^[ ]*[`~][`~][`~]/ || (execution && NF) { bad_execution = 1 }
        END { exit (bad_execution || in_fence || execution_count != 1 || latest["d1"] != "public") }
    "###;

    let campaign = parse_campaign(CAMPAIGN).unwrap();
    let (_, digest, _) = approval_batch(&campaign).unwrap();
    let approval = ExecutionEntry {
        approval: Some(approve_campaign(&campaign, &digest).unwrap()),
        ..Default::default()
    };
    let first = parse_campaign(&append_execution(&campaign, &approval).unwrap()).unwrap();
    let record: ExecutionEntry = serde_yaml::from_str(
        "record:\n  item: d1\n  milestone: public\n  intent:\n    repo: test/release\n    account: test\n    tag: v1\n    title: Release\n    body: |\n      line one\n      - line two\n      ```\n",
    )
    .unwrap();
    let output = append_execution(&first, &record).unwrap();
    let parsed = parse_campaign(&output).unwrap();
    assert!(
        matches!(&parsed.records[0].intent, Some(Intent::Release(intent)) if intent.body == "line one\n- line two\n```\n")
    );
    let workspace = Workspace::new(&output);
    let validate = || {
        Command::new("awk")
            .env("LC_ALL", "C")
            .arg(VALIDATOR)
            .arg(workspace.0.join(NAME))
            .status()
            .unwrap()
    };
    assert!(validate().success());
    fs::write(workspace.0.join(NAME), output.replace("    - d1", "  - d1")).unwrap();
    assert!(!validate().success());
}

#[test]
fn campaign_cli_commands_do_not_initialize_data_storage() {
    let workspace = Workspace::new(CAMPAIGN);
    let data = workspace.0.join("unused-data");
    let run = |verb| {
        Command::new(env!("CARGO_BIN_EXE_kirzner"))
            .current_dir(&workspace.0)
            .env("KIRZNER_DATA_DIR", &data)
            .args(["campaign", verb, NAME])
            .output()
            .unwrap()
    };
    for (verb, key) in [("plan", "actions"), ("status", "items")] {
        let output = run(verb);
        assert!(output.status.success(), "{:?}", output.stderr);
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(json[key].as_array().unwrap().len(), 1);
    }
    let rejected = run("approve");
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("interactive operator terminal"));
    assert_eq!(
        fs::read_to_string(workspace.0.join(NAME)).unwrap(),
        CAMPAIGN
    );
    assert!(!data.exists());
    assert!(!workspace.0.join("data").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn campaign_cli_approves_only_runnable_batches_on_a_terminal() {
    use std::{io::Write, process::Stdio};

    let workspace = Workspace::new(CAMPAIGN);
    let data = workspace.0.join("unused-data");
    let run = |answer: &[u8]| {
        let mut child = Command::new("script")
            .args([
                "-qec",
                r#"exec "$KIRZNER_TEST_BIN" campaign approve campaign.md"#,
                "/dev/null",
            ])
            .current_dir(&workspace.0)
            .env("KIRZNER_TEST_BIN", env!("CARGO_BIN_EXE_kirzner"))
            .env("KIRZNER_DATA_DIR", &data)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        if let Err(error) = child.stdin.take().unwrap().write_all(answer) {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
        child.wait_with_output().unwrap()
    };
    let approved = run(b"y\n");
    assert!(approved.status.success(), "{:?}", approved.stdout);
    assert!(
        read_campaign(&workspace.0, NAME)
            .unwrap()
            .approval
            .is_some()
    );
    fs::write(
        workspace.0.join(NAME),
        CAMPAIGN.replace("    at:", "    status: hold\n    at:"),
    )
    .unwrap();

    let empty = run(b"y\n");
    assert!(!empty.status.success());
    let output = String::from_utf8_lossy(&empty.stdout);
    assert!(output.contains("No runnable campaign actions to approve"));
    assert!(!output.contains("Approve this batch?"));
    assert!(!data.exists());
    assert!(!workspace.0.join("data").exists());
}
