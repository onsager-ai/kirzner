use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    mem::MaybeUninit,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::approval::{bluesky_handle, repo_syntax};

// An omitted field defaults to None. Explicit YAML null is outside the schema.
fn defined<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: u8,
    pub campaign: String,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub goal: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub window_utc: Option<[String; 2]>,
    pub items: Vec<PlanItem>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub metrics: Option<Mapping>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanItem {
    pub id: String,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub content: Option<String>,
    pub target: String,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub account: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub steps: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub tag: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub commit: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub mode: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub after: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub at: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub status: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub reason: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub assets: Option<Vec<Asset>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub path: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub items: Vec<String>,
    pub hashes: Vec<String>,
    pub digest: String,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub expires_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEntry {
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub approval: Option<Approval>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub record: Option<ActionRecord>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub records: Option<Vec<ActionRecord>>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub observations: Option<Vec<Mapping>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Milestone {
    Prepared,
    DraftSaved,
    Scheduled,
    Public,
    FollowersNotified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Reserved,
    Unknown,
    Paused,
    Hold,
    Pending,
    Blocked,
    AwaitingPublish,
    AwaitingAuthor,
    AwaitingUpload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    SaveDraft,
    Publish,
    NotifyFollowers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub item: String,
    pub milestone: Milestone,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub version: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub step: Option<Step>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub request_key: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub remote_key: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub intent: Option<Intent>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub attempted_at: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub status: Option<RecordStatus>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub cid: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub native_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub url: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub observed_at: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub source: Option<String>,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub reason: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Intent {
    Release(ReleaseIntent),
    Bluesky(BlueskyIntent),
    Retired(RetiredIntent),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseIntent {
    pub repo: String,
    pub account: String,
    pub tag: String,
    #[serde(
        default,
        deserialize_with = "defined",
        skip_serializing_if = "Option::is_none"
    )]
    pub commit: Option<String>,
    pub title: String,
    pub body: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueskyIntent {
    pub account: String,
    pub did: String,
    pub pds: String,
    pub rkey: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetiredIntent {
    pub provider: String,
    pub account: String,
    pub text: String,
    pub integration_id: String,
    pub actor_id: String,
    pub instance: String,
}
#[derive(Debug, Clone)]
pub struct Campaign {
    pub text: String,
    pub plan: Plan,
    pub payloads: HashMap<String, String>,
    pub records: Vec<ActionRecord>,
    pub approval: Option<Approval>,
    pub execution: Option<(usize, usize)>,
    pub newline: &'static str,
}

pub(super) fn valid_name(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

pub(super) fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(super) fn parse_timestamp(value: &str) -> Result<OffsetDateTime, String> {
    let invalid = || format!("Invalid UTC timestamp: {value}");
    let utc = value.strip_suffix('Z').ok_or_else(invalid)?;
    if value.as_bytes().get(10) != Some(&b'T') {
        return Err(invalid());
    }
    // Zod accepts UTC datetimes at minute precision as well as with seconds.
    if utc.len() == 16 {
        return OffsetDateTime::parse(&format!("{utc}:00Z"), &Rfc3339).map_err(|_| invalid());
    }
    if utc.as_bytes().get(17..19) == Some(b"60") {
        return Err(invalid());
    }
    OffsetDateTime::parse(value, &Rfc3339).map_err(|_| invalid())
}

fn string_keys(mapping: &Mapping) -> bool {
    mapping.keys().all(Value::is_string)
}

fn validate_plan(plan: &Plan) -> Result<(), String> {
    if plan.schema != 1 || plan.items.is_empty() || !valid_name(&plan.campaign) {
        return Err("Invalid schema-1 campaign plan".into());
    }
    if plan
        .metrics
        .as_ref()
        .is_some_and(|metrics| !string_keys(metrics))
    {
        return Err("Campaign metrics require string keys".into());
    }
    let window = if let Some([start, end]) = &plan.window_utc {
        let (start, end) = (parse_timestamp(start)?, parse_timestamp(end)?);
        if start >= end {
            return Err("The campaign window must end after it starts".into());
        }
        Some(start..end)
    } else {
        None
    };
    let ids: HashSet<_> = plan.items.iter().map(|i| i.id.as_str()).collect();
    if ids.len() != plan.items.len() {
        return Err("Campaign item IDs must be unique".into());
    }
    for item in &plan.items {
        if !valid_name(&item.id)
            || item
                .content
                .as_deref()
                .is_some_and(|content| !valid_name(content))
            || item.target.is_empty()
            || item.account.as_ref().is_some_and(String::is_empty)
            || item.tag.as_ref().is_some_and(String::is_empty)
            || item.commit.as_deref().is_some_and(|v| !valid_hex(v, 40))
            || item.assets.as_ref().is_some_and(|assets| {
                assets
                    .iter()
                    .any(|asset| asset.path.is_empty() || asset.sha256.is_empty())
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
                    || steps.iter().collect::<HashSet<_>>().len() != steps.len()
                    || steps.iter().any(|s| {
                        !matches!(s.as_str(), "save_draft" | "publish" | "notify_followers")
                    })
            })
        {
            return Err(format!("Invalid campaign item: {}", item.id));
        }
        if let Some(at) = &item.at {
            let at = parse_timestamp(at)?;
            if window.as_ref().is_some_and(|window| !window.contains(&at)) {
                return Err("Every item at time must be inside the campaign window".into());
            }
        }
        if let Some(after) = &item.after
            && (!ids.contains(after.as_str()) || after == &item.id)
        {
            return Err(format!("Invalid dependency for {}", item.id));
        }
        if item.steps.is_some() && (item.content.is_none() || item.mode.is_some()) {
            return Err(format!("Invalid executable plan for {}", item.id));
        }
    }
    Ok(())
}
struct YamlParser(unsafe_libyaml::yaml_parser_t);

impl Drop for YamlParser {
    fn drop(&mut self) {
        // The guard is only created after successful initialization.
        unsafe { unsafe_libyaml::yaml_parser_delete(&mut self.0) };
    }
}

fn reject_yaml_references(input: &str) -> Result<(), String> {
    use unsafe_libyaml::*;
    let mut parser = MaybeUninit::<yaml_parser_t>::uninit();
    // SAFETY: libyaml initializes the parser before it is wrapped. The input
    // outlives the guard, and the parser stays in place after setting its input.
    let mut parser = unsafe {
        if yaml_parser_initialize(parser.as_mut_ptr()).fail {
            return Err("Could not initialize YAML parser".into());
        }
        YamlParser(parser.assume_init())
    };
    let parser = &raw mut parser.0;
    unsafe {
        yaml_parser_set_encoding(parser, YAML_UTF8_ENCODING);
        yaml_parser_set_input_string(parser, input.as_ptr(), input.len() as u64);
    }
    loop {
        let mut event = MaybeUninit::<yaml_event_t>::uninit();
        // SAFETY: successful parsing initializes the event. Its discriminator
        // selects the active union field, and we delete the event after inspection.
        let (kind, reference) = unsafe {
            if yaml_parser_parse(parser, event.as_mut_ptr()).fail {
                return Err("Invalid campaign YAML".into());
            }
            let mut event = event.assume_init();
            let kind = event.type_;
            let reference = match kind {
                YAML_ALIAS_EVENT => true,
                YAML_SCALAR_EVENT => !event.data.scalar.anchor.is_null(),
                YAML_SEQUENCE_START_EVENT => !event.data.sequence_start.anchor.is_null(),
                YAML_MAPPING_START_EVENT => !event.data.mapping_start.anchor.is_null(),
                _ => false,
            };
            yaml_event_delete(&mut event);
            (kind, reference)
        };
        if reference {
            return Err("YAML aliases and anchors are not supported".into());
        }
        if kind == YAML_STREAM_END_EVENT {
            return Ok(());
        }
    }
}

fn yaml<T: serde::de::DeserializeOwned>(input: &str) -> Result<T, String> {
    reject_yaml_references(input)?;
    serde_yaml::from_value(serde_yaml::from_str(input).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

impl Intent {
    fn is_valid(&self) -> bool {
        match self {
            Self::Release(intent) => {
                repo_syntax(&intent.repo)
                    && !intent.account.is_empty()
                    && !intent.tag.is_empty()
                    && !intent.title.is_empty()
                    && intent
                        .commit
                        .as_deref()
                        .is_none_or(|commit| valid_hex(commit, 40))
            }
            Self::Bluesky(intent) => {
                let valid_did = intent.did.strip_prefix("did:plc:").is_some_and(|id| {
                    !id.is_empty()
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b))
                }) || intent.did.strip_prefix("did:web:").is_some_and(|id| {
                    !id.is_empty()
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
                });
                let valid_rkey = intent
                    .rkey
                    .strip_prefix("kirzner-")
                    .is_some_and(|key| valid_hex(key, 64))
                    || (intent.rkey.len() == 13
                        && intent.rkey.as_bytes()[0] <= b'j'
                        && intent
                            .rkey
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b)));
                bluesky_handle(&intent.account)
                    && valid_did
                    && valid_rkey
                    && !intent.text.is_empty()
                    && url::Url::parse(&intent.pds).is_ok_and(|pds| {
                        pds.scheme() == "https" && pds.origin().ascii_serialization() == intent.pds
                    })
            }
            Self::Retired(intent) => {
                !intent.provider.is_empty()
                    && !intent.account.is_empty()
                    && !intent.text.is_empty()
                    && !intent.integration_id.is_empty()
                    && !intent.actor_id.is_empty()
                    && valid_digest(&intent.instance)
            }
        }
    }
}

fn validate_execution(entry: &ExecutionEntry) -> Result<(), String> {
    if let Some(approval) = &entry.approval {
        if approval.items.is_empty()
            || approval.hashes.is_empty()
            || !approval.items.iter().all(|item| valid_name(item))
            || !approval.hashes.iter().all(|hash| valid_digest(hash))
            || !valid_digest(&approval.digest)
        {
            return Err("Invalid campaign approval".into());
        }
        if let Some(expires) = &approval.expires_at {
            parse_timestamp(expires)?;
        }
    }
    for record in entry.record.iter().chain(entry.records.iter().flatten()) {
        if !valid_name(&record.item)
            || record
                .version
                .as_deref()
                .is_some_and(|version| !valid_digest(version))
            || record
                .request_key
                .as_deref()
                .is_some_and(|key| !valid_digest(key))
            || record.cid.as_ref().is_some_and(String::is_empty)
            || record
                .intent
                .as_ref()
                .is_some_and(|intent| !intent.is_valid())
            || record
                .url
                .as_ref()
                .is_some_and(|url| url::Url::parse(url).is_err())
        {
            return Err(format!("Invalid campaign action record: {}", record.item));
        }
        for timestamp in record.attempted_at.iter().chain(&record.observed_at) {
            parse_timestamp(timestamp)?;
        }
    }
    if entry
        .observations
        .iter()
        .flatten()
        .any(|observation| !string_keys(observation))
    {
        return Err("Campaign observations require string keys".into());
    }
    Ok(())
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
        if bare.trim_end_matches([' ', '\t']) == "---" {
            closing = Some((cursor, cursor + line.len()));
            break;
        }
        cursor += line.len();
    }
    let (front_close, body_start) = closing.ok_or("Unclosed campaign front matter")?;
    let front = &text[front_end..front_close];
    let plan: Plan = yaml(front)?;
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
        let trimmed = bare.trim_start_matches(' ');
        let t = if bare.len() - trimmed.len() <= 3 {
            trimmed
        } else {
            bare
        };
        let boundary = t.chars().next().and_then(|marker| {
            if !matches!(marker, '`' | '~') {
                return None;
            }
            let count = t.chars().take_while(|&ch| ch == marker).count();
            (count >= 3).then_some((marker, count, &t[count..]))
        });
        if let Some((ch, count, content, execution)) = fence {
            if boundary.is_some_and(|(marker, length, rest)| {
                marker == ch && length >= count && rest.trim().is_empty()
            }) {
                if execution {
                    let ent: ExecutionEntry = yaml(&text[content..start])?;
                    validate_execution(&ent)?;
                    if let Some(a) = ent.approval {
                        approval = Some(a);
                    }
                    if let Some(r) = ent.records {
                        records.extend(r);
                    }
                    if let Some(r) = ent.record {
                        records.push(r);
                    }
                }
                fence = None;
            }
            continue;
        }
        if let Some((marker, count, rest)) = boundary
            && (marker == '~' || !rest.contains('`'))
        {
            let exec = section.as_deref() == Some("Execution") && rest.trim() == "yaml";
            fence = Some((marker, count, offset, exec));
            continue;
        }
        let hashes = t.bytes().take_while(|&byte| byte == b'#').count();
        if matches!(hashes, 1 | 2)
            && t[hashes..].starts_with([' ', '\t'])
            && !t[hashes..].trim_matches([' ', '\t']).is_empty()
        {
            if let Some(s) = section.take()
                && let Some(r) = sections.get_mut(&s)
            {
                r.1 = start;
            }
            if hashes == 2 {
                let heading = t[hashes..].trim_matches([' ', '\t']);
                let without_hashes = heading.trim_end_matches('#');
                let name = if without_hashes.ends_with([' ', '\t']) {
                    without_hashes
                } else {
                    heading
                };
                let name = name.trim_matches([' ', '\t']).to_owned();
                if sections
                    .insert(name.clone(), (offset, text.len()))
                    .is_some()
                {
                    return Err(format!("Duplicate section: {name}"));
                }
                section = Some(name);
            }
        }
    }
    if fence.is_some() {
        return Err("Unclosed Markdown fence".into());
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
            let blank = |line: &str| line.trim_matches([' ', '\t']).is_empty();
            let mut lines = v
                .split('\n')
                .skip_while(|line| blank(line))
                .collect::<Vec<_>>();
            while lines.last().is_some_and(|line| blank(line)) {
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
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| valid_hex(hex, 64))
}
pub fn append_execution(c: &Campaign, entry: &ExecutionEntry) -> Result<String, String> {
    validate_execution(entry)?;
    let nl = c.newline;
    let value = serde_yaml::to_value(entry).map_err(|e| e.to_string())?;
    let mut serialized = String::new();
    write_yaml(&mut serialized, "", "", &value)?;
    let serialized = serialized.replace('\n', nl);
    let end = c.execution.map_or(c.text.len(), |r| r.1);
    let prefix = &c.text[..end];
    let suffix = &c.text[end..];
    let sep = if prefix.ends_with(&format!("{nl}{nl}")) {
        ""
    } else if prefix.ends_with(nl) {
        nl
    } else if nl == "\r\n" {
        "\r\n\r\n"
    } else {
        "\n\n"
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

fn write_yaml(out: &mut String, indent: &str, field: &str, value: &Value) -> Result<(), String> {
    if let Value::Tagged(tagged) = value {
        let field = if field.is_empty() {
            tagged.tag.to_string()
        } else {
            format!("{field} {}", tagged.tag)
        };
        return write_yaml(out, indent, &field, &tagged.value);
    }
    if !field.is_empty() {
        out.push_str(indent);
        out.push_str(field);
    }
    let child = if field.is_empty() {
        indent.to_owned()
    } else {
        format!("{indent}  ")
    };
    match value {
        Value::Mapping(mapping) if !mapping.is_empty() => {
            if !field.is_empty() {
                out.push('\n');
            }
            for (key, value) in mapping {
                if matches!(key, Value::String(_)) {
                    let key = yaml_scalar(key)?;
                    write_yaml(out, &child, &format!("{key}:"), value)?;
                } else {
                    write_yaml(out, &child, "?", key)?;
                    write_yaml(out, &child, ":", value)?;
                }
            }
        }
        Value::Sequence(sequence) if !sequence.is_empty() => {
            if !field.is_empty() {
                out.push('\n');
            }
            for value in sequence {
                write_yaml(out, &child, "-", value)?;
            }
        }
        _ => {
            if field.is_empty() {
                out.push_str(indent);
            } else {
                out.push(' ');
            }
            out.push_str(&yaml_scalar(value)?);
            out.push('\n');
        }
    }
    Ok(())
}

fn yaml_scalar(value: &Value) -> Result<String, String> {
    let scalar = serde_yaml::to_string(value).map_err(|e| e.to_string())?;
    let scalar = scalar.trim_end_matches('\n');
    if let Some(string) = value.as_str()
        && (scalar.contains('\n') || scalar.starts_with(['|', '>']))
    {
        // Keep multiline strings inside the Markdown fence, even when they
        // contain a fence delimiter or are used as an explicit mapping key.
        return serde_json::to_string(string)
            .map(|encoded| {
                encoded
                    .replace('\u{85}', "\\u0085")
                    .replace('\u{2028}', "\\u2028")
                    .replace('\u{2029}', "\\u2029")
            })
            .map_err(|e| e.to_string());
    }
    Ok(scalar.into())
}
