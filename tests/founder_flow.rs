use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use kirzner::{AppState, BusinessStore, marketing, model::*, router};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn command(value: Value) -> Command {
    serde_json::from_value(value).unwrap()
}
fn minimal_brief() -> Value {
    json!({"product":"Synthetic founder fixture", "objective":"Review a cheap experiment", "constraints":"Prepare only; no spend or outreach"})
}
fn proposal(action: &str) -> Value {
    json!({"audience":"Hypothesized solo founders", "action":action, "expected_deliverables":"One source-grounded draft", "observation_window":"Seven days", "success_criteria":"Two qualified replies", "failure_criteria":"No replies after the full window", "inconclusive_criteria":"Incomplete window or missing evidence", "resource_limits":"One hour; zero spend", "authorization_scope":"Preparation only; founder performs any real action"})
}
fn research_return() -> Value {
    json!({"status":"succeeded", "external_effects":"none", "summary":"Synthetic research; demand unknown", "deliverables":[], "opportunities":[{"title":"Synthetic bounded opportunity", "customer_relevance":"Hypothesis only", "why_now":"Documented technical capability exists", "counterevidence":"No real demand evidence", "unknowns":"Audience, urgency and willingness to pay", "validation_action":"Review a prepared draft", "sources":[{"url":"https://github.com/onsager-ai/semon", "title":"Technical source", "evidence":"No customer evidence implied"}], "experiment_draft":proposal("Original agent draft")}], "execution_refs":[], "brief_suggestion":minimal_brief()})
}
fn materials(effects: &str) -> Value {
    json!({"status":"partial", "external_effects":effects, "summary":"Synthetic materials; no real outreach", "deliverables":[{"title":"Synthetic draft", "reference":"https://example.com/draft", "media_type":"text/markdown", "note":"Synthetic fixture"}], "opportunities":[], "execution_refs":[]})
}
fn historical(version: u32, completed_at: u64, self_prepared: bool) -> Value {
    json!({"type":"record_historical_action", "experiment_id":"experiment-1", "version":version, "reference":"Founder-attested synthetic historical action", "note":"Synthetic evidence supplied by founder, not authenticated execution", "self_prepared":self_prepared, "completed_at":completed_at})
}
fn reject_unchanged(w: &Workspace, value: Value) {
    let mut candidate = w.clone();
    assert!(marketing::apply(&mut candidate, command(value)).is_err());
    assert_eq!(
        &candidate, w,
        "Rejected input must not mutate business records"
    );
}
fn reject_unknown_effects(w: &Workspace, value: Value) {
    let mut candidate = w.clone();
    let error = marketing::apply(&mut candidate, command(value)).unwrap_err();
    assert!(
        error.to_string().contains("Reconcile unknown"),
        "Unexpected blocker: {error}"
    );
    assert_eq!(&candidate, w);
}
async fn apply(store: &BusinessStore, w: &mut Workspace, value: Value) -> marketing::Applied {
    let revision = w.revision;
    let result = marketing::apply(w, command(value)).unwrap();
    if result.changed {
        w.revision += 1;
        store.commit_workspace(revision, w).await.unwrap();
    }
    result
}
async fn return_input_replays(url: &str, id: &str) {
    let store = BusinessStore::connect(url).await.unwrap();
    let mut w = store.load_workspace(id).await.unwrap();
    apply(&store, &mut w, json!({"type":"create_research", "context":"Synthetic compatibility fixture", "objective":"Review return replay", "authorization_scope":"Read only"})).await;
    let attempt_id = w.attempts.last().unwrap().id.clone();
    for (return_id, null_optionals) in [("minimal-suggestion", false), ("null-optionals", true)] {
        let mut original = research_return();
        original["attempt_id"] = attempt_id.clone().into();
        original["return_id"] = return_id.into();
        original["no_opportunity_reason"] = Value::Null;
        if null_optionals {
            original["brief_suggestion"] = Value::Null;
            original["opportunities"][0]["experiment_draft"] = Value::Null;
        }
        let original_text = format!(
            " \n{}\n  ",
            serde_json::to_string_pretty(&original).unwrap()
        );
        apply(
            &store,
            &mut w,
            json!({"type":"import_return", "payload":original, "original_text":original_text}),
        )
        .await;
        let receipt = w.attempts.last().unwrap().returns.last().unwrap();
        assert_eq!(receipt.original, original);
        assert_eq!(receipt.original_text, original_text);
        assert_eq!(
            receipt.digest,
            format!("{:x}", Sha256::digest(original.to_string().as_bytes()))
        );
    }
    store.close().await;
    let reopened = BusinessStore::connect(url).await.unwrap();
    assert_eq!(reopened.load_workspace(id).await.unwrap(), w);
    let before_replay = w.clone();
    for receipt in &before_replay.attempts.last().unwrap().returns {
        let api_input = serde_json::to_value(&receipt.input).unwrap();
        assert_ne!(
            api_input, receipt.original,
            "Fixture must change the raw payload digest"
        );
        if receipt.input.return_id == "minimal-suggestion" {
            assert_eq!(api_input["brief_suggestion"]["customer"], "");
            assert_eq!(api_input["brief_suggestion"]["materials"], json!([]));
        } else {
            assert!(api_input.get("brief_suggestion").is_none());
            assert!(
                api_input["opportunities"][0]
                    .get("experiment_draft")
                    .is_none()
            );
        }
        for import_type in ["import_return", "import_task_return"] {
            let mut replay = json!({"type":import_type, "payload":api_input});
            if import_type == "import_task_return" {
                replay["attempt_id"] = attempt_id.clone().into();
            }
            let result = apply(&reopened, &mut w, replay).await;
            assert!(!result.changed && !result.conflict);
            assert_eq!(
                w, before_replay,
                "Replay must preserve revision, receipts, conflicts and exact originals"
            );
            assert_eq!(reopened.load_workspace(id).await.unwrap(), before_replay);
        }
    }
    let receipts = before_replay.attempts.last().unwrap().returns.clone();
    let api_input = serde_json::to_value(&receipts[0].input).unwrap();
    let mut changed_summary = api_input.clone();
    changed_summary["summary"] = "Changed meaningful summary".into();
    let mut changed_provenance = api_input.clone();
    changed_provenance["opportunities"][0]["sources"][0]["evidence"] =
        "Changed source evidence".into();
    let mut changed_draft = api_input.clone();
    changed_draft["opportunities"][0]["experiment_draft"]["action"] =
        "Changed proposed action".into();
    let mut changed_suggestion = api_input;
    changed_suggestion["brief_suggestion"]["product"] = "Changed suggested product".into();
    let conflict_count = w.return_conflicts.len();
    for (index, payload) in [
        changed_summary,
        changed_provenance,
        changed_draft,
        changed_suggestion,
    ]
    .into_iter()
    .enumerate()
    {
        let conflict =
            json!({"type":"import_task_return", "attempt_id":attempt_id, "payload":payload});
        let result = apply(&reopened, &mut w, conflict.clone()).await;
        assert!(result.changed && result.conflict);
        assert_eq!(w.return_conflicts.len(), conflict_count + index + 1);
        assert_eq!(w.return_conflicts.last().unwrap().original, payload);
        assert_eq!(w.attempts.last().unwrap().returns, receipts);
        let after_conflict = w.clone();
        let result = apply(&reopened, &mut w, conflict).await;
        assert!(!result.changed && result.conflict);
        assert_eq!(w, after_conflict);
    }
    reopened.close().await;
    let reopened = BusinessStore::connect(url).await.unwrap();
    assert_eq!(reopened.load_workspace(id).await.unwrap(), w);
    reopened.close().await;
}
async fn founder_flow(url: &str, id: &str) {
    let store = BusinessStore::connect(url).await.unwrap();
    let mut w = store.load_workspace(id).await.unwrap();
    apply(
        &store,
        &mut w,
        json!({"type":"save_brief", "brief":minimal_brief()}),
    )
    .await;
    assert_eq!(w.brief.as_ref().unwrap().customer, "");
    assert!(w.brief.as_ref().unwrap().materials.is_empty());
    let mut oversized = minimal_brief();
    oversized["customer"] = "x".repeat(100_001).into();
    reject_unchanged(&w, json!({"type":"save_brief", "brief":oversized}));
    let mut missing_required = minimal_brief();
    missing_required["constraints"] = "".into();
    reject_unchanged(&w, json!({"type":"save_brief", "brief":missing_required}));
    apply(&store, &mut w, json!({"type":"create_research", "context":"Selected public source only", "objective":"Find an evidenced cheap draft", "authorization_scope":"Read only; no contact"})).await;
    assert_eq!(
        w.attempts[0].snapshot["context"]["unknown_brief_fields"],
        json!([
            "customer",
            "capabilities",
            "time_budget",
            "money_budget",
            "channels",
            "materials"
        ])
    );
    assert!(w.attempts[0].brief_markdown.contains("attempt-1-return-1"));
    assert!(w.attempts[0].brief_markdown.contains("experiment_draft"));
    assert!(w.attempts[0].brief_markdown.contains("brief_suggestion"));
    let template_text = w.attempts[0]
        .brief_markdown
        .rsplit("```json\n")
        .next()
        .unwrap()
        .split("\n```")
        .next()
        .unwrap();
    let template: Value = serde_json::from_str(template_text).unwrap();
    assert_eq!(template["status"], "unknown");
    assert_eq!(template["external_effects"], "unknown");
    let source_snapshot = w.attempts[0].snapshot.clone();
    let mut original = research_return();
    original["external_effects"] = "unknown".into();
    let original_text = format!("  {}\n", serde_json::to_string_pretty(&original).unwrap());
    let paste = json!({"type":"import_task_return", "attempt_id":"attempt-1", "payload":original, "original_text":original_text});
    apply(&store, &mut w, paste.clone()).await;
    assert!(
        w.experiments.is_empty(),
        "Import is not founder selection or approval"
    );
    assert_eq!(
        w.brief_history.len(),
        1,
        "Suggested brief remains unadopted"
    );
    assert_eq!(w.attempts.len(), 1, "Import never creates another attempt");
    let receipt = &w.attempts[0].returns[0];
    assert_eq!(receipt.original, original);
    assert_eq!(receipt.original_text, original_text);
    assert_eq!(receipt.input.attempt_id, "attempt-1");
    assert!(receipt.input.brief_suggestion.is_some());
    let mut identity_payload = original.clone();
    identity_payload["attempt_id"] = "attempt-1".into();
    let expected_id = format!(
        "return-{:x}",
        Sha256::digest(identity_payload.to_string().as_bytes())
    );
    assert_eq!(receipt.input.return_id, expected_id);
    assert!(!apply(&store, &mut w, paste).await.changed);
    let mut blank = original.clone();
    blank["return_id"] = "  ".into();
    assert!(
        !apply(
            &store,
            &mut w,
            json!({"type":"import_task_return", "attempt_id":"attempt-1", "payload":blank})
        )
        .await
        .changed
    );
    let mut explicit = original.clone();
    explicit["attempt_id"] = "attempt-1".into();
    explicit["return_id"] = expected_id.clone().into();
    assert!(
        !apply(
            &store,
            &mut w,
            json!({"type":"import_return", "payload":explicit})
        )
        .await
        .changed,
        "Strict legacy import and selected paste share effective identities"
    );
    let mut conflict = explicit.clone();
    conflict["summary"] = "Changed output retaining an explicit identity".into();
    let conflict_text = format!("\n{}  ", conflict);
    let conflict_command = json!({"type":"import_task_return", "attempt_id":"attempt-1", "payload":conflict, "original_text":conflict_text});
    let result = apply(&store, &mut w, conflict_command.clone()).await;
    assert!(result.changed && result.conflict);
    assert!(!apply(&store, &mut w, conflict_command).await.changed);
    assert_eq!(w.return_conflicts[0].original, conflict);
    assert_eq!(w.return_conflicts[0].original_text, conflict_text);
    assert_eq!(w.attempts[0].returns.len(), 1);
    assert_eq!(w.opportunities.len(), 1);
    let mut mismatch = original.clone();
    mismatch["attempt_id"] = "other-task".into();
    reject_unchanged(
        &w,
        json!({"type":"import_task_return", "attempt_id":"attempt-1", "payload":mismatch}),
    );
    reject_unchanged(
        &w,
        json!({"type":"import_task_return", "attempt_id":"attempt-1", "payload":original, "original_text":"{}"}),
    );
    reject_unchanged(
        &w,
        json!({"type":"import_task_return", "attempt_id":"missing-task", "payload":original}),
    );
    reject_unchanged(
        &w,
        json!({"type":"import_task_return", "attempt_id":"attempt-1", "payload":[]}),
    );
    let mut invalid_return = original.clone();
    invalid_return["opportunities"][0]["experiment_draft"]["audience"] = "".into();
    reject_unchanged(
        &w,
        json!({"type":"import_task_return", "attempt_id":"attempt-1", "payload":invalid_return}),
    );
    let mut invalid_proposal = proposal("Invalid edited draft");
    invalid_proposal["authorization_scope"] = "".into();
    reject_unchanged(
        &w,
        json!({"type":"create_draft_experiment", "opportunity_id":"opportunity-1", "proposal":invalid_proposal}),
    );
    assert_eq!(w.opportunities[0].disposition, "unreviewed");
    let edited = proposal("Founder-edited draft");
    apply(&store, &mut w, json!({"type":"create_draft_experiment", "opportunity_id":"opportunity-1", "proposal":edited})).await;
    assert_eq!(w.opportunities[0].disposition, "choose");
    assert_eq!(w.opportunities[0].decisions.len(), 1);
    assert!(w.experiments[0].decisions.is_empty());
    assert_eq!(
        serde_json::to_value(&w.experiments[0].proposals[0].content).unwrap(),
        edited
    );
    assert_eq!(
        serde_json::to_value(
            w.opportunities[0]
                .evidence
                .experiment_draft
                .as_ref()
                .unwrap()
        )
        .unwrap(),
        proposal("Original agent draft")
    );
    assert!(!apply(&store, &mut w, json!({"type":"create_draft_experiment", "opportunity_id":"opportunity-1", "proposal":proposal("Do not overwrite")})).await.changed);
    assert_eq!(w.experiments.len(), 1);
    assert_eq!(w.opportunities[0].decisions.len(), 1);
    let prepare_v1 = json!({"type":"prepare_handoff", "experiment_id":"experiment-1", "version":1, "context":"Selected source"});
    reject_unchanged(&w, prepare_v1.clone());
    apply(&store, &mut w, json!({"type":"decide_proposal", "experiment_id":"experiment-1", "version":1, "approved":true, "reason":"Bounded synthetic approval"})).await;
    reject_unknown_effects(&w, prepare_v1.clone());
    let source_action = json!({"type":"record_action", "experiment_id":"experiment-1", "version":1, "reference":"Synthetic founder action", "note":"Synthetic evidence only", "self_prepared":true});
    reject_unknown_effects(&w, source_action);
    let mut old_version = w.clone();
    marketing::apply(&mut old_version, command(json!({"type":"revise_proposal", "experiment_id":"experiment-1", "proposal":proposal("Historical source blocker fixture")}))).unwrap();
    let historical_completion = old_version.experiments[0].proposals[1].created_at;
    reject_unknown_effects(&old_version, historical(1, historical_completion, true));
    let mut independent = w.clone();
    marketing::apply(&mut independent, command(json!({"type":"create_experiment", "opportunity_id":null, "proposal":proposal("Independent legacy experiment")}))).unwrap();
    marketing::apply(&mut independent, command(json!({"type":"decide_proposal", "experiment_id":"experiment-2", "version":1, "approved":true, "reason":"Synthetic independent scope"}))).unwrap();
    marketing::apply(&mut independent, command(json!({"type":"prepare_handoff", "experiment_id":"experiment-2", "version":1, "context":"Independent source"}))).unwrap();
    marketing::apply(&mut independent, command(json!({"type":"record_action", "experiment_id":"experiment-2", "version":1, "reference":"Independent synthetic action", "note":"No actual outreach", "self_prepared":true}))).unwrap();
    assert_eq!(
        w.attempts.len(),
        1,
        "Blocked operations never retry the source research"
    );
    apply(&store, &mut w, json!({"type":"reconcile_effects", "attempt_id":"attempt-1", "return_id":expected_id, "external_effects":"none", "evidence":"Synthetic explicit source check: no external write"})).await;
    assert_eq!(w.attempts[0].snapshot, source_snapshot);
    assert_eq!(w.attempts[0].returns[0].original, original);
    assert_eq!(w.attempts[0].returns[0].original_text, original_text);
    assert_eq!(
        w.attempts[0].returns[0].input.external_effects, "unknown",
        "Reconciliation appends evidence without rewriting a return"
    );
    apply(&store, &mut w, prepare_v1).await;
    let immutable_snapshot = w.attempts[1].snapshot.clone();
    apply(&store, &mut w, json!({"type":"import_task_return", "attempt_id":"attempt-2", "payload":materials("unknown")})).await;
    let return_id = w.attempts[1].returns[0].input.return_id.clone();
    apply(&store, &mut w, json!({"type":"review_return", "attempt_id":"attempt-2", "return_id":return_id, "accepted":true, "note":"Accept synthetic materials only"})).await;
    assert!(w.experiments[0].actions.is_empty());
    apply(&store, &mut w, json!({"type":"revise_proposal", "experiment_id":"experiment-1", "proposal":proposal("Revised current action")})).await;
    assert!(marketing::authorized(&w.experiments[0], 2).is_err());
    let action_v2 = json!({"type":"record_action", "experiment_id":"experiment-1", "version":2, "reference":"Synthetic action only", "note":"No real business execution", "self_prepared":true});
    reject_unchanged(&w, action_v2.clone());
    apply(&store, &mut w, json!({"type":"decide_proposal", "experiment_id":"experiment-1", "version":2, "approved":true, "reason":"Fresh revised approval"})).await;
    // Explicit synthetic timestamps avoid sleeps and test whole-second boundaries.
    let base = now() - 100;
    let e = &mut w.experiments[0];
    e.proposals[0].created_at = base;
    e.proposals[1].created_at = base + 30;
    e.decisions[0].recorded_at = base + 10;
    e.decisions[1].recorded_at = base + 40;
    e.decisions.push(Decision {
        version: 1,
        approved: false,
        reason: "Synthetic revocation".into(),
        recorded_at: base + 20,
    });
    reject_unchanged(&w, action_v2.clone());
    reject_unchanged(
        &w,
        json!({"type":"prepare_handoff", "experiment_id":"experiment-1", "version":2, "context":""}),
    );
    reject_unchanged(&w, historical(1, base + 15, false));
    apply(&store, &mut w, json!({"type":"reconcile_effects", "attempt_id":"attempt-2", "return_id":return_id, "external_effects":"none", "evidence":"Synthetic operator attestation: no external write"})).await;
    reject_unchanged(&w, historical(2, base + 45, true));
    reject_unchanged(&w, historical(9, base + 15, true));
    reject_unchanged(&w, historical(1, 0, true));
    reject_unchanged(&w, historical(1, now() + 100, true));
    reject_unchanged(&w, historical(1, base + 5, true));
    reject_unchanged(&w, historical(1, base + 31, true));
    reject_unchanged(&w, historical(1, base + 25, true));
    let mut boundary = w.clone();
    boundary.experiments[0].decisions.push(Decision {
        version: 1,
        approved: true,
        reason: "Synthetic reapproval at boundary".into(),
        recorded_at: base + 30,
    });
    marketing::apply(&mut boundary, command(historical(1, base + 30, true))).unwrap();
    assert!(boundary.experiments[0].actions[0].historical);
    let mut no_approval = w.clone();
    no_approval.experiments[0]
        .decisions
        .retain(|d| d.version != 1);
    reject_unchanged(&no_approval, historical(1, base + 15, true));
    let mut no_materials = w.clone();
    no_materials.attempts[1].reviews.clear();
    reject_unchanged(&no_materials, historical(1, base + 15, false));
    apply(&store, &mut w, historical(1, base + 15, false)).await;
    let historical_action = &w.experiments[0].actions[0];
    assert!(historical_action.historical);
    assert_eq!(historical_action.completed_at, Some(base + 15));
    assert_eq!(
        w.experiments[0].decisions.len(),
        3,
        "Historical recording creates no current approval"
    );
    assert_eq!(
        w.attempts.len(),
        2,
        "Historical recording creates no handoff"
    );
    let mut old_materials = action_v2.clone();
    old_materials["self_prepared"] = false.into();
    reject_unchanged(&w, old_materials);
    apply(&store, &mut w, action_v2).await;
    assert!(!w.experiments[0].actions[1].historical);
    assert_eq!(w.experiments[0].actions[1].completed_at, None);
    apply(&store, &mut w, json!({"type":"record_observation", "experiment_id":"experiment-1", "action_id":"action-1", "outcome":"inconclusive", "evidence":"Synthetic incomplete window", "comparison":"Original v1 criteria remain in force", "learning":"Tentative synthetic learning"})).await;
    assert_eq!(w.experiments[0].observations[0].version, 1);
    assert_eq!(
        w.experiments[0].proposals[0].content.success_criteria,
        "Two qualified replies"
    );
    assert_eq!(w.attempts[1].snapshot, immutable_snapshot);
    assert!(w.adopted_learning.is_empty());
    apply(&store, &mut w, json!({"type":"decide_next", "experiment_id":"experiment-1", "observation_id":"observation-1", "choice":"adjust", "reason":"Synthetic evidence is incomplete"})).await;
    store.close().await;
    let reopened = BusinessStore::connect(url).await.unwrap();
    assert_eq!(reopened.load_workspace(id).await.unwrap(), w);
    reopened.close().await;
    return_input_replays(url, id).await;
}

#[tokio::test]
async fn sqlite_founder_flow_and_restart() {
    let dir =
        std::env::temp_dir().join(format!("kirzner-founder-{}-{}", std::process::id(), now()));
    std::fs::create_dir_all(&dir).unwrap();
    founder_flow(
        &format!("sqlite://{}", dir.join("business.sqlite3").display()),
        "founder-sqlite",
    )
    .await;
    std::fs::remove_dir_all(dir).unwrap();
}
#[tokio::test]
#[ignore = "requires KIRZNER_TEST_POSTGRES_URL pointing to a disposable PostgreSQL database"]
async fn postgres_founder_flow_and_restart() {
    let url = std::env::var("KIRZNER_TEST_POSTGRES_URL")
        .expect("Set a disposable PostgreSQL database URL");
    let id = format!("founder-postgres-{}-{}", std::process::id(), now());
    founder_flow(&url, &id).await;
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query("DELETE FROM business_workspaces WHERE workspace_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[test]
fn legacy_shapes_and_default_draft_selection() {
    let mut w = Workspace::empty("legacy-shapes");
    let mut full = minimal_brief();
    for key in [
        "customer",
        "capabilities",
        "time_budget",
        "money_budget",
        "channels",
    ] {
        full[key] = "Known legacy value".into();
    }
    full["materials"] = json!([]);
    marketing::apply(&mut w, command(json!({"type":"save_brief", "brief":full}))).unwrap();
    marketing::apply(&mut w, command(json!({"type":"create_research", "context":"", "objective":"Research", "authorization_scope":"Read only"}))).unwrap();
    let mut input = research_return();
    input["attempt_id"] = "attempt-1".into();
    input["return_id"] = "legacy-return".into();
    input["no_opportunity_reason"] = Value::Null;
    input.as_object_mut().unwrap().remove("brief_suggestion");
    input["opportunities"][0]
        .as_object_mut()
        .unwrap()
        .remove("experiment_draft");
    marketing::apply(
        &mut w,
        command(json!({"type":"import_return", "payload":input})),
    )
    .unwrap();
    let original_digest = w.attempts[0].returns[0].digest.clone();
    let legacy_workspace: Workspace =
        serde_json::from_value(serde_json::to_value(&w).unwrap()).unwrap();
    let api_input = serde_json::to_value(&legacy_workspace.attempts[0].returns[0].input).unwrap();
    assert!(api_input.get("brief_suggestion").is_none());
    assert!(
        api_input["opportunities"][0]
            .get("experiment_draft")
            .is_none()
    );
    assert!(
        !marketing::apply(
            &mut w,
            command(json!({"type":"import_return", "payload":api_input}))
        )
        .unwrap()
        .changed,
        "Reopened legacy API input must remain an identical return"
    );
    assert_eq!(w.attempts[0].returns[0].digest, original_digest);
    assert!(w.attempts[0].returns[0].input.brief_suggestion.is_none());
    assert!(w.opportunities[0].evidence.experiment_draft.is_none());
    reject_unchanged(
        &w,
        json!({"type":"create_draft_experiment", "opportunity_id":"opportunity-1"}),
    );
    w.opportunities[0].evidence.experiment_draft =
        Some(serde_json::from_value(proposal("Default agent draft")).unwrap());
    marketing::apply(
        &mut w,
        command(json!({"type":"create_draft_experiment", "opportunity_id":"opportunity-1"})),
    )
    .unwrap();
    assert_eq!(
        w.experiments[0].proposals[0].content.action,
        "Default agent draft"
    );
    assert!(w.experiments[0].decisions.is_empty());
    let action: Action = serde_json::from_value(json!({"id":"action-legacy", "version":1, "reference":"Legacy founder evidence", "note":"Legacy note", "self_prepared":true, "recorded_at":42})).unwrap();
    assert!(!action.historical);
    assert_eq!(action.completed_at, None);
    w.experiments[0].actions.push(action);
    let mut stored = serde_json::to_value(&w).unwrap();
    stored["experiments"][0]["actions"][0]
        .as_object_mut()
        .unwrap()
        .remove("historical");
    stored["experiments"][0]["actions"][0]
        .as_object_mut()
        .unwrap()
        .remove("completed_at");
    stored["opportunities"][0]["evidence"]
        .as_object_mut()
        .unwrap()
        .remove("experiment_draft");
    stored["attempts"][0]["returns"][0]["input"]
        .as_object_mut()
        .unwrap()
        .remove("brief_suggestion");
    let restored: Workspace = serde_json::from_value(stored).unwrap();
    assert_eq!(
        restored.experiments[0].actions[0],
        w.experiments[0].actions[0]
    );
    assert_eq!(restored.brief, w.brief);
    assert!(serde_json::from_value::<Brief>(json!({"product":"p", "objective":"o"})).is_err());
    assert!(
        serde_json::from_value::<ReturnInput>(research_return()).is_err(),
        "Legacy import still requires executor IDs"
    );
}

#[tokio::test]
async fn malformed_mutation_errors_are_json() {
    let store = BusinessStore::connect("sqlite::memory:").await.unwrap();
    let app = router(
        AppState::new(store.clone(), std::env::temp_dir()),
        std::env::temp_dir(),
    );
    for (input, expected_status) in [
        ("{", 400),
        ("{}", 422),
        (
            "{\"expected_revision\":0,\"command\":{\"type\":\"save_brief\",\"brief\":{}}}",
            422,
        ),
    ] {
        let response = tower::ServiceExt::oneshot(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/workspace")
                .header("Host", "127.0.0.1:4317")
                .header("Content-Type", "application/json")
                .body(Body::from(input))
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status().as_u16(), expected_status);
        let value: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert!(
            value["error"]
                .as_str()
                .is_some_and(|error| !error.is_empty())
        );
    }
    assert_eq!(store.load_workspace("local").await.unwrap().revision, 0);
    store.close().await;
}
