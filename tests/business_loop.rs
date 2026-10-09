use kirzner::{marketing, model::*, storage::BusinessStore};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static TEST_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn test_directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kirzner-{name}-{}-{}",
        std::process::id(),
        TEST_SEQUENCE.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn command(value: Value) -> Command {
    serde_json::from_value(value).unwrap()
}
fn brief() -> Value {
    json!({"product":"Synthetic test product", "customer":"Solo founders", "capabilities":"A working demo", "objective":"Validate acquisition", "time_budget":"2 hours", "money_budget":"0", "channels":"Existing owned channel", "constraints":"No external writes by agent", "materials":[]})
}
fn proposal(action: &str) -> Value {
    json!({"audience":"Solo founders", "action":action, "expected_deliverables":"A draft and source list", "observation_window":"7 days", "success_criteria":"Two qualified replies", "failure_criteria":"Zero replies after seven days", "inconclusive_criteria":"Window incomplete or unavailable evidence", "resource_limits":"2 hours; 0 spend", "authorization_scope":"Prepare only; no outreach or publication"})
}
fn research_return(attempt: &str, id: &str) -> Value {
    json!({"attempt_id":attempt, "return_id":id, "status":"succeeded", "external_effects":"none", "summary":"Synthetic research fixture", "deliverables":[], "opportunities":[{"title":"Synthetic opportunity", "customer_relevance":"Customer uses manual handoffs", "why_now":"A demo can be tested", "counterevidence":"No demand evidence", "unknowns":"Willingness to pay", "validation_action":"Prepare a small draft", "sources":[{"url":"https://github.com/onsager-ai/kirzner", "title":"Real product source; synthetic interpretation", "evidence":"Local-first business loop direction"}]}], "no_opportunity_reason":null, "execution_refs":[]})
}
fn preparation_return(attempt: &str, id: &str, status: &str, effects: &str) -> Value {
    json!({"attempt_id":attempt, "return_id":id, "status":status, "external_effects":effects, "summary":"Synthetic fixture; no outreach performed", "deliverables":[{"title":"Synthetic draft", "reference":"https://example.com/draft", "media_type":"text/markdown", "note":"Synthetic test only"}], "opportunities":[], "no_opportunity_reason":null, "execution_refs":[]})
}
async fn apply(
    store: &BusinessStore,
    workspace: &mut Workspace,
    value: Value,
) -> marketing::Applied {
    let old = workspace.revision;
    let result = marketing::apply(workspace, command(value)).unwrap();
    if result.changed {
        workspace.revision += 1;
        store.commit_workspace(old, workspace).await.unwrap();
    }
    result
}
async fn run_loop(url: &str, id: &str) {
    let store = BusinessStore::connect(url).await.unwrap();
    let mut w = store.load_workspace(id).await.unwrap();
    apply(
        &store,
        &mut w,
        json!({"type":"save_brief", "brief":brief()}),
    )
    .await;
    apply(&store, &mut w, json!({"type":"create_research", "context":"Only selected product sources", "objective":"Find a cheap evidenced test", "authorization_scope":"Read public sources only"})).await;
    assert!(
        w.experiments.is_empty(),
        "Research needs no experiment proposal"
    );
    let input = research_return("attempt-1", "research-1");
    apply(
        &store,
        &mut w,
        json!({"type":"import_return", "payload":input}),
    )
    .await;
    let revision = w.revision;
    assert!(
        !apply(
            &store,
            &mut w,
            json!({"type":"import_return", "payload":input})
        )
        .await
        .changed
    );
    assert_eq!(w.revision, revision);
    assert_eq!(w.opportunities.len(), 1);
    let mut different = input.clone();
    different["summary"] = "Conflicting content".into();
    assert!(
        apply(
            &store,
            &mut w,
            json!({"type":"import_return", "payload":different})
        )
        .await
        .conflict
    );
    assert_eq!(w.attempts[0].returns[0].original, input);
    assert_eq!(w.return_conflicts.len(), 1);
    assert!(
        !apply(
            &store,
            &mut w,
            json!({"type":"import_return", "payload":different})
        )
        .await
        .changed
    );
    apply(&store, &mut w, json!({"type":"decide_opportunity", "opportunity_id":"opportunity-1", "disposition":"choose", "reason":"Cheap and reversible"})).await;
    apply(&store, &mut w, json!({"type":"create_experiment", "opportunity_id":"opportunity-1", "proposal":proposal("Prepare a draft")})).await;
    let prep = json!({"type":"prepare_handoff", "experiment_id":"experiment-1", "version":1, "context":"Selected source only"});
    assert!(
        marketing::apply(&mut w.clone(), command(prep.clone())).is_err(),
        "Unapproved version cannot export a handoff"
    );
    apply(&store, &mut w, json!({"type":"decide_proposal", "experiment_id":"experiment-1", "version":1, "approved":true, "reason":"Scope is bounded"})).await;
    apply(&store, &mut w, json!({"type":"decide_proposal", "experiment_id":"experiment-1", "version":1, "approved":false, "reason":"Synthetic approval revocation"})).await;
    assert!(
        marketing::apply(&mut w.clone(), command(prep.clone())).is_err(),
        "A later decline revokes approval for this version"
    );
    apply(&store, &mut w, json!({"type":"decide_proposal", "experiment_id":"experiment-1", "version":1, "approved":true, "reason":"Renew bounded preparation approval"})).await;
    apply(&store, &mut w, prep.clone()).await;
    let frozen = w.attempts[1].snapshot.clone();
    for session in ["native-one", "native-two"] {
        apply(&store, &mut w, json!({"type":"attach_execution", "attempt_id":"attempt-2", "execution_ref":{"namespace":"fixture-instance", "harness":"codex", "session_id":session, "machine":"machine-a", "source_revision":"synthetic-reference-manifest", "segment":{"start_offset":0,"end_offset":42}, "continuation":session}})).await;
    }
    assert_eq!(w.attempts[1].execution_refs.len(), 2);
    let original_text = serde_json::to_string_pretty(&preparation_return(
        "attempt-2",
        "partial-1",
        "partial",
        "unknown",
    ))
    .unwrap();
    apply(&store, &mut w, json!({"type":"import_return", "payload":serde_json::from_str::<Value>(&original_text).unwrap(), "original_text":original_text})).await;
    apply(&store, &mut w, json!({"type":"review_return", "attempt_id":"attempt-2", "return_id":"partial-1", "accepted":false, "note":"Needs source corrections"})).await;
    apply(&store, &mut w, json!({"type":"review_return", "attempt_id":"attempt-2", "return_id":"partial-1", "accepted":true, "note":"Accept only the returned draft"})).await;
    assert!(w.experiments[0].actions.is_empty());
    assert!(
        w.experiments[0].observations.is_empty(),
        "Accepting materials is not business success"
    );
    assert!(
        marketing::apply(&mut w.clone(), command(prep.clone())).is_err(),
        "Unknown effects block another attempt"
    );
    assert!(marketing::apply(&mut w.clone(), command(json!({"type":"record_action", "experiment_id":"experiment-1", "version":1, "reference":"Synthetic action", "note":"Synthetic fixture", "self_prepared":false}))).is_err());
    apply(&store, &mut w, json!({"type":"reconcile_effects", "attempt_id":"attempt-2", "return_id":"partial-1", "external_effects":"none", "evidence":"Synthetic operator check: no external write"})).await;
    apply(&store, &mut w, json!({"type":"record_action", "experiment_id":"experiment-1", "version":1, "reference":"Synthetic action only", "note":"Synthetic local validation", "self_prepared":false})).await;
    apply(&store, &mut w, json!({"type":"revise_proposal", "experiment_id":"experiment-1", "proposal":proposal("Changed action")})).await;
    assert!(
        marketing::apply(&mut w.clone(), command(prep)).is_err(),
        "Older proposal approval cannot authorize changed proposal"
    );
    assert!(marketing::apply(&mut w.clone(), command(json!({"type":"prepare_handoff", "experiment_id":"experiment-1", "version":2, "context":""}))).is_err());
    assert!(marketing::apply(&mut w.clone(), command(json!({"type":"decide_proposal", "experiment_id":"experiment-1", "version":1, "approved":true, "reason":"Old approval"}))).is_err());
    apply(&store, &mut w, json!({"type":"import_return", "payload":preparation_return("attempt-2", "late-failed", "failed", "none")})).await;
    assert_eq!(w.attempts[1].proposal_version, Some(1));
    assert_eq!(w.attempts[1].snapshot, frozen);
    assert_eq!(w.attempts[1].returns[0].original_text, original_text);
    assert_eq!(
        w.attempts.len(),
        2,
        "Import never schedules or retries an attempt"
    );
    apply(&store, &mut w, json!({"type":"record_observation", "experiment_id":"experiment-1", "action_id":"action-1", "outcome":"inconclusive", "evidence":"Synthetic test: window incomplete", "comparison":"Cannot assess original v1 criteria", "learning":"Wait for the original observation window"})).await;
    assert_eq!(
        w.experiments[0].observations[0].version, 1,
        "Observation compares to the action's original proposal"
    );
    assert!(w.adopted_learning.is_empty());
    apply(&store, &mut w, json!({"type":"decide_next", "experiment_id":"experiment-1", "observation_id":"observation-1", "choice":"adjust", "reason":"Improve evidence collection"})).await;
    apply(&store, &mut w, json!({"type":"adopt_learning", "experiment_id":"experiment-1", "observation_id":"observation-1"})).await;
    assert!(!apply(&store, &mut w, json!({"type":"adopt_learning", "experiment_id":"experiment-1", "observation_id":"observation-1"})).await.changed);
    // A self-prepared experiment bypasses agent stages but still needs approval.
    apply(&store, &mut w, json!({"type":"create_experiment", "opportunity_id":null, "proposal":proposal("Self-prepared test")})).await;
    apply(&store, &mut w, json!({"type":"decide_proposal", "experiment_id":"experiment-2", "version":1, "approved":true, "reason":"Founder handles preparation"})).await;
    apply(&store, &mut w, json!({"type":"record_action", "experiment_id":"experiment-2", "version":1, "reference":"Synthetic self-prepared action", "note":"Synthetic test only", "self_prepared":true})).await;
    // Zero opportunities and a failed/unknown execution are durable valid results.
    apply(&store, &mut w, json!({"type":"create_research", "context":"", "objective":"Check for stronger evidence", "authorization_scope":"Read only"})).await;
    let mut empty = research_return("attempt-3", "no-opportunity");
    empty["opportunities"] = json!([]);
    empty["no_opportunity_reason"] = "No sufficient evidence".into();
    apply(
        &store,
        &mut w,
        json!({"type":"import_return", "payload":empty}),
    )
    .await;
    let mut unknown = preparation_return("attempt-2", "unknown-1", "unknown", "unknown");
    unknown["deliverables"] = json!([]);
    apply(
        &store,
        &mut w,
        json!({"type":"import_return", "payload":unknown}),
    )
    .await;
    let mut race = w.clone();
    race.revision += 1;
    store.commit_workspace(w.revision, &race).await.unwrap();
    assert!(
        store.commit_workspace(w.revision, &race).await.is_err(),
        "Stale writers cannot overwrite committed data"
    );
    w = race;
    store.close().await;
    let reopened = BusinessStore::connect(url).await.unwrap();
    assert_eq!(
        reopened.load_workspace(id).await.unwrap(),
        w,
        "Restart and repeated migration preserve all records"
    );
    reopened.close().await;
}

#[tokio::test]
async fn sqlite_complete_loop_and_restart() {
    let directory = test_directory("sqlite");
    run_loop(
        &format!("sqlite://{}", directory.join("business.sqlite3").display()),
        "sqlite-parity",
    )
    .await;
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
#[ignore = "requires KIRZNER_TEST_POSTGRES_URL pointing to a disposable PostgreSQL database"]
async fn postgres_complete_loop_and_restart() {
    let url = std::env::var("KIRZNER_TEST_POSTGRES_URL")
        .expect("Set KIRZNER_TEST_POSTGRES_URL to a disposable database");
    let id = format!("parity-{}-{}", std::process::id(), now());
    run_loop(&url, &id).await;
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query("DELETE FROM business_workspaces WHERE workspace_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[test]
fn identity_and_return_validation() {
    let reference = ExecutionRef {
        namespace: "source".into(),
        harness: "codex".into(),
        session_id: "explicit-native-id".into(),
        machine: None,
        source_revision: "revision".into(),
        segment: Some(Segment {
            start_offset: 10,
            end_offset: 10,
        }),
        continuation: None,
    };
    assert!(marketing::validate_execution_ref(&reference).is_err());
    assert!(serde_json::from_value::<Command>(json!({"type":"save_brief", "brief":{}})).is_err());
}
