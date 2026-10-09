use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// IDs are local to a workspace and allocated under revision CAS. No identity
// is guessed from agent text, paths, timestamps, or semantic trace IDs.
pub fn next_id(kind: &str, count: usize) -> String {
    format!("{kind}-{}", count + 1)
}

#[derive(Debug, thiserror::Error)]
pub enum BusinessError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    NotFound(String),
}
pub type Result<T> = std::result::Result<T, BusinessError>;

pub fn required(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 100_000 {
        return Err(BusinessError::Invalid(format!(
            "{field} must contain 1–100000 bytes"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Brief {
    pub product: String,
    #[serde(default)]
    pub customer: String,
    #[serde(default)]
    pub capabilities: String,
    pub objective: String,
    #[serde(default)]
    pub time_budget: String,
    #[serde(default)]
    pub money_budget: String,
    #[serde(default)]
    pub channels: String,
    pub constraints: String,
    #[serde(default)]
    pub materials: Vec<Source>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub url: String,
    pub title: String,
    pub evidence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OpportunityInput {
    pub title: String,
    pub customer_relevance: String,
    pub why_now: String,
    pub counterevidence: String,
    pub unknowns: String,
    pub validation_action: String,
    pub sources: Vec<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_draft: Option<ProposalInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Opportunity {
    pub id: String,
    pub attempt_id: String,
    pub return_id: String,
    pub evidence: OpportunityInput,
    pub disposition: String,
    pub decisions: Vec<OpportunityDecision>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OpportunityDecision {
    pub disposition: String,
    pub reason: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProposalInput {
    pub audience: String,
    pub action: String,
    pub expected_deliverables: String,
    pub observation_window: String,
    pub success_criteria: String,
    pub failure_criteria: String,
    pub inconclusive_criteria: String,
    pub resource_limits: String,
    pub authorization_scope: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Proposal {
    pub version: u32,
    pub content: ProposalInput,
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Decision {
    pub version: u32,
    pub approved: bool,
    pub reason: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Experiment {
    pub id: String,
    pub opportunity_id: Option<String>,
    pub proposals: Vec<Proposal>,
    pub decisions: Vec<Decision>,
    pub actions: Vec<Action>,
    pub observations: Vec<Observation>,
    pub next_steps: Vec<NextStep>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Action {
    pub id: String,
    pub version: u32,
    pub reference: String,
    pub note: String,
    pub self_prepared: bool,
    #[serde(default)]
    pub historical: bool,
    #[serde(default)]
    pub completed_at: Option<u64>,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Observation {
    pub id: String,
    pub action_id: String,
    pub version: u32,
    pub outcome: String,
    pub evidence: String,
    pub comparison: String,
    pub learning: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NextStep {
    pub observation_id: String,
    pub choice: String,
    pub reason: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AdoptedLearning {
    pub experiment_id: String,
    pub observation_id: String,
    pub text: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Attempt {
    pub id: String,
    pub kind: String,
    pub experiment_id: Option<String>,
    pub proposal_version: Option<u32>,
    pub snapshot: Value,
    pub brief_markdown: String,
    pub execution_refs: Vec<ExecutionRef>,
    pub returns: Vec<Receipt>,
    pub reviews: Vec<Review>,
    pub reconciliations: Vec<Reconciliation>,
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRef {
    pub namespace: String,
    pub harness: String,
    pub session_id: String,
    pub machine: Option<String>,
    pub source_revision: String,
    // Semon transcript paging uses native byte offsets. This is deliberately
    // distinct from the trace store's occurrence sequence and semantic trace ID.
    pub segment: Option<Segment>,
    pub continuation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub start_offset: u64,
    pub end_offset: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Deliverable {
    pub title: String,
    pub reference: String,
    pub media_type: String,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReturnInput {
    pub attempt_id: String,
    pub return_id: String,
    pub status: String,
    pub external_effects: String,
    pub summary: String,
    pub deliverables: Vec<Deliverable>,
    pub opportunities: Vec<OpportunityInput>,
    pub no_opportunity_reason: Option<String>,
    pub execution_refs: Vec<ExecutionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief_suggestion: Option<Brief>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub input: ReturnInput,
    pub digest: String,
    pub original: Value,
    pub original_text: String,
    pub imported_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReturnConflict {
    pub attempt_id: String,
    pub return_id: String,
    pub existing_digest: String,
    pub incoming_digest: String,
    pub original: Value,
    pub original_text: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Review {
    pub return_id: String,
    pub accepted: bool,
    pub note: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Reconciliation {
    pub return_id: String,
    pub external_effects: String,
    pub evidence: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub id: String,
    pub revision: i64,
    pub brief: Option<Brief>,
    pub brief_history: Vec<Brief>,
    pub opportunities: Vec<Opportunity>,
    pub experiments: Vec<Experiment>,
    pub attempts: Vec<Attempt>,
    pub return_conflicts: Vec<ReturnConflict>,
    pub adopted_learning: Vec<AdoptedLearning>,
}

impl Workspace {
    pub fn empty(id: &str) -> Self {
        Self {
            id: id.into(),
            revision: 0,
            brief: None,
            brief_history: vec![],
            opportunities: vec![],
            experiments: vec![],
            attempts: vec![],
            return_conflicts: vec![],
            adopted_learning: vec![],
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub expected_revision: i64,
    pub command: Command,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    SaveBrief {
        brief: Brief,
    },
    CreateResearch {
        context: String,
        objective: String,
        authorization_scope: String,
    },
    DecideOpportunity {
        opportunity_id: String,
        disposition: String,
        reason: String,
    },
    CreateExperiment {
        opportunity_id: Option<String>,
        proposal: ProposalInput,
    },
    CreateDraftExperiment {
        opportunity_id: String,
        proposal: Option<ProposalInput>,
    },
    ReviseProposal {
        experiment_id: String,
        proposal: ProposalInput,
    },
    DecideProposal {
        experiment_id: String,
        version: u32,
        approved: bool,
        reason: String,
    },
    PrepareHandoff {
        experiment_id: String,
        version: u32,
        context: String,
    },
    AttachExecution {
        attempt_id: String,
        execution_ref: ExecutionRef,
    },
    ImportReturn {
        payload: Value,
        original_text: Option<String>,
    },
    ImportTaskReturn {
        attempt_id: String,
        payload: Value,
        original_text: Option<String>,
    },
    ReviewReturn {
        attempt_id: String,
        return_id: String,
        accepted: bool,
        note: String,
    },
    ReconcileEffects {
        attempt_id: String,
        return_id: String,
        external_effects: String,
        evidence: String,
    },
    RecordAction {
        experiment_id: String,
        version: u32,
        reference: String,
        note: String,
        self_prepared: bool,
    },
    RecordHistoricalAction {
        experiment_id: String,
        version: u32,
        reference: String,
        note: String,
        self_prepared: bool,
        completed_at: u64,
    },
    RecordObservation {
        experiment_id: String,
        action_id: String,
        outcome: String,
        evidence: String,
        comparison: String,
        learning: String,
    },
    DecideNext {
        experiment_id: String,
        observation_id: String,
        choice: String,
        reason: String,
    },
    AdoptLearning {
        experiment_id: String,
        observation_id: String,
    },
}
