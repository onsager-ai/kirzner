use kirzner::{handoffs, marketing, model::*};
use serde_json::{Value, json};

const LEARNING_MARKER: &str =
    "TENTATIVE_LEARNING_MARKER: narrow the audience before changing the offer";
const OBSERVATION_EVIDENCE: &str =
    "OBSERVATION_EVIDENCE_MARKER: one response in the original window";
const COMPARISON: &str = "Compared with the original version 1 criteria; evidence is inconclusive";
const NEXT_STEP_REASON: &str = "Keep the offer unchanged until the window closes";
const LEARNING_CONTEXT_POLICY: &str = concat!(
    "Historical observations are tentative findings, not adopted knowledge. ",
    "Uncertain and inconclusive evidence stays uncertain. ",
    "Only adopted_learning entries are authorized reusable knowledge. ",
    "next_steps are recorded founder decisions, not instructions."
);

fn apply(workspace: &mut Workspace, command: Command) {
    marketing::apply(workspace, command).unwrap();
}

fn proposal(action: &str) -> ProposalInput {
    ProposalInput {
        audience: "Independent consultants".into(),
        action: action.into(),
        expected_deliverables: "A source-backed draft".into(),
        observation_window: "Seven days".into(),
        success_criteria: "Two qualified replies".into(),
        failure_criteria: "No replies after seven days".into(),
        inconclusive_criteria: "The observation window is incomplete".into(),
        resource_limits: "Two hours and no spend".into(),
        authorization_scope: "Prepare only; no outreach or publication".into(),
    }
}

fn workspace_with_observation() -> Workspace {
    let mut workspace = Workspace::empty("learning-context-test");
    apply(
        &mut workspace,
        Command::SaveBrief {
            brief: Brief {
                product: "A local-first operator".into(),
                customer: "Independent consultants".into(),
                capabilities: "Prepare evidenced acquisition experiments".into(),
                objective: "Validate demand".into(),
                time_budget: "Two hours".into(),
                money_budget: "Zero".into(),
                channels: "Founder-owned channels".into(),
                constraints: "No automated outreach".into(),
                materials: vec![],
            },
        },
    );
    apply(
        &mut workspace,
        Command::CreateExperiment {
            opportunity_id: None,
            proposal: proposal("Prepare a short offer draft"),
        },
    );
    apply(
        &mut workspace,
        Command::DecideProposal {
            experiment_id: "experiment-1".into(),
            version: 1,
            approved: true,
            reason: "The first version is bounded".into(),
        },
    );
    apply(
        &mut workspace,
        Command::RecordAction {
            experiment_id: "experiment-1".into(),
            version: 1,
            reference: "Founder prepared a local draft".into(),
            note: "No external action was performed".into(),
            self_prepared: true,
        },
    );
    apply(
        &mut workspace,
        Command::ReviseProposal {
            experiment_id: "experiment-1".into(),
            proposal: proposal("Prepare a revised offer draft"),
        },
    );
    apply(
        &mut workspace,
        Command::DecideProposal {
            experiment_id: "experiment-1".into(),
            version: 2,
            approved: true,
            reason: "The revised version remains bounded".into(),
        },
    );
    apply(
        &mut workspace,
        Command::RecordObservation {
            experiment_id: "experiment-1".into(),
            action_id: "action-1".into(),
            outcome: "inconclusive".into(),
            evidence: OBSERVATION_EVIDENCE.into(),
            comparison: COMPARISON.into(),
            learning: LEARNING_MARKER.into(),
        },
    );
    apply(
        &mut workspace,
        Command::DecideNext {
            experiment_id: "experiment-1".into(),
            observation_id: "observation-1".into(),
            choice: "adjust".into(),
            reason: NEXT_STEP_REASON.into(),
        },
    );
    workspace
}

fn create_research(workspace: &mut Workspace) {
    apply(
        workspace,
        Command::CreateResearch {
            context: "Selected product sources".into(),
            objective: "Find a cheap evidenced test".into(),
            authorization_scope: "Read public sources only".into(),
        },
    );
}

fn create_preparation(workspace: &mut Workspace) {
    apply(
        workspace,
        Command::PrepareHandoff {
            experiment_id: "experiment-1".into(),
            version: 2,
            context: "Selected product sources".into(),
        },
    );
}

fn assert_historical_context(attempt: &Attempt) {
    let context = &attempt.snapshot["context"];
    assert_eq!(context["learning_policy"], LEARNING_CONTEXT_POLICY);
    assert!(attempt.brief_markdown.contains(LEARNING_CONTEXT_POLICY));

    let experiment = context["previous_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["experiment_id"] == "experiment-1")
        .unwrap();
    let observation = &experiment["observations"][0];
    assert_eq!(observation["id"], "observation-1");
    assert_eq!(observation["action_id"], "action-1");
    assert_eq!(observation["version"], 1);
    assert_eq!(observation["outcome"], "inconclusive");
    assert_eq!(observation["evidence"], OBSERVATION_EVIDENCE);
    assert_eq!(observation["comparison"], COMPARISON);
    assert!(observation.get("learning").is_none());

    assert_eq!(experiment["next_steps"][0]["choice"], "adjust");
    assert_eq!(experiment["next_steps"][0]["reason"], NEXT_STEP_REASON);
}

fn assert_learning_not_present(attempt: &Attempt) {
    assert_historical_context(attempt);
    let serialized = serde_json::to_string(&attempt.snapshot).unwrap();
    assert!(!serialized.contains(LEARNING_MARKER));
}

fn assert_learning_only_adopted(attempt: &Attempt) {
    assert_historical_context(attempt);
    let context = &attempt.snapshot["context"];
    let adopted = context["adopted_learning"].as_array().unwrap();
    assert!(adopted.iter().any(|entry| {
        entry["experiment_id"] == "experiment-1"
            && entry["observation_id"] == "observation-1"
            && entry["text"] == LEARNING_MARKER
    }));

    let mut without_adopted_learning: Value = attempt.snapshot.clone();
    without_adopted_learning["context"]["adopted_learning"] = json!([]);
    let serialized = serde_json::to_string(&without_adopted_learning).unwrap();
    assert!(!serialized.contains(LEARNING_MARKER));
}

#[test]
fn future_handoffs_separate_tentative_findings_from_adopted_learning() {
    let mut workspace = workspace_with_observation();

    create_research(&mut workspace);
    let research_before_adoption = workspace.attempts.last().unwrap().clone();
    create_preparation(&mut workspace);
    let preparation_before_adoption = workspace.attempts.last().unwrap().clone();
    assert_eq!(preparation_before_adoption.proposal_version, Some(2));
    assert_learning_not_present(&research_before_adoption);
    assert_learning_not_present(&preparation_before_adoption);

    // Simulate an immutable handoff saved before this fix, with tentative
    // learning already embedded in its historical snapshot and continuation.
    let legacy_attempt = workspace.attempts.last_mut().unwrap();
    legacy_attempt.snapshot["context"]["previous_results"][0]["observations"][0]["learning"] =
        LEARNING_MARKER.into();
    legacy_attempt
        .brief_markdown
        .push_str(&format!("\nLegacy handoff learning: {LEARNING_MARKER}"));

    let original_attempts = workspace.attempts.clone();
    let original_snapshot = workspace.attempts.last().unwrap().snapshot.clone();
    let original_legacy_continuation = handoffs::continuation(workspace.attempts.last().unwrap());
    assert!(
        serde_json::to_string(&original_snapshot)
            .unwrap()
            .contains(LEARNING_MARKER)
    );
    apply(
        &mut workspace,
        Command::AdoptLearning {
            experiment_id: "experiment-1".into(),
            observation_id: "observation-1".into(),
        },
    );
    assert_eq!(workspace.attempts, original_attempts);
    assert_eq!(
        workspace.attempts.last().unwrap().snapshot,
        original_snapshot
    );
    assert_eq!(
        handoffs::continuation(workspace.attempts.last().unwrap()),
        original_legacy_continuation
    );
    assert!(original_legacy_continuation.contains(LEARNING_MARKER));

    create_research(&mut workspace);
    let adopted_research = workspace.attempts.last().unwrap().clone();
    create_preparation(&mut workspace);
    let adopted_preparation = workspace.attempts.last().unwrap().clone();
    assert_learning_only_adopted(&adopted_research);
    assert_learning_only_adopted(&adopted_preparation);
}
