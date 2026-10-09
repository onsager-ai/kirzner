//! Business rules for the first acquisition loop. This module never runs an
//! executor, publishes material, or guesses the result of an external action.
use crate::{handoffs, model::*};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct Applied {
    pub changed: bool,
    pub conflict: bool,
    pub message: &'static str,
}

fn one_of(value: &str, allowed: &[&str], field: &str) -> Result<()> {
    if !allowed.contains(&value) {
        return Err(BusinessError::Invalid(format!("Invalid {field}: {value}")));
    }
    Ok(())
}

fn validate_sources(sources: &[Source]) -> Result<()> {
    for source in sources {
        required(&source.title, "source title")?;
        required(&source.evidence, "source evidence")?;
        if !(source.url.starts_with("https://") || source.url.starts_with("http://")) {
            return Err(BusinessError::Invalid(
                "Sources require an http(s) URL".into(),
            ));
        }
    }
    Ok(())
}

fn validate_brief(brief: &Brief) -> Result<()> {
    for (field, value) in [
        ("product", &brief.product),
        ("objective", &brief.objective),
        ("constraints", &brief.constraints),
    ] {
        required(value, field)?;
    }
    for (field, value) in [
        ("customer", &brief.customer),
        ("capabilities", &brief.capabilities),
        ("time budget", &brief.time_budget),
        ("money budget", &brief.money_budget),
        ("channels", &brief.channels),
    ] {
        if !value.is_empty() {
            required(value, field)?;
        }
    }
    validate_sources(&brief.materials)
}

fn validate_proposal(proposal: &ProposalInput) -> Result<()> {
    for (field, value) in [
        ("audience", &proposal.audience),
        ("action", &proposal.action),
        ("deliverables", &proposal.expected_deliverables),
        ("window", &proposal.observation_window),
        ("success criteria", &proposal.success_criteria),
        ("failure criteria", &proposal.failure_criteria),
        ("inconclusive criteria", &proposal.inconclusive_criteria),
        ("limits", &proposal.resource_limits),
        ("authorization scope", &proposal.authorization_scope),
    ] {
        required(value, field)?;
    }
    Ok(())
}

pub fn validate_execution_ref(reference: &ExecutionRef) -> Result<()> {
    required(&reference.namespace, "Semon instance/source namespace")?;
    required(&reference.session_id, "native session ID")?;
    required(&reference.source_revision, "reference provenance/revision")?;
    one_of(
        &reference.harness,
        &["codex", "claude", "copilot"],
        "harness",
    )?;
    if let Some(segment) = &reference.segment
        && segment.end_offset <= segment.start_offset
    {
        return Err(BusinessError::Invalid(
            "Segment end must be after its start byte offset".into(),
        ));
    }
    Ok(())
}

fn experiment_mut<'a>(workspace: &'a mut Workspace, id: &str) -> Result<&'a mut Experiment> {
    workspace
        .experiments
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| BusinessError::NotFound("Experiment not found".into()))
}
fn attempt_mut<'a>(workspace: &'a mut Workspace, id: &str) -> Result<&'a mut Attempt> {
    workspace
        .attempts
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or_else(|| BusinessError::NotFound("Handoff attempt not found".into()))
}
pub fn authorized(experiment: &Experiment, version: u32) -> Result<()> {
    if experiment.proposals.last().map(|p| p.version) != Some(version) {
        return Err(BusinessError::Conflict(
            "This proposal changed; review its current version".into(),
        ));
    }
    if !experiment
        .decisions
        .iter()
        .rev()
        .find(|d| d.version == version)
        .is_some_and(|d| d.approved)
    {
        return Err(BusinessError::Conflict(
            "This proposal version has no current founder approval".into(),
        ));
    }
    Ok(())
}

fn unresolved_effects(attempt: &Attempt) -> bool {
    attempt.returns.iter().any(|r| {
        r.input.external_effects == "unknown"
            && !attempt
                .reconciliations
                .iter()
                .any(|c| c.return_id == r.input.return_id)
    })
}

fn validate_return(input: &ReturnInput, attempt: &Attempt) -> Result<()> {
    required(&input.return_id, "return ID")?;
    required(&input.summary, "summary")?;
    one_of(
        &input.status,
        &["succeeded", "partial", "failed", "unknown"],
        "execution status",
    )?;
    one_of(
        &input.external_effects,
        &["none", "confirmed", "unknown"],
        "external effects",
    )?;
    for reference in &input.execution_refs {
        validate_execution_ref(reference)?;
    }
    for deliverable in &input.deliverables {
        required(&deliverable.title, "deliverable title")?;
        required(&deliverable.media_type, "media type")?;
        if !(deliverable.reference.starts_with("https://")
            || deliverable.reference.starts_with("http://")
            || (deliverable.reference.starts_with("files/")
                && deliverable.reference.len() == 70
                && deliverable.reference[6..]
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())))
        {
            return Err(BusinessError::Invalid(
                "Deliverables require an http(s) reference or an uploaded files/<sha256> reference"
                    .into(),
            ));
        }
    }
    if attempt.kind != "research" && !input.opportunities.is_empty() {
        return Err(BusinessError::Invalid(
            "Only research attempts return opportunities".into(),
        ));
    }
    if input.opportunities.len() > 3 {
        return Err(BusinessError::Invalid(
            "Research may return zero to three opportunities".into(),
        ));
    }
    if attempt.kind == "research" && input.opportunities.is_empty() && input.status == "succeeded" {
        required(
            input.no_opportunity_reason.as_deref().unwrap_or(""),
            "reason for no actionable opportunity",
        )?;
    }
    for opportunity in &input.opportunities {
        for (field, value) in [
            ("opportunity title", &opportunity.title),
            ("customer relevance", &opportunity.customer_relevance),
            ("why now", &opportunity.why_now),
            ("counterevidence", &opportunity.counterevidence),
            ("unknowns", &opportunity.unknowns),
            ("validation action", &opportunity.validation_action),
        ] {
            required(value, field)?;
        }
        if opportunity.sources.is_empty() {
            return Err(BusinessError::Invalid(
                "An opportunity needs source evidence".into(),
            ));
        }
        validate_sources(&opportunity.sources)?;
        if let Some(draft) = &opportunity.experiment_draft {
            validate_proposal(draft)?;
        }
    }
    if let Some(suggestion) = &input.brief_suggestion {
        validate_brief(suggestion)?;
    }
    Ok(())
}

fn action_materials(
    workspace: &Workspace,
    experiment_id: &str,
    version: u32,
    self_prepared: bool,
) -> Result<()> {
    let attempts: Vec<_> = workspace
        .attempts
        .iter()
        .filter(|a| a.experiment_id.as_deref() == Some(experiment_id))
        .collect();
    if attempts.iter().any(|a| unresolved_effects(a)) {
        return Err(BusinessError::Conflict(
            "Reconcile unknown effects before recording an external action".into(),
        ));
    }
    if !self_prepared
        && !attempts.iter().any(|a| {
            a.proposal_version == Some(version)
                && a.returns.iter().any(|r| {
                    a.reviews
                        .iter()
                        .rev()
                        .find(|v| v.return_id == r.input.return_id)
                        .is_some_and(|v| v.accepted)
                })
        })
    {
        return Err(BusinessError::Conflict(
            "Accept returned materials for this version or explicitly record self-preparation"
                .into(),
        ));
    }
    Ok(())
}

fn content_digest(payload: &Value) -> String {
    // serde_json's map order is canonical here; retain omitted optional fields.
    format!("{:x}", Sha256::digest(payload.to_string().as_bytes()))
}

fn checked_original_text(payload: &Value, original_text: Option<String>) -> Result<String> {
    let text = original_text.unwrap_or_else(|| payload.to_string());
    let original: Value = serde_json::from_str(&text)
        .map_err(|e| BusinessError::Invalid(format!("Invalid original return: {e}")))?;
    if &original != payload {
        return Err(BusinessError::Invalid(
            "Original text and return content differ".into(),
        ));
    }
    Ok(text)
}

fn import_return(
    workspace: &mut Workspace,
    normalized: Value,
    original: Value,
    original_text: Option<String>,
) -> Result<Applied> {
    let original_text = checked_original_text(&original, original_text)?;
    let input: ReturnInput = serde_json::from_value(normalized.clone())
        .map_err(|e| BusinessError::Invalid(format!("Invalid return: {e}")))?;
    let digest = content_digest(&normalized);
    let attempt = workspace
        .attempts
        .iter()
        .find(|a| a.id == input.attempt_id)
        .ok_or_else(|| BusinessError::NotFound("Return refers to an unknown attempt".into()))?;
    if let Some(existing) = attempt
        .returns
        .iter()
        .find(|r| r.input.return_id == input.return_id)
    {
        if existing.digest == digest {
            return Ok(unchanged());
        }
        if workspace.return_conflicts.iter().any(|c| {
            c.attempt_id == input.attempt_id
                && c.return_id == input.return_id
                && c.incoming_digest == digest
        }) {
            return Ok(Applied {
                changed: false,
                conflict: true,
                message: "This conflicting return is already preserved",
            });
        }
        workspace.return_conflicts.push(ReturnConflict {
            attempt_id: input.attempt_id,
            return_id: input.return_id,
            existing_digest: existing.digest.clone(),
            incoming_digest: digest,
            original,
            original_text,
            recorded_at: now(),
        });
        return Ok(Applied {
            changed: true,
            conflict: true,
            message: "Conflicting return preserved; effective return unchanged",
        });
    }
    validate_return(&input, attempt)?;
    for evidence in &input.opportunities {
        workspace.opportunities.push(Opportunity {
            id: next_id("opportunity", workspace.opportunities.len()),
            attempt_id: input.attempt_id.clone(),
            return_id: input.return_id.clone(),
            evidence: evidence.clone(),
            disposition: "unreviewed".into(),
            decisions: vec![],
        });
    }
    let attempt = attempt_mut(workspace, &input.attempt_id)?;
    for reference in &input.execution_refs {
        if !attempt.execution_refs.contains(reference) {
            attempt.execution_refs.push(reference.clone());
        }
    }
    attempt.returns.push(Receipt {
        input,
        digest,
        original,
        original_text,
        imported_at: now(),
    });
    Ok(Applied {
        changed: true,
        conflict: false,
        message: "Saved",
    })
}

pub fn apply(workspace: &mut Workspace, command: Command) -> Result<Applied> {
    match command {
        Command::SaveBrief { brief } => {
            validate_brief(&brief)?;
            if workspace.brief.as_ref() == Some(&brief) {
                return Ok(unchanged());
            }
            workspace.brief_history.push(brief.clone());
            workspace.brief = Some(brief);
        }
        Command::CreateResearch {
            context,
            objective,
            authorization_scope,
        } => {
            if workspace.brief.is_none() {
                return Err(BusinessError::Invalid("Save a business brief first".into()));
            }
            required(&objective, "research objective")?;
            required(&authorization_scope, "authorization scope")?;
            let attempt = handoffs::research(workspace, context, objective, authorization_scope);
            workspace.attempts.push(attempt);
        }
        Command::DecideOpportunity {
            opportunity_id,
            disposition,
            reason,
        } => {
            one_of(
                &disposition,
                &["choose", "research", "watch", "dismiss"],
                "opportunity decision",
            )?;
            required(&reason, "decision reason")?;
            let opportunity = workspace
                .opportunities
                .iter_mut()
                .find(|o| o.id == opportunity_id)
                .ok_or_else(|| BusinessError::NotFound("Opportunity not found".into()))?;
            opportunity.decisions.push(OpportunityDecision {
                disposition: disposition.clone(),
                reason,
                recorded_at: now(),
            });
            opportunity.disposition = disposition;
        }
        Command::CreateExperiment {
            opportunity_id,
            proposal,
        } => {
            if workspace.brief.is_none() {
                return Err(BusinessError::Invalid("Save a business brief first".into()));
            }
            validate_proposal(&proposal)?;
            if let Some(id) = &opportunity_id
                && !workspace
                    .opportunities
                    .iter()
                    .any(|o| &o.id == id && o.disposition == "choose")
            {
                return Err(BusinessError::Invalid(
                    "Choose this opportunity before proposing an experiment".into(),
                ));
            }
            workspace.experiments.push(Experiment {
                id: next_id("experiment", workspace.experiments.len()),
                opportunity_id,
                proposals: vec![Proposal {
                    version: 1,
                    content: proposal,
                    created_at: now(),
                }],
                decisions: vec![],
                actions: vec![],
                observations: vec![],
                next_steps: vec![],
            });
        }
        Command::CreateDraftExperiment {
            opportunity_id,
            proposal,
        } => {
            if workspace.brief.is_none() {
                return Err(BusinessError::Invalid("Save a business brief first".into()));
            }
            let opportunity = workspace
                .opportunities
                .iter()
                .find(|o| o.id == opportunity_id)
                .ok_or_else(|| BusinessError::NotFound("Opportunity not found".into()))?;
            if workspace
                .experiments
                .iter()
                .any(|e| e.opportunity_id.as_ref() == Some(&opportunity_id))
            {
                return Ok(unchanged());
            }
            let proposal = proposal
                .or_else(|| opportunity.evidence.experiment_draft.clone())
                .ok_or_else(|| {
                    BusinessError::Invalid(
                        "This opportunity has no experiment draft; supply an edited proposal"
                            .into(),
                    )
                })?;
            validate_proposal(&proposal)?;
            // All validation precedes the atomic selection plus experiment creation.
            let opportunity = workspace
                .opportunities
                .iter_mut()
                .find(|o| o.id == opportunity_id)
                .expect("validated opportunity");
            opportunity.disposition = "choose".into();
            opportunity.decisions.push(OpportunityDecision {
                disposition: "choose".into(),
                reason: "Founder selected this opportunity and reviewed its experiment draft"
                    .into(),
                recorded_at: now(),
            });
            workspace.experiments.push(Experiment {
                id: next_id("experiment", workspace.experiments.len()),
                opportunity_id: Some(opportunity_id),
                proposals: vec![Proposal {
                    version: 1,
                    content: proposal,
                    created_at: now(),
                }],
                decisions: vec![],
                actions: vec![],
                observations: vec![],
                next_steps: vec![],
            });
        }
        Command::ReviseProposal {
            experiment_id,
            proposal,
        } => {
            validate_proposal(&proposal)?;
            let experiment = experiment_mut(workspace, &experiment_id)?;
            if experiment
                .proposals
                .last()
                .is_some_and(|p| p.content == proposal)
            {
                return Ok(unchanged());
            }
            experiment.proposals.push(Proposal {
                version: experiment.proposals.len() as u32 + 1,
                content: proposal,
                created_at: now(),
            });
        }
        Command::DecideProposal {
            experiment_id,
            version,
            approved,
            reason,
        } => {
            required(&reason, "decision reason")?;
            let experiment = experiment_mut(workspace, &experiment_id)?;
            if experiment.proposals.last().map(|p| p.version) != Some(version) {
                return Err(BusinessError::Conflict(
                    "Decisions must reference the current proposal version".into(),
                ));
            }
            experiment.decisions.push(Decision {
                version,
                approved,
                reason,
                recorded_at: now(),
            });
        }
        Command::PrepareHandoff {
            experiment_id,
            version,
            context,
        } => {
            let experiment = workspace
                .experiments
                .iter()
                .find(|e| e.id == experiment_id)
                .ok_or_else(|| BusinessError::NotFound("Experiment not found".into()))?;
            authorized(experiment, version)?;
            if workspace
                .attempts
                .iter()
                .any(|a| a.experiment_id.as_ref() == Some(&experiment_id) && unresolved_effects(a))
            {
                return Err(BusinessError::Conflict(
                    "Reconcile unknown external effects before preparing another attempt".into(),
                ));
            }
            let attempt = handoffs::preparation(workspace, experiment, context);
            workspace.attempts.push(attempt);
        }
        Command::AttachExecution {
            attempt_id,
            execution_ref,
        } => {
            validate_execution_ref(&execution_ref)?;
            let attempt = attempt_mut(workspace, &attempt_id)?;
            if attempt.execution_refs.contains(&execution_ref) {
                return Ok(unchanged());
            }
            attempt.execution_refs.push(execution_ref);
        }
        Command::ImportReturn {
            payload,
            original_text,
        } => {
            return import_return(workspace, payload.clone(), payload, original_text);
        }
        Command::ImportTaskReturn {
            attempt_id,
            payload,
            original_text,
        } => {
            // Check the original bytes before making any task-scoped changes.
            let original_text = checked_original_text(&payload, original_text)?;
            if !workspace.attempts.iter().any(|a| a.id == attempt_id) {
                return Err(BusinessError::NotFound(
                    "Selected handoff task not found".into(),
                ));
            }
            let mut normalized = payload.clone();
            let object = normalized
                .as_object_mut()
                .ok_or_else(|| BusinessError::Invalid("Return must be a JSON object".into()))?;
            match object.get("attempt_id") {
                Some(Value::String(id)) if id == &attempt_id => {}
                None => {
                    object.insert("attempt_id".into(), Value::String(attempt_id));
                }
                _ => {
                    return Err(BusinessError::Invalid(
                        "Return attempt_id does not match the selected task".into(),
                    ));
                }
            }
            let missing_return_id = match object.get("return_id") {
                None => true,
                Some(Value::String(id)) => id.trim().is_empty(),
                _ => {
                    return Err(BusinessError::Invalid(
                        "return_id must be text or omitted".into(),
                    ));
                }
            };
            if missing_return_id {
                object.remove("return_id");
                let identity = content_digest(&normalized);
                normalized.as_object_mut().expect("object").insert(
                    "return_id".into(),
                    Value::String(format!("return-{identity}")),
                );
            }
            return import_return(workspace, normalized, payload, Some(original_text));
        }
        Command::ReviewReturn {
            attempt_id,
            return_id,
            accepted,
            note,
        } => {
            required(&note, "review note")?;
            let attempt = attempt_mut(workspace, &attempt_id)?;
            let receipt = attempt
                .returns
                .iter()
                .find(|r| r.input.return_id == return_id)
                .ok_or_else(|| BusinessError::NotFound("Return not found".into()))?;
            if accepted && receipt.input.deliverables.is_empty() {
                return Err(BusinessError::Invalid(
                    "There are no deliverables to accept".into(),
                ));
            }
            attempt.reviews.push(Review {
                return_id,
                accepted,
                note,
                recorded_at: now(),
            });
        }
        Command::ReconcileEffects {
            attempt_id,
            return_id,
            external_effects,
            evidence,
        } => {
            required(&evidence, "reconciliation evidence")?;
            one_of(
                &external_effects,
                &["none", "confirmed"],
                "reconciled external effects",
            )?;
            let attempt = attempt_mut(workspace, &attempt_id)?;
            if !attempt
                .returns
                .iter()
                .any(|r| r.input.return_id == return_id && r.input.external_effects == "unknown")
            {
                return Err(BusinessError::Invalid(
                    "Only an unknown effect can be reconciled".into(),
                ));
            }
            attempt.reconciliations.push(Reconciliation {
                return_id,
                external_effects,
                evidence,
                recorded_at: now(),
            });
        }
        Command::RecordAction {
            experiment_id,
            version,
            reference,
            note,
            self_prepared,
        } => {
            required(&reference, "actual action reference")?;
            required(&note, "action evidence/note")?;
            let experiment = workspace
                .experiments
                .iter()
                .find(|e| e.id == experiment_id)
                .ok_or_else(|| BusinessError::NotFound("Experiment not found".into()))?;
            authorized(experiment, version)?;
            action_materials(workspace, &experiment_id, version, self_prepared)?;
            let experiment = experiment_mut(workspace, &experiment_id)?;
            experiment.actions.push(Action {
                id: next_id("action", experiment.actions.len()),
                version,
                reference,
                note,
                self_prepared,
                historical: false,
                completed_at: None,
                recorded_at: now(),
            });
        }
        Command::RecordHistoricalAction {
            experiment_id,
            version,
            reference,
            note,
            self_prepared,
            completed_at,
        } => {
            required(&reference, "historical action reference")?;
            required(&note, "founder-supplied historical action evidence/note")?;
            let experiment = workspace
                .experiments
                .iter()
                .find(|e| e.id == experiment_id)
                .ok_or_else(|| BusinessError::NotFound("Experiment not found".into()))?;
            let index = experiment
                .proposals
                .iter()
                .position(|p| p.version == version)
                .ok_or_else(|| {
                    BusinessError::NotFound("Historical proposal version not found".into())
                })?;
            let next = experiment.proposals.get(index + 1).ok_or_else(|| {
                BusinessError::Conflict("Use RecordAction for the current proposal version".into())
            })?;
            if completed_at == 0 || completed_at > now() {
                return Err(BusinessError::Invalid(
                    "Completion time must be a nonzero past timestamp".into(),
                ));
            }
            if completed_at < experiment.proposals[index].created_at
                || completed_at > next.created_at
            {
                return Err(BusinessError::Conflict("Completion must fall within the original proposal's interval, up to its next revision".into()));
            }
            let decision = experiment
                .decisions
                .iter()
                .enumerate()
                .filter(|(_, d)| d.version == version && d.recorded_at <= completed_at)
                .max_by_key(|(index, d)| (d.recorded_at, *index));
            if !decision.is_some_and(|(_, d)| d.approved) {
                return Err(BusinessError::Conflict(
                    "The original proposal was not approved at completion time".into(),
                ));
            }
            action_materials(workspace, &experiment_id, version, self_prepared)?;
            let experiment = experiment_mut(workspace, &experiment_id)?;
            experiment.actions.push(Action {
                id: next_id("action", experiment.actions.len()),
                version,
                reference,
                note,
                self_prepared,
                historical: true,
                completed_at: Some(completed_at),
                recorded_at: now(),
            });
        }
        Command::RecordObservation {
            experiment_id,
            action_id,
            outcome,
            evidence,
            comparison,
            learning,
        } => {
            one_of(
                &outcome,
                &["success", "failure", "inconclusive"],
                "business outcome",
            )?;
            required(&evidence, "real observation evidence")?;
            required(&comparison, "comparison with original criteria")?;
            required(&learning, "learning")?;
            let experiment = experiment_mut(workspace, &experiment_id)?;
            let action = experiment
                .actions
                .iter()
                .find(|a| a.id == action_id)
                .ok_or_else(|| BusinessError::NotFound("Record the actual action first".into()))?;
            experiment.observations.push(Observation {
                id: next_id("observation", experiment.observations.len()),
                action_id,
                version: action.version,
                outcome,
                evidence,
                comparison,
                learning,
                recorded_at: now(),
            });
        }
        Command::DecideNext {
            experiment_id,
            observation_id,
            choice,
            reason,
        } => {
            one_of(&choice, &["continue", "adjust", "stop"], "next step")?;
            required(&reason, "next-step reason")?;
            let experiment = experiment_mut(workspace, &experiment_id)?;
            if !experiment
                .observations
                .iter()
                .any(|o| o.id == observation_id)
            {
                return Err(BusinessError::NotFound("Observation not found".into()));
            }
            experiment.next_steps.push(NextStep {
                observation_id,
                choice,
                reason,
                recorded_at: now(),
            });
        }
        Command::AdoptLearning {
            experiment_id,
            observation_id,
        } => {
            if workspace
                .adopted_learning
                .iter()
                .any(|l| l.experiment_id == experiment_id && l.observation_id == observation_id)
            {
                return Ok(unchanged());
            }
            let observation = workspace
                .experiments
                .iter()
                .find(|e| e.id == experiment_id)
                .and_then(|e| e.observations.iter().find(|o| o.id == observation_id))
                .ok_or_else(|| BusinessError::NotFound("Observation not found".into()))?;
            workspace.adopted_learning.push(AdoptedLearning {
                experiment_id,
                observation_id,
                text: observation.learning.clone(),
                recorded_at: now(),
            });
        }
    }
    Ok(Applied {
        changed: true,
        conflict: false,
        message: "Saved",
    })
}

fn unchanged() -> Applied {
    Applied {
        changed: false,
        conflict: false,
        message: "Already recorded; no duplicate created",
    }
}
