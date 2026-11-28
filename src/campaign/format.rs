use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: u8,
    pub campaign: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_utc: Option<[String; 2]>,
    pub items: Vec<PlanItem>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<serde_yaml::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanItem {
    pub id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub target: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<String>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<serde_yaml::Value>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub items: Vec<String>,
    pub hashes: Vec<String>,
    pub digest: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<Approval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<serde_yaml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub records: Option<Vec<serde_yaml::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observations: Option<Vec<serde_yaml::Value>>,
}
#[derive(Debug, Clone)]
pub struct Campaign {
    pub text: String,
    pub plan: Plan,
    pub payloads: HashMap<String, String>,
    pub records: Vec<serde_yaml::Value>,
    pub approval: Option<Approval>,
    pub execution: Option<(usize, usize)>,
    pub newline: &'static str,
}

fn validate_plan(plan: &Plan) -> Result<(), String> {
    let valid_name = |name: &str| {
        !name.is_empty()
            && name.as_bytes()[0].is_ascii_alphanumeric()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    };
    if plan.schema != 1 || plan.items.is_empty() || !valid_name(&plan.campaign) {
        return Err("Invalid schema-1 campaign plan".into());
    }
    if let Some(w) = &plan.window_utc {
        let start = OffsetDateTime::parse(&w[0], &Rfc3339).map_err(|e| e.to_string())?;
        let end = OffsetDateTime::parse(&w[1], &Rfc3339).map_err(|e| e.to_string())?;
        if start >= end {
            return Err("The campaign window must end after it starts".into());
        }
    }
    let ids: HashSet<_> = plan.items.iter().map(|i| i.id.as_str()).collect();
    if ids.len() != plan.items.len() {
        return Err("Campaign item IDs must be unique".into());
    }
    for item in &plan.items {
        if !valid_name(&item.id)
            || item.target.is_empty()
            || item.account.as_ref().is_some_and(String::is_empty)
            || item.tag.as_ref().is_some_and(String::is_empty)
            || item.commit.as_ref().is_some_and(|v| {
                v.len() != 40
                    || !v
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
            || item
                .status
                .as_deref()
                .is_some_and(|v| !matches!(v, "hold" | "paused"))
            || item.mode.as_deref().is_some_and(|v| {
                !matches!(v, "prefill_if_permitted" | "human_entry" | "human_upload")
            })
            || item.steps.as_ref().is_some_and(|steps| {
                steps.is_empty()
                    || steps.iter().any(|s| {
                        !matches!(s.as_str(), "save_draft" | "publish" | "notify_followers")
                    })
            })
        {
            return Err(format!("Invalid campaign item: {}", item.id));
        }
        if let Some(at) = &item.at {
            OffsetDateTime::parse(at, &Rfc3339).map_err(|e| e.to_string())?;
            if let Some(w) = &plan.window_utc {
                let at = OffsetDateTime::parse(at, &Rfc3339).map_err(|e| e.to_string())?;
                let start = OffsetDateTime::parse(&w[0], &Rfc3339).map_err(|e| e.to_string())?;
                let end = OffsetDateTime::parse(&w[1], &Rfc3339).map_err(|e| e.to_string())?;
                if at < start || at >= end {
                    return Err("Every item at time must be inside the campaign window".into());
                }
            }
        }
        if let Some(after) = &item.after {
            if !ids.contains(after.as_str()) || after == &item.id {
                return Err(format!("Invalid dependency for {}", item.id));
            }
        }
        if item.steps.is_some() && (item.content.is_none() || item.mode.is_some()) {
            return Err(format!("Invalid executable plan for {}", item.id));
        }
    }
    Ok(())
}
fn yaml(s: &str) -> Result<serde_yaml::Value, String> {
    let mut block_indent = None;
    for line in s.lines() {
        let indent = line.len() - line.trim_start().len();
        if let Some(parent_indent) = block_indent {
            if line.trim().is_empty() || indent > parent_indent {
                continue;
            }
            block_indent = None;
        }
        let trimmed = line.trim_start();
        if trimmed.contains(": |")
            || trimmed.contains(": >")
            || trimmed.ends_with(": |-")
            || trimmed.ends_with(": >-")
        {
            block_indent = Some(indent);
            continue;
        }
        let value = trimmed
            .split_once(':')
            .map(|(_, value)| value.trim_start())
            .or_else(|| trimmed.strip_prefix("- ").map(str::trim_start))
            .unwrap_or("");
        if value.starts_with('&') || value.starts_with('*') {
            return Err("YAML aliases and anchors are not supported".into());
        }
    }
    serde_yaml::from_str(s).map_err(|e| e.to_string())
}
pub fn parse_campaign(text: &str) -> Result<Campaign, String> {
    let (nl, front_end) = match text.find('\n') {
        Some(i) if i > 0 && text.as_bytes()[i - 1] == b'\r' => ("\r\n", i + 1),
        Some(i) => ("\n", i + 1),
        None => ("\n", text.len()),
    };
    let first_line = text[..front_end].trim_end_matches(['\r', '\n']);
    if first_line.trim_end_matches([' ', '\t']) != "---" || front_end == text.len() {
        return Err("Start the campaign with YAML front matter".into());
    }
    let mut closing = None;
    let mut cursor = front_end;
    for line in text[front_end..].split_inclusive('\n') {
        let bare = line.trim_end_matches(['\r', '\n']);
        if bare.trim_matches([' ', '\t']) == "---" {
            closing = Some((cursor, cursor + line.len()));
            break;
        }
        cursor += line.len();
    }
    let (front_close, body_start) = closing.ok_or("Unclosed campaign front matter")?;
    let front = &text[front_end..front_close];
    let plan: Plan = serde_yaml::from_value(yaml(front)?).map_err(|e| e.to_string())?;
    validate_plan(&plan)?;
    let mut sections: HashMap<String, (usize, usize)> = HashMap::new();
    let mut section: Option<String> = None;
    let mut records = Vec::new();
    let mut approval = None;
    let mut fence: Option<(char, usize, usize, bool)> = None;
    let mut offset = body_start;
    for line in text[body_start..].split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let bare = line.trim_end_matches(['\r', '\n']);
        let t = bare.trim_start();
        if let Some((ch, count, content, execution)) = fence {
            if t.starts_with(&ch.to_string().repeat(count)) && t[count..].trim().is_empty() {
                if execution {
                    let ent: ExecutionEntry = serde_yaml::from_value(yaml(&text[content..start])?)
                        .map_err(|e| e.to_string())?;
                    if let Some(a) = ent.approval {
                        if a.items.is_empty()
                            || a.hashes.is_empty()
                            || a.items.iter().any(|item| {
                                !item
                                    .bytes()
                                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                            })
                            || a.hashes.iter().any(|hash| !valid_digest(hash))
                            || !valid_digest(&a.digest)
                        {
                            return Err("Invalid campaign approval".into());
                        }
                        if let Some(expires) = &a.expires_at {
                            OffsetDateTime::parse(expires, &Rfc3339).map_err(|e| e.to_string())?;
                        }
                        approval = Some(a)
                    }
                    if let Some(r) = ent.records {
                        records.extend(r)
                    }
                    if let Some(r) = ent.record {
                        records.push(r)
                    }
                }
                fence = None;
            }
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            let ch = t.chars().next().unwrap();
            let count = t.chars().take_while(|c| *c == ch).count();
            let exec = section.as_deref() == Some("Execution") && t[count..].trim() == "yaml";
            fence = Some((ch, count, offset, exec));
            continue;
        }
        if let Some(h) = t.strip_prefix("## ") {
            if let Some(s) = section.take() {
                if let Some(r) = sections.get_mut(&s) {
                    r.1 = start;
                }
            }
            let name = h.trim().trim_end_matches('#').trim().to_string();
            if sections.contains_key(&name) {
                return Err(format!("Duplicate section: {name}"));
            }
            sections.insert(name.clone(), (offset, text.len()));
            section = Some(name);
        }
    }
    if fence.is_some() {
        return Err("Unclosed Markdown fence".into());
    }
    if let Some(s) = section {
        if let Some(r) = sections.get_mut(&s) {
            r.1 = text.len()
        }
    }
    let mut payloads = HashMap::new();
    for item in &plan.items {
        if let Some(name) = &item.content {
            let (a, b) = sections
                .get(name)
                .ok_or_else(|| format!("Missing payload: {name}"))?;
            if name == "Execution" {
                return Err("Missing payload: Execution".into());
            }
            let v = text[*a..*b].replace("\r\n", "\n");
            let mut lines: Vec<&str> = v.split('\n').collect();
            while lines.first().is_some_and(|line| line.trim().is_empty()) {
                lines.remove(0);
            }
            while lines.last().is_some_and(|line| line.trim().is_empty()) {
                lines.pop();
            }
            payloads.insert(name.clone(), lines.join("\n"));
        }
    }
    let execution = sections.get("Execution").copied();
    Ok(Campaign {
        text: text.into(),
        plan,
        payloads,
        records,
        approval,
        execution,
        newline: nl,
    })
}
fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
pub fn append_execution(c: &Campaign, entry: &ExecutionEntry) -> Result<String, String> {
    let nl = c.newline;
    let serialized = serde_yaml::to_string(entry).map_err(|e| e.to_string())?;
    let serialized = serialized.strip_prefix("---\n").unwrap_or(&serialized);
    let serialized = serialized
        .lines()
        .map(|line| {
            if line.starts_with("  - ") {
                format!("  {line}{nl}")
            } else {
                format!("{line}{nl}")
            }
        })
        .collect::<String>();
    let end = c.execution.map_or(c.text.len(), |r| r.1);
    let prefix = &c.text[..end];
    let suffix = &c.text[end..];
    let sep = if prefix.ends_with(&format!("{nl}{nl}")) {
        ""
    } else if prefix.ends_with(nl) {
        nl
    } else {
        if nl == "\r\n" { "\r\n\r\n" } else { "\n\n" }
    };
    let heading = if c.execution.is_some() {
        String::new()
    } else {
        format!("## Execution{nl}{nl}")
    };
    let block = format!("{heading}```yaml{nl}{serialized}```{nl}");
    Ok(format!(
        "{prefix}{sep}{block}{}{suffix}",
        if suffix.is_empty() { "" } else { nl }
    ))
}
