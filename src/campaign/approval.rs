use super::format::{Approval, Campaign, PlanItem, parse_timestamp, valid_hex};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CampaignAction {
    pub kind: String,
    pub item: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub version: String,
    #[serde(skip)]
    pub request_key: String,
    #[serde(skip)]
    pub remote_key: String,
}
fn sha(value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    h.update(bytes);
    Ok(format!("sha256:{:x}", h.finalize()))
}
fn jopt(v: &Option<String>) -> Value {
    v.as_ref().map_or(Value::Null, |s| json!(s))
}
pub(super) fn bluesky_handle(value: &str) -> bool {
    value.len() <= 253
        && value.split('.').count() >= 2
        && value.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && part.as_bytes()[0].is_ascii_alphanumeric()
                && part.as_bytes()[part.len() - 1].is_ascii_alphanumeric()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        && value
            .rsplit('.')
            .next()
            .is_some_and(|part| part.starts_with(|c: char| c.is_ascii_lowercase()))
}
pub(super) fn repo_syntax(path: &str) -> bool {
    path.split_once('/').is_some_and(|(owner, repo)| {
        !owner.is_empty()
            && owner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            && !repo.is_empty()
            && repo
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    })
}
fn valid_release_tag(tag: &str) -> bool {
    !tag.chars().any(|c| {
        c.is_whitespace()
            || matches!(c, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
            || c <= '\u{1f}'
            || c == '\u{7f}'
    }) && !tag.contains("..")
        && !tag.contains("@{")
        && !tag.ends_with('.')
        && tag
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.') && !part.ends_with(".lock"))
}
fn action(item: &PlanItem, c: &Campaign) -> Result<Option<CampaignAction>, String> {
    if item.status.is_some() {
        return Ok(None);
    }
    let Some(steps) = item.steps.as_ref() else {
        return Ok(None);
    };
    if let Some(raw_account) = item.target.strip_prefix("bluesky/") {
        let a = raw_account
            .strip_prefix('@')
            .unwrap_or(raw_account)
            .to_lowercase();
        let text = c
            .payloads
            .get(item.content.as_deref().unwrap_or(""))
            .ok_or("Missing Bluesky payload")?
            .clone();
        if steps.as_slice() != ["publish"]
            || !bluesky_handle(&a)
            || item
                .account
                .as_ref()
                .is_some_and(|x| x.strip_prefix('@').unwrap_or(x).to_lowercase() != a)
            || item.tag.is_some()
            || item.commit.is_some()
            || item.assets.as_ref().is_some_and(|a| !a.is_empty())
            || text.trim().is_empty()
            || text.contains('\0')
            || text.len() > 3000
            || text.graphemes(true).count() > 300
        {
            return Err(format!("Invalid Bluesky text post: {}", item.id));
        }
        let target = format!("bluesky/{a}");
        let request = sha(&json!([target, "publish", text]))?;
        let remote = format!("{target}@{}", sha(&json!(text))?);
        let version = sha(&json!([item.id, request, item.after, item.at]))?;
        return Ok(Some(CampaignAction {
            kind: "bluesky".into(),
            item: item.id.clone(),
            after: item.after.clone(),
            at: item.at.clone(),
            target: Some(target),
            account: Some(a),
            text: Some(text),
            repo: None,
            tag: None,
            commit: None,
            title: None,
            body: None,
            version,
            request_key: request,
            remote_key: remote,
        }));
    }
    if let Some(path) = item.target.strip_prefix("github/") {
        if steps.as_slice() != ["publish"]
            || !repo_syntax(path)
            || path.ends_with("/.")
            || path.ends_with("/..")
        {
            return Err(format!("Invalid GitHub Release action: {}", item.id));
        }
        let repo = path.to_ascii_lowercase();
        let account = item
            .account
            .as_deref()
            .ok_or("Missing GitHub account")?
            .to_ascii_lowercase();
        let base_account = account.strip_suffix("[bot]").unwrap_or(&account);
        if base_account.is_empty()
            || !base_account.as_bytes()[0].is_ascii_alphanumeric()
            || !base_account.as_bytes()[base_account.len() - 1].is_ascii_alphanumeric()
            || !base_account
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(format!("Invalid GitHub Release action: {}", item.id));
        }
        let tag = item.tag.as_deref().ok_or("Missing GitHub tag")?;
        if !valid_release_tag(tag)
            || item
                .commit
                .as_deref()
                .is_some_and(|commit| !valid_hex(commit, 40))
            || item
                .assets
                .as_ref()
                .is_some_and(|assets| !assets.is_empty())
        {
            return Err(format!("Invalid GitHub Release action: {}", item.id));
        }
        let p = c
            .payloads
            .get(item.content.as_deref().unwrap_or(""))
            .ok_or("Missing release payload")?;
        let (titleline, mut body) = p.split_once('\n').ok_or("Set a Title and Markdown body")?;
        let title = titleline
            .strip_prefix("Title:")
            .filter(|v| v.starts_with(' ') || v.starts_with('\t'))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or("Set a Title and Markdown body")?
            .to_string();
        while let Some((line, rest)) = body.split_once('\n')
            && line.trim_matches([' ', '\t']).is_empty()
        {
            body = rest;
        }
        if body.trim().is_empty() {
            return Err("Set a Title and Markdown body".into());
        }
        let version = sha(&json!([
            item.id,
            repo,
            account,
            "publish",
            tag,
            jopt(&item.commit),
            title,
            body,
            jopt(&item.after),
            jopt(&item.at)
        ]))?;
        let request = sha(&json!([version, "publish"]))?;
        let remote = format!("github/{repo}@{tag}");
        return Ok(Some(CampaignAction {
            kind: "github".into(),
            item: item.id.clone(),
            after: item.after.clone(),
            at: item.at.clone(),
            target: None,
            account: Some(account),
            text: None,
            repo: Some(repo),
            tag: Some(tag.into()),
            commit: item.commit.clone(),
            title: Some(title),
            body: Some(body.into()),
            version,
            request_key: request,
            remote_key: remote,
        }));
    }
    Ok(None)
}
pub fn approval_batch(
    c: &Campaign,
) -> Result<(Vec<CampaignAction>, String, Option<String>), String> {
    let mut actions = Vec::new();
    let mut remotes = std::collections::HashSet::new();
    let mut bluesky_requests = std::collections::HashSet::new();
    for i in &c.plan.items {
        if let Some(a) = action(i, c)? {
            if a.kind == "github" && !remotes.insert(a.remote_key.clone()) {
                return Err(format!("Duplicate GitHub Release target: {}", a.remote_key));
            }
            if a.kind == "bluesky" && !bluesky_requests.insert(a.request_key.clone()) {
                return Err(format!(
                    "Duplicate Bluesky text for {}",
                    a.target.as_deref().unwrap_or("Bluesky")
                ));
            }
            actions.push(a)
        }
    }
    let rows: Vec<_> = actions
        .iter()
        .map(|a| json!([a.item, a.version, a.after]))
        .collect();
    let window = c
        .plan
        .window_utc
        .as_ref()
        .map(|w| json!(w))
        .unwrap_or(Value::Null);
    let digest = sha(&json!([c.plan.campaign, window, rows]))?;
    let expires = c.plan.window_utc.as_ref().map(|w| w[1].clone());
    Ok((actions, digest, expires))
}
pub fn approve_campaign(c: &Campaign, reviewed: &str) -> Result<Approval, String> {
    let (a, d, e) = approval_batch(c)?;
    if a.is_empty() {
        return Err("No runnable campaign actions to approve".into());
    }
    if d != reviewed {
        return Err("Campaign changed during review. Review it again.".into());
    }
    if let Some(expires) = &e
        && OffsetDateTime::now_utc() >= parse_timestamp(expires)?
    {
        return Err("Campaign window has expired".into());
    }
    Ok(Approval {
        items: a.iter().map(|x| x.item.clone()).collect(),
        hashes: a.iter().map(|x| x.version.clone()).collect(),
        digest: d,
        expires_at: e,
    })
}

pub fn approved(
    campaign: &Campaign,
    batch_expires_at: Option<&str>,
    action: &CampaignAction,
) -> bool {
    campaign.approval.as_ref().is_some_and(|approval| {
        approval.expires_at.as_deref() == batch_expires_at
            && approval.hashes.contains(&action.version)
    })
}

pub fn in_window(campaign: &Campaign, now: OffsetDateTime) -> bool {
    campaign
        .plan
        .window_utc
        .as_ref()
        .is_none_or(|[start, end]| {
            parse_timestamp(start).is_ok_and(|start| now >= start)
                && parse_timestamp(end).is_ok_and(|end| now < end)
        })
}
