use super::{
    append_execution, approval_batch, approve_campaign,
    format::{Campaign, ExecutionEntry, parse_campaign, parse_timestamp, valid_name},
};
use serde_json::json;
use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use time::OffsetDateTime;

fn campaign_name(name: &str) -> Result<(), String> {
    if !name.strip_suffix(".md").is_some_and(valid_name) {
        return Err("Use a Markdown filename in the workspace, such as campaign.md".into());
    }
    Ok(())
}

fn file_metadata(path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("The campaign must be a regular file".into());
    }
    Ok(metadata)
}

fn read_campaign_file(path: &Path) -> Result<Campaign, String> {
    file_metadata(path)?;
    parse_campaign(&fs::read_to_string(path).map_err(|e| e.to_string())?)
}

pub fn read_campaign(root: &Path, name: &str) -> Result<Campaign, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    campaign_name(name)?;
    read_campaign_file(&root.join(name))
}

struct WorkspaceLock(PathBuf);

impl WorkspaceLock {
    fn acquire(path: PathBuf) -> Result<Self, String> {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(|e| {
            if e.kind() == ErrorKind::AlreadyExists {
                "Workspace is locked. See crash recovery in the README.".into()
            } else {
                e.to_string()
            }
        })?;
        Ok(Self(path))
    }
}

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
}

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub struct CampaignStore {
    root: PathBuf,
    name: String,
    campaign: Campaign,
    _lock: WorkspaceLock,
}

impl CampaignStore {
    pub fn campaign(&self) -> &Campaign {
        &self.campaign
    }

    pub fn paused(&self) -> Result<bool, String> {
        for marker in [".kirzner.pause".to_owned(), format!("{}.pause", self.name)] {
            match fs::symlink_metadata(self.root.join(marker)) {
                Ok(_) => return Ok(true),
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(false)
    }

    pub fn append(&mut self, entry: &ExecutionEntry) -> Result<(), String> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let next = append_execution(&self.campaign, entry)?;
        let parsed_next = parse_campaign(&next)?;
        let path = self.root.join(&self.name);
        let metadata = file_metadata(&path)?;
        let tmp = self.root.join(format!(
            ".kirzner-{}-{}-{}.tmp",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let mode = metadata.permissions().mode() & 0o777;
            options.mode(mode);
            mode
        };
        #[cfg(not(unix))]
        let _ = metadata;
        let mut file = options.open(&tmp).map_err(|e| e.to_string())?;
        let temp = TempFile(tmp);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Creation honors umask; keep the original mode on the replacement.
            file.set_permissions(fs::Permissions::from_mode(mode))
                .map_err(|e| e.to_string())?;
        }
        file.write_all(next.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);

        let directory = fs::File::open(&self.root).map_err(|e| e.to_string())?;
        file_metadata(&path)?;
        if fs::read(&path).map_err(|e| e.to_string())? != self.campaign.text.as_bytes() {
            return Err("Campaign changed during execution".into());
        }
        fs::rename(&temp.0, &path).map_err(|e| e.to_string())?;
        let synced = directory.sync_all().map_err(|e| e.to_string());
        self.campaign = parsed_next;
        synced
    }
}

pub fn with_campaign<T>(
    root: &Path,
    name: &str,
    op: impl FnOnce(&mut CampaignStore) -> Result<T, String>,
) -> Result<T, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    campaign_name(name)?;
    let lock = WorkspaceLock::acquire(root.join(".kirzner.lock"))?;
    let campaign = read_campaign_file(&root.join(name))?;
    op(&mut CampaignStore {
        root,
        name: name.into(),
        campaign,
        _lock: lock,
    })
}

pub fn campaign_plan(root: &Path, name: &str) -> Result<serde_json::Value, String> {
    let campaign = read_campaign(root, name)?;
    let (actions, digest, _) = approval_batch(&campaign)?;
    let actions: Vec<_> = actions
        .iter()
        .map(|action| {
            let mut value = serde_json::to_value(action).map_err(|e| e.to_string())?;
            if let Some(fields) = value.as_object_mut() {
                fields.remove("kind");
            }
            Ok(value)
        })
        .collect::<Result<_, String>>()?;
    let mut out = json!({"campaign":campaign.plan.campaign,"digest":digest,"actions":actions});
    if let Some(window) = &campaign.plan.window_utc {
        out["window_utc"] = json!(window);
    }
    Ok(out)
}

pub fn campaign_status(root: &Path, name: &str) -> Result<serde_json::Value, String> {
    let campaign = read_campaign(root, name)?;
    let now = OffsetDateTime::now_utc();
    let next = campaign
        .plan
        .items
        .iter()
        .filter_map(|item| item.at.as_ref())
        .filter_map(|at| parse_timestamp(at).ok().map(|instant| (at, instant)))
        .filter(|(_, instant)| *instant > now)
        .min_by_key(|(_, instant)| *instant);
    let mut out = json!({
        "campaign":campaign.plan.campaign,
        "items":campaign.plan.items,
        "records":campaign.records
    });
    if let Some((at, _)) = next {
        out["next_due_at"] = json!(at);
    }
    if let Some(approval) = &campaign.approval {
        out["approval"] = json!(approval);
    }
    Ok(out)
}

pub fn campaign_approve(root: &Path, name: &str, digest: &str) -> Result<super::Approval, String> {
    with_campaign(root, name, |store| {
        let approval = approve_campaign(store.campaign(), digest)?;
        store.append(&ExecutionEntry {
            approval: Some(approval.clone()),
            ..Default::default()
        })?;
        Ok(approval)
    })
}
