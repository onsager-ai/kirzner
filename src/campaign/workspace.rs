use super::{
    append_execution, approval_batch, approve_campaign,
    format::{Campaign, ExecutionEntry, parse_campaign},
};
use serde_json::json;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
fn name(n: &str) -> Result<(), String> {
    if n.len() < 4
        || !n.ends_with(".md")
        || !n[..n.len() - 3]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || !n.as_bytes()[0].is_ascii_alphanumeric()
    {
        return Err("Use a Markdown filename in the workspace, such as campaign.md".into());
    }
    Ok(())
}
fn path(root: &Path, n: &str) -> Result<PathBuf, String> {
    name(n)?;
    let p = root.join(n);
    let m = fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
    if !m.file_type().is_file() {
        return Err("The campaign must be a regular file".into());
    }
    Ok(p)
}
pub fn with_campaign<T>(
    root: &Path,
    n: &str,
    op: impl FnOnce(&mut Campaign) -> Result<T, String>,
) -> Result<T, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let lock = root.join(".kirzner.lock");
    fs::create_dir(&lock).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            "Workspace is locked. See crash recovery in the README.".into()
        } else {
            e.to_string()
        }
    })?;
    let result = (|| {
        let p = path(&root, n)?;
        let original = fs::read(&p).map_err(|e| e.to_string())?;
        let mut c = parse_campaign(std::str::from_utf8(&original).map_err(|e| e.to_string())?)?;
        let out = op(&mut c)?;
        if c.text.as_bytes() != original {
            return Err("Campaign changed during execution".into());
        }
        Ok(out)
    })();
    let _ = fs::remove_dir(&lock);
    result
}
fn append(root: &Path, n: &str, c: &Campaign, e: &ExecutionEntry) -> Result<(), String> {
    let p = path(root, n)?;
    let original = fs::read(&p).map_err(|e| e.to_string())?;
    let next = append_execution(c, e)?;
    let tmp = root.join(format!(
        ".kirzner-{}-{}.tmp",
        std::process::id(),
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| e.to_string())?;
    let write_result = (|| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(fs::Permissions::from_mode(
                fs::metadata(&p)
                    .map_err(|e| e.to_string())?
                    .permissions()
                    .mode()
                    & 0o777,
            ))
            .map_err(|e| e.to_string())?;
        }
        f.write_all(next.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        drop(f);
        if fs::read(&p).map_err(|e| e.to_string())? != original {
            return Err("Campaign changed during execution".into());
        }
        fs::rename(&tmp, &p).map_err(|e| e.to_string())?;
        fs::File::open(root)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_result?;
    Ok(())
}
pub fn campaign_plan(root: &Path, n: &str) -> Result<serde_json::Value, String> {
    with_campaign(root, n, |c| {
        let (a, d, _) = approval_batch(c)?;
        let acts: Vec<_> = a
            .iter()
            .map(|a| {
                let mut v = serde_json::to_value(a).map_err(|e| e.to_string())?;
                if let Some(o) = v.as_object_mut() {
                    o.remove("kind");
                    o.remove("request_key");
                    o.remove("remote_key");
                }
                Ok(v)
            })
            .collect::<Result<_, String>>()?;
        let mut out = json!({"campaign":c.plan.campaign,"digest":d,"actions":acts});
        if let Some(window) = &c.plan.window_utc {
            out["window_utc"] = json!(window);
        }
        Ok(out)
    })
}
pub fn campaign_status(root: &Path, n: &str) -> Result<serde_json::Value, String> {
    with_campaign(root, n, |c| {
        let next = c
            .plan
            .items
            .iter()
            .filter_map(|i| i.at.as_ref())
            .filter(|at| {
                time::OffsetDateTime::parse(at, &time::format_description::well_known::Rfc3339)
                    .is_ok_and(|t| t > time::OffsetDateTime::now_utc())
            })
            .min()
            .cloned();
        let mut out = json!({"campaign":c.plan.campaign,"items":c.plan.items,"records":c.records});
        if let Some(at) = next {
            out["next_due_at"] = json!(at);
        }
        if let Some(approval) = &c.approval {
            out["approval"] = json!(approval);
        }
        Ok(out)
    })
}
pub fn campaign_approve(root: &Path, n: &str, digest: &str) -> Result<super::Approval, String> {
    with_campaign(root, n, |c| {
        if root.join(".kirzner.pause").exists() || root.join(format!("{n}.pause")).exists() {
            return Err("Campaign is paused".into());
        }
        let a = approve_campaign(c, digest)?;
        append(
            root,
            n,
            c,
            &ExecutionEntry {
                approval: Some(a.clone()),
                ..Default::default()
            },
        )?;
        Ok(a)
    })
}
