//! Immutable, manual executor handoffs. The executor owns native continuation,
//! execution permissions, credentials, retries, and scheduling.
use crate::model::*;
use serde_json::{Value, json};

fn context(workspace: &Workspace, selected: String) -> Value {
    json!({
        "business_brief": workspace.brief,
        "selected_context": selected,
        "previous_results": workspace.experiments.iter().map(|e| json!({"experiment_id": e.id, "observations": e.observations, "next_steps": e.next_steps})).collect::<Vec<_>>(),
        "adopted_learning": workspace.adopted_learning,
    })
}

fn make(
    workspace: &Workspace,
    kind: &str,
    experiment_id: Option<String>,
    proposal_version: Option<u32>,
    snapshot: Value,
    instructions: &str,
) -> Attempt {
    let id = next_id("attempt", workspace.attempts.len());
    let template = ReturnInput {
        attempt_id: id.clone(),
        return_id: "executor-assigned-stable-return-id".into(),
        status: "succeeded".into(),
        external_effects: "none".into(),
        summary: String::new(),
        deliverables: vec![],
        opportunities: vec![],
        no_opportunity_reason: None,
        execution_refs: vec![],
    };
    let brief_markdown = format!(
        "# Kirzner {kind} handoff: {id}\n\nWorkspace: {}\nProposal version: {}\n\n{instructions}\n\n## Immutable input snapshot\n\n```json\n{}\n```\n\n## Return envelope\n\nUse the stable attempt ID above and assign a stable return ID. Reuse that identity only for the exact same content. New revisions require a new return ID. Report execution status, deliverables and external effects separately. Never infer business success. Include explicit native execution references when available.\n\n```json\n{}\n```\n",
        workspace.id,
        proposal_version.map_or("not applicable".into(), |v| v.to_string()),
        serde_json::to_string_pretty(&snapshot).expect("JSON value"),
        serde_json::to_string_pretty(&template).expect("return serialization"),
    );
    Attempt {
        id,
        kind: kind.into(),
        experiment_id,
        proposal_version,
        snapshot,
        brief_markdown,
        execution_refs: vec![],
        returns: vec![],
        reviews: vec![],
        reconciliations: vec![],
        created_at: now(),
    }
}

pub fn research(
    workspace: &Workspace,
    selected: String,
    objective: String,
    authorization_scope: String,
) -> Attempt {
    let snapshot = json!({"context": context(workspace, selected), "objective": objective, "authorization_scope": authorization_scope,
        "acceptance": "Zero to three evidenced opportunities. Each needs source URL/title/evidence, customer relevance, why now, counterevidence, unknowns, and a cheap validation action. Zero opportunities is valid with an explicit reason."});
    make(
        workspace,
        "research",
        None,
        None,
        snapshot,
        "Research only within the authorization scope. Do not contact prospects or publish. Return zero to three opportunities, or state that no sufficiently actionable opportunity was found. Opportunity fields: title, customer_relevance, why_now, counterevidence, unknowns, validation_action, sources [{url,title,evidence}]. Keep inference separate from verified evidence.",
    )
}

pub fn preparation(workspace: &Workspace, experiment: &Experiment, selected: String) -> Attempt {
    let proposal = experiment
        .proposals
        .last()
        .expect("experiment always has a proposal");
    let snapshot = json!({"context": context(workspace, selected), "proposal": proposal,
        "founder_decision": experiment.decisions.iter().rev().find(|d| d.version == proposal.version),
        "opportunity": workspace.opportunities.iter().find(|o| Some(&o.id) == experiment.opportunity_id.as_ref())});
    make(
        workspace,
        "preparation",
        Some(experiment.id.clone()),
        Some(proposal.version),
        snapshot,
        "Prepare the expected deliverables under this specific approved proposal and scope. This handoff authorizes preparation only; the founder performs outreach or publication outside Kirzner. Return partial work and errors honestly. Unknown external effects require reconciliation, not a blind retry.",
    )
}

pub fn continuation(attempt: &Attempt) -> String {
    format!("# Continuation for {}\n\nUse the attached native continuation references where the executor supports them. Otherwise start with this brief. No external write is automatically authorized or retried. Keep the original attempt and proposal version. Assign a new return ID for changed output.\n\n## Explicit native references\n```json\n{}\n```\n\n## Previous returns and review\n```json\n{}\n```\n\n{}", attempt.id,
        serde_json::to_string_pretty(&attempt.execution_refs).expect("references"),
        serde_json::to_string_pretty(&serde_json::json!({"returns": attempt.returns, "reviews": attempt.reviews, "reconciliations": attempt.reconciliations})).expect("returns"), attempt.brief_markdown)
}
