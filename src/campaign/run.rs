// Portions ported from ArtiPub, Copyright (c) 2019 Crawlab Team.
// Licensed under the BSD-3-Clause License; see THIRD_PARTY_NOTICES.md.
use super::{
    ActionRecord, Bluesky, BlueskyIntent, CampaignAction, CampaignStore, ExecutionEntry,
    GitHubReleases, Intent, Milestone, RecordStatus, ReleaseIntent, Step, approval_batch, approved,
    bluesky,
    format::{ABSENCE_SOURCE, parse_timestamp},
    github, in_window, timestamp,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct CampaignResult {
    pub item: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
}

impl CampaignResult {
    fn new(item: &str, status: &str, reason: Option<String>) -> Self {
        Self {
            item: item.into(),
            status: status.into(),
            reason,
            url: None,
            native_id: None,
            cid: None,
        }
    }

    fn public(record: &ActionRecord) -> Self {
        Self {
            url: record.url.clone(),
            native_id: record.cid.as_ref().and(record.native_id.clone()),
            cid: record.cid.clone(),
            ..Self::new(&record.item, "public", None)
        }
    }
}

fn latest_records(records: &[ActionRecord]) -> Vec<&ActionRecord> {
    let mut latest = Vec::new();
    let mut indices = HashMap::new();
    for record in records {
        if let Some(key) = &record.request_key {
            let index = *indices.entry(key).or_insert(latest.len());
            if index == latest.len() {
                latest.push(record);
            } else {
                latest[index] = record;
            }
        }
    }
    latest
}

fn unsettled(record: &ActionRecord) -> bool {
    matches!(
        record.status,
        Some(RecordStatus::Reserved | RecordStatus::Unknown | RecordStatus::Pending)
    )
}

fn reservation(action: &CampaignAction, intent: Intent) -> ActionRecord {
    ActionRecord {
        item: action.item.clone(),
        milestone: Milestone::Prepared,
        version: Some(action.version.clone()),
        step: Some(Step::Publish),
        request_key: Some(action.request_key.clone()),
        remote_key: Some(action.remote_key.clone()),
        intent: Some(intent),
        attempted_at: None,
        status: Some(RecordStatus::Reserved),
        cid: None,
        native_id: None,
        url: None,
        observed_at: None,
        source: None,
        reason: None,
        extra: BTreeMap::new(),
    }
}

fn append(store: &mut CampaignStore, mut record: ActionRecord) -> Result<(), String> {
    // New fences use only fields understood by the campaign runner.
    record.extra.clear();
    store.append(&ExecutionEntry {
        record: Some(record),
        ..ExecutionEntry::default()
    })
}

fn record_public(
    store: &mut CampaignStore,
    mut record: ActionRecord,
    source: &str,
) -> Result<CampaignResult, String> {
    record.intent = None;
    record.milestone = Milestone::Public;
    record.status = None;
    record.observed_at = Some(timestamp());
    record.source = Some(source.into());
    record.reason = None;
    let result = CampaignResult::public(&record);
    append(store, record)?;
    Ok(result)
}

fn record_bluesky(
    store: &mut CampaignStore,
    mut record: ActionRecord,
    lookup: bluesky::Lookup,
) -> Result<CampaignResult, String> {
    let old_status = record.status;
    let old_reason = record.reason.clone();
    let old_cid = record.cid.clone();
    let old_native_id = record.native_id.clone();
    let (status, label, reason) = match lookup {
        bluesky::Lookup::Public(evidence) => {
            record.native_id = Some(evidence.receipt.uri);
            record.cid = Some(evidence.receipt.cid);
            record.url = Some(evidence.url);
            return record_public(store, record, "bluesky_public_appview");
        }
        bluesky::Lookup::Pending(evidence) => {
            record.native_id = Some(evidence.receipt.uri);
            record.cid = Some(evidence.receipt.cid);
            (
                RecordStatus::Pending,
                "pending",
                "Bluesky AppView has not indexed the repository record yet. Rerun to reconcile."
                    .into(),
            )
        }
        bluesky::Lookup::UnverifiedView(evidence, reason) => {
            record.native_id = Some(evidence.receipt.uri);
            record.cid = Some(evidence.receipt.cid);
            (RecordStatus::Unknown, "unknown", reason)
        }
        bluesky::Lookup::Conflict(reason) | bluesky::Lookup::Unverified(reason) => {
            (RecordStatus::Unknown, "unknown", reason)
        }
        bluesky::Lookup::Missing => (
            RecordStatus::Unknown,
            "unknown",
            "A prior Bluesky write may still take effect. Rerun to reconcile by record key.".into(),
        ),
    };
    record.status = Some(status);
    record.reason = Some(reason.clone());
    let result = CampaignResult {
        native_id: record.native_id.clone(),
        cid: record.cid.clone(),
        ..CampaignResult::new(&record.item, label, Some(reason))
    };
    if record.status != old_status
        || record.reason != old_reason
        || record.cid != old_cid
        || record.native_id != old_native_id
    {
        append(store, record)?;
    }
    Ok(result)
}

fn record_release(
    store: &mut CampaignStore,
    mut record: ActionRecord,
    lookup: github::Lookup,
) -> Result<CampaignResult, String> {
    let reason = match lookup {
        github::Lookup::Found { id, url } => {
            record.native_id = Some(id);
            record.url = Some(url);
            return record_public(store, record, "github_public_api");
        }
        github::Lookup::Blocked(reason) => reason,
        github::Lookup::Absent => {
            "A prior write may still take effect. Check the release by tag before further action."
                .into()
        }
    };
    let result = CampaignResult::new(&record.item, "unknown", Some(reason.clone()));
    if record.status != Some(RecordStatus::Unknown) || record.reason.as_ref() != Some(&reason) {
        record.status = Some(RecordStatus::Unknown);
        record.reason = Some(reason);
        append(store, record)?;
    }
    Ok(result)
}

fn blocked(item: &str, reason: impl Into<String>) -> CampaignResult {
    CampaignResult::new(item, "blocked", Some(reason.into()))
}

fn before_dispatch(store: &CampaignStore, item: &str) -> Result<Option<CampaignResult>, String> {
    if store.paused()? {
        return Ok(Some(CampaignResult::new(item, "paused", None)));
    }
    if !in_window(store.campaign(), OffsetDateTime::now_utc()) {
        return Ok(Some(CampaignResult::new(
            item,
            "hold",
            Some("Outside the approved campaign window".into()),
        )));
    }
    Ok(None)
}

fn stop_reserved(
    store: &mut CampaignStore,
    record: &ActionRecord,
    result: CampaignResult,
) -> Result<CampaignResult, String> {
    let mut stopped = record.clone();
    stopped.status = Some(if result.status == "paused" {
        RecordStatus::Paused
    } else {
        RecordStatus::Blocked
    });
    stopped.reason = Some(if result.status == "paused" {
        "Pause set before dispatch".into()
    } else {
        "Campaign window ended before dispatch".into()
    });
    append(store, stopped)?;
    Ok(result)
}

fn reserve(
    store: &mut CampaignStore,
    record: &mut ActionRecord,
) -> Result<Option<CampaignResult>, String> {
    if let Some(result) = before_dispatch(store, &record.item)? {
        return Ok(Some(result));
    }
    record.attempted_at = Some(timestamp());
    append(store, record.clone())?;
    store.assert_unchanged()?;
    if let Some(result) = before_dispatch(store, &record.item)? {
        return stop_reserved(store, record, result).map(Some);
    }
    Ok(None)
}

async fn execute_release(
    store: &mut CampaignStore,
    github: &GitHubReleases,
    action: &CampaignAction,
    previously_public: bool,
) -> Result<CampaignResult, String> {
    let intent = ReleaseIntent {
        repo: action.repo.clone().expect("GitHub action has a repository"),
        account: action
            .account
            .clone()
            .expect("GitHub action has an account"),
        tag: action.tag.clone().expect("GitHub action has a tag"),
        commit: action.commit.clone(),
        title: action.title.clone().expect("GitHub action has a title"),
        body: action.body.clone().expect("GitHub action has a body"),
    };
    let mut record = reservation(action, Intent::Release(intent.clone()));
    for preflight in [false, true] {
        if preflight && let Err(reason) = github.preflight(&intent).await {
            return Ok(blocked(&action.item, reason));
        }
        match github.lookup(&intent).await {
            github::Lookup::Absent if previously_public => {
                return Ok(CampaignResult::new(
                    &action.item,
                    "hold",
                    Some("This release was public and is now missing. Review it in GitHub.".into()),
                ));
            }
            github::Lookup::Absent => {}
            github::Lookup::Blocked(reason) => return Ok(blocked(&action.item, reason)),
            found @ github::Lookup::Found { .. } => return record_release(store, record, found),
        }
    }
    if let Some(result) = reserve(store, &mut record)? {
        return Ok(result);
    }
    if let Err(reason) = github.create(&intent).await {
        record.intent = None;
        record.status = Some(RecordStatus::Blocked);
        record.reason = Some(reason.clone());
        append(store, record)?;
        return Ok(blocked(&action.item, reason));
    }
    record_release(store, record, github.lookup(&intent).await)
}

fn preflight_bluesky(
    store: &mut CampaignStore,
    record: &ActionRecord,
    lookup: bluesky::Lookup,
    absence: Option<&ActionRecord>,
) -> Result<Option<CampaignResult>, String> {
    match lookup {
        bluesky::Lookup::Missing => Ok(None),
        bluesky::Lookup::Conflict(_) if absence.is_some() => {
            record_bluesky(store, record.clone(), lookup).map(Some)
        }
        bluesky::Lookup::Conflict(reason) | bluesky::Lookup::Unverified(reason) => {
            Ok(Some(blocked(&record.item, reason)))
        }
        lookup => record_bluesky(store, record.clone(), lookup).map(Some),
    }
}

async fn execute_bluesky(
    store: &mut CampaignStore,
    bluesky: &Bluesky,
    action: &CampaignAction,
    previously_public: bool,
    absence: Option<ActionRecord>,
) -> Result<CampaignResult, String> {
    let account = action
        .account
        .as_deref()
        .expect("Bluesky action has an account");
    let text = action.text.as_deref().expect("Bluesky action has text");
    let intent = if let Some(absence) = &absence {
        let Some(Intent::Bluesky(intent)) = &absence.intent else {
            return Err("Verified absence is missing its saved Bluesky intent".into());
        };
        if intent.account != account || intent.text != text {
            return Ok(blocked(
                &action.item,
                "The saved Bluesky intent differs from the approved post",
            ));
        }
        intent.clone()
    } else {
        match bluesky.resolve(account, text, &action.request_key).await {
            Ok(intent) => intent,
            Err(reason) => return Ok(blocked(&action.item, reason)),
        }
    };
    let mut record = reservation(action, Intent::Bluesky(intent.clone()));
    if let Some(absence) = &absence {
        record.cid.clone_from(&absence.cid);
        record.native_id.clone_from(&absence.native_id);
    }
    let lookup = bluesky.lookup(&intent, record.cid.as_deref()).await;
    if let Some(result) = preflight_bluesky(store, &record, lookup, absence.as_ref())? {
        return Ok(result);
    }
    if previously_public {
        return Ok(CampaignResult::new(&action.item, "hold", Some("The campaign already recorded this text as public. Review the public feed before taking further action.".into())));
    }
    if !BlueskyIntent::valid_tid(&intent.rkey) {
        return Ok(blocked(
            &action.item,
            "The saved Bluesky key is a legacy non-TID key. Review it before publishing.",
        ));
    }
    let token = match bluesky.session(&intent).await {
        Ok(token) => token,
        Err(reason) => return Ok(blocked(&action.item, reason)),
    };
    let card = bluesky.card(text).await;
    let lookup = bluesky.lookup(&intent, record.cid.as_deref()).await;
    if let Some(result) = preflight_bluesky(store, &record, lookup, absence.as_ref())? {
        return Ok(result);
    }
    record.cid = None;
    record.native_id = None;
    if let Some(result) = reserve(store, &mut record)? {
        return Ok(result);
    }
    let embed = if let Some(card) = card {
        let embed = bluesky.upload_card(&intent, &token, card).await;
        store.assert_unchanged()?;
        if let Some(result) = before_dispatch(store, &record.item)? {
            return stop_reserved(store, &record, result);
        }
        embed
    } else {
        None
    };
    match bluesky.create(&intent, &token, embed).await {
        Ok(receipt) => {
            let lookup = bluesky.lookup(&intent, Some(&receipt.cid)).await;
            record.native_id = Some(receipt.uri);
            record.cid = Some(receipt.cid);
            record_bluesky(store, record, lookup)
        }
        Err(error) if error.rejected => {
            record.status = Some(RecordStatus::Rejected);
            record.reason = Some(error.reason.clone());
            append(store, record)?;
            Ok(blocked(&action.item, error.reason))
        }
        Err(error) => {
            let reason = format!("{}. Rerun to reconcile by record key.", error.reason);
            record.status = Some(RecordStatus::Unknown);
            record.reason = Some(reason.clone());
            append(store, record)?;
            Ok(CampaignResult::new(&action.item, "unknown", Some(reason)))
        }
    }
}

pub async fn run_campaign(
    root: &Path,
    file: &str,
    github: &GitHubReleases,
    bluesky: &Bluesky,
) -> Result<Vec<CampaignResult>, String> {
    let mut store = CampaignStore::open(root, file)?;
    let (actions, _, expires) = approval_batch(store.campaign())?;
    let pending: Vec<_> = latest_records(&store.campaign().records)
        .into_iter()
        .filter(|record| unsettled(record))
        .cloned()
        .collect();
    let mut results = Vec::new();
    let mut reconciled = HashSet::new();
    for record in pending {
        reconciled.insert(record.request_key.clone().expect("latest requested record"));
        let result = match record.intent.clone() {
            Some(Intent::Bluesky(intent)) => {
                let lookup = bluesky.lookup(&intent, record.cid.as_deref()).await;
                record_bluesky(&mut store, record, lookup)?
            }
            Some(Intent::Retired(_)) => CampaignResult::new(&record.item, "unknown", Some("A retired adapter left an unresolved write. Verify it before publishing an amended plan.".into())),
            Some(Intent::Release(intent)) => record_release(&mut store, record, github.lookup(&intent).await)?,
            None => record_release(&mut store, record, github::Lookup::Absent)?,
        };
        results.push(result);
    }

    for action in &actions {
        if reconciled.contains(&action.request_key) {
            continue;
        }
        let history = &store.campaign().records;
        let records = latest_records(history);
        let prior = records
            .iter()
            .copied()
            .find(|record| record.request_key.as_ref() == Some(&action.request_key));
        let pending = records.iter().any(|record| unsettled(record) && (
            record.item == action.item
                || record.remote_key.as_ref() == Some(&action.remote_key)
                || matches!(&record.intent, Some(Intent::Retired(intent)) if action.kind == "bluesky" && intent.provider == "bluesky" && Some(&intent.account) == action.account.as_ref())
        ));
        let result = if pending {
            CampaignResult::new(
                &action.item,
                "hold",
                Some("Reconcile the prior write before using the amended plan".into()),
            )
        } else if let Some(prior) = prior.filter(|prior| {
            prior.milestone == Milestone::Public && prior.version.as_ref() == Some(&action.version)
        }) {
            CampaignResult::public(prior)
        } else if let Some(prior) =
            prior.filter(|prior| prior.status == Some(RecordStatus::Rejected))
        {
            CampaignResult::new(&action.item, "blocked", prior.reason.clone())
        } else if store.paused()? {
            CampaignResult::new(&action.item, "paused", None)
        } else if !approved(store.campaign(), expires.as_deref(), action) {
            CampaignResult::new(
                &action.item,
                "unapproved",
                Some("Review and approve the current batch".into()),
            )
        } else if !in_window(store.campaign(), OffsetDateTime::now_utc()) {
            CampaignResult::new(
                &action.item,
                "hold",
                Some("Outside the approved campaign window".into()),
            )
        } else if let Some(at) = &action.at
            && OffsetDateTime::now_utc() < parse_timestamp(at)?
        {
            CampaignResult::new(&action.item, "waiting", Some(format!("waiting until {at}")))
        } else if let Some(after) = &action.after
            && !dependency_public(&actions, &records, after)
        {
            CampaignResult::new(
                &action.item,
                "hold",
                Some(format!("Waiting for {after} to become public")),
            )
        } else {
            let previously_public = history.iter().any(|record| {
                record.milestone == Milestone::Public
                    && record.remote_key.as_deref().is_some_and(|key| {
                        key == action.remote_key
                            || (action.kind == "bluesky"
                                && key.strip_prefix("postiz/") == Some(action.remote_key.as_str()))
                    })
            });
            if action.kind == "github" {
                execute_release(&mut store, github, action, previously_public).await?
            } else {
                let absence = prior
                    .and_then(|record| match record.status {
                        Some(RecordStatus::Absent) => Some(record),
                        Some(RecordStatus::Paused | RecordStatus::Blocked) => {
                            history.iter().rev().find(|entry| {
                                entry.status == Some(RecordStatus::Absent)
                                    && entry.request_key == record.request_key
                            })
                        }
                        _ => None,
                    })
                    .cloned();
                execute_bluesky(&mut store, bluesky, action, previously_public, absence).await?
            }
        };
        results.push(result);
    }
    for item in &store.campaign().plan.items {
        if let Some(status) = &item.status {
            results.push(CampaignResult::new(&item.id, status, item.reason.clone()));
        }
    }
    Ok(results)
}

fn dependency_public(actions: &[CampaignAction], records: &[&ActionRecord], after: &str) -> bool {
    actions
        .iter()
        .find(|action| action.item == after)
        .is_some_and(|dependency| {
            records.iter().any(|record| {
                record.item == dependency.item
                    && record.version.as_ref() == Some(&dependency.version)
                    && record.milestone == Milestone::Public
            })
        })
}

pub async fn reconcile_absent(
    root: &Path,
    file: &str,
    item: &str,
    bluesky: &Bluesky,
) -> Result<ActionRecord, String> {
    let mut store = CampaignStore::open(root, file)?;
    let records = &store.campaign().records;
    let mut candidates = latest_records(records)
        .into_iter()
        .filter(|record| record.item == item && unsettled(record));
    let mut record = candidates
        .next()
        .cloned()
        .ok_or("No latest unknown attempt for this item")?;
    if candidates.next().is_some()
        || records
            .iter()
            .any(|record| record.item == item && record.request_key.is_none() && unsettled(record))
    {
        return Err("Multiple or unkeyed unsettled attempts exist for this item".into());
    }
    if record.status != Some(RecordStatus::Unknown)
        || record.milestone != Milestone::Prepared
        || record.version.is_none()
    {
        return Err("The latest attempt is not a versioned unknown write".into());
    }
    let Some(Intent::Bluesky(intent)) = record.intent.as_mut() else {
        return Err("Only unknown Bluesky attempts support absence reconciliation".into());
    };
    bluesky.verify_absent(intent).await?;
    intent.collection = Some(intent.collection().to_owned());
    record.status = Some(RecordStatus::Absent);
    record.observed_at = Some(timestamp());
    record.source = Some(ABSENCE_SOURCE.into());
    record.reason =
        Some("Operator verified HTTP 400 RecordNotFound at the saved Bluesky record key".into());
    record.extra.clear();
    append(&mut store, record.clone())?;
    Ok(record)
}
