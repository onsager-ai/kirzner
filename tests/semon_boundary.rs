//! Synthetic adapter acceptance, never evidence of a real harness session.
use kirzner::semon::SemonConnection;
use serde_json::json;
use std::{path::PathBuf, process::Command};

#[tokio::test]
#[ignore = "requires KIRZNER_TEST_SEMON_CODEX_BIN and KIRZNER_TEST_SEMON_BIN"]
async fn synthetic_capture_is_durable_and_ordinary_reads_exclude_raw() {
    let binary = std::env::var("KIRZNER_TEST_SEMON_CODEX_BIN").expect("Set Semon Codex binary");
    let semon = std::env::var("KIRZNER_TEST_SEMON_BIN").expect("Set Semon ordinary-read binary");
    let directory =
        std::env::temp_dir().join(format!("kirzner-semon-synthetic-{}", std::process::id()));
    let sessions = directory.join("native/sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let native_id = "00000000-0000-4000-8000-000000000099";
    let records = [
        json!({"type":"session_meta","timestamp":"2026-10-09T00:00:00Z","payload":{"id":native_id,"cwd":"/synthetic/kirzner","raw_only_secret":"SYNTHETIC_FORENSIC_MARKER"}}),
        json!({"type":"event_msg","timestamp":"2026-10-09T00:00:01Z","payload":{"type":"user_message","message":"Synthetic adapter test: prepare a draft only."}}),
        json!({"type":"event_msg","timestamp":"2026-10-09T00:00:02Z","payload":{"type":"token_count","raw_only_secret":"SYNTHETIC_FORENSIC_MARKER"}}),
    ];
    std::fs::write(
        sessions.join(format!("rollout-2026-10-09T00-00-00-{native_id}.jsonl")),
        records.iter().map(|r| format!("{r}\n")).collect::<String>(),
    )
    .unwrap();
    let connection = SemonConnection {
        codex_binary: PathBuf::from(binary),
        store: directory.join("semon/traces.sqlite3"),
        cursor: directory.join("semon/cursor.json"),
    };
    assert!(
        connection
            .capture_codex(&sessions, &directory.join("absent-history.jsonl"))
            .unwrap()
    );
    assert!(
        connection
            .capture_codex(&sessions, &directory.join("absent-history.jsonl"))
            .unwrap()
    );
    // Test-only inspection verifies durable custody. Product code never reads
    // Semon tables or reconstructs/parses native harness logs.
    let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", connection.store.display()))
        .await
        .unwrap();
    let raw_count: i64 = sqlx::query_scalar("SELECT count(*) FROM raw_carrier_records")
        .fetch_one(&pool)
        .await
        .unwrap();
    let occurrence_count: i64 = sqlx::query_scalar("SELECT count(*) FROM occurrences")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        raw_count, 3,
        "Repeated capture must not duplicate complete raw records"
    );
    assert_eq!(occurrence_count, 1);
    let session: String = sqlx::query_scalar("SELECT session FROM occurrences LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        session, native_id,
        "Capture retains native identity, not a semantic trace ID"
    );
    let read = || {
        let output = Command::new(&semon)
            .arg("log")
            .arg("--store")
            .arg(&connection.store)
            .arg("--limit")
            .arg("100")
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let before = read();
    assert!(before.contains("Synthetic adapter test"));
    assert!(!before.contains("SYNTHETIC_FORENSIC_MARKER"));
    std::fs::remove_dir_all(directory.join("native")).unwrap();
    assert_eq!(
        read(),
        before,
        "Semon ordinary reads survive loss of the native source"
    );
    // Remove forensic rows only inside this disposable fixture, to establish
    // the read boundary. Operators use Semon's explicit forensic command.
    sqlx::query("DELETE FROM raw_record_traces")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM raw_carrier_records")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        read(),
        before,
        "Ordinary reads are independent of forensic bytes"
    );
    pool.close().await;
    std::fs::remove_dir_all(directory).unwrap();
}
