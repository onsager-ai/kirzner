use kirzner::campaign::{
    Approval, ExecutionEntry, append_execution, approval_batch, parse_campaign,
};

const CAMPAIGN: &str = "---\nschema: 1\ncampaign: week48\nitems:\n  - id: d1\n    content: post\n    target: bluesky/test.example\n    steps: [publish]\n    at: '2026-11-28T10:00:00Z'\n---\n\n## post\n\nhello 🌎\n";

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
fn rejects_yaml_aliases() {
    let text = "---\nschema: 1\ncampaign: x\nitems: &items\n  - id: d1\n    target: x\n---\n";
    assert!(
        parse_campaign(text)
            .unwrap_err()
            .contains("aliases and anchors")
    );
}

#[test]
fn reads_legacy_records_arrays_and_postiz_intents() {
    let text = format!(
        "{CAMPAIGN}\n## Execution\n\n```yaml\nrecords:\n  - item: d1\n    milestone: prepared\n    intent:\n      provider: postiz\n      account: legacy\n      text: old text\n      integration_id: i1\n      actor_id: a1\n      instance: 'sha256:{}'\n```\n",
        "0".repeat(64)
    );
    let parsed = parse_campaign(&text).unwrap();
    assert_eq!(parsed.records.len(), 1);
}

#[test]
fn inserts_awk_compatible_approval_fence_and_preserves_crlf() {
    let source = CAMPAIGN.replace('\n', "\r\n");
    let campaign = parse_campaign(&source).unwrap();
    let output = append_execution(
        &campaign,
        &ExecutionEntry {
            approval: Some(Approval {
                items: vec!["d1".into()],
                hashes: vec!["sha256:abc".into()],
                digest: "sha256:def".into(),
                expires_at: None,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        output.contains("## Execution\r\n\r\n```yaml\r\napproval:\r\n  items:\r\n    - d1\r\n")
    );
    assert!(!output.replace("\r\n", "").contains('\n'));
}
