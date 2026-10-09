//! Thin connection to Semon's documented CLI. No harness parsing, log
//! normalization, direct log database reads, or forensic HTTP route exists here.
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const REFERENCE_REVISION: &str = "defe9d791a197902c5bebcba2e2b7fe87dd9a075";

#[derive(Clone)]
pub struct SemonConnection {
    pub codex_binary: PathBuf,
    pub store: PathBuf,
    pub cursor: PathBuf,
}

#[derive(Serialize)]
pub struct SemonStatus {
    pub capture_available: bool,
    pub store_present: bool,
    pub reference_revision: &'static str,
    pub limitation: &'static str,
}

impl SemonConnection {
    pub fn from_env(data: &Path) -> Self {
        Self {
            codex_binary: std::env::var_os("KIRZNER_SEMON_CODEX_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|| "semon-codex".into()),
            store: std::env::var_os("KIRZNER_SEMON_STORE")
                .map(PathBuf::from)
                .unwrap_or_else(|| data.join("semon/traces.sqlite3")),
            cursor: std::env::var_os("KIRZNER_SEMON_STATE")
                .map(PathBuf::from)
                .unwrap_or_else(|| data.join("semon/codex-cursor.json")),
        }
    }
    pub async fn status(&self) -> SemonStatus {
        let capture_available = if self.codex_binary.components().count() > 1 {
            self.codex_binary.is_file()
        } else {
            std::env::var_os("PATH").is_some_and(|p| {
                std::env::split_paths(&p).any(|d| d.join(&self.codex_binary).is_file())
            })
        };
        SemonStatus {
            capture_available,
            store_present: self.store.is_file(),
            reference_revision: REFERENCE_REVISION,
            limitation: "Store presence does not verify capture completeness. Explicit references are user-supplied; native viewer transcripts may be unavailable. Business records remain accessible. Raw forensic records are never served by Kirzner.",
        }
    }
    pub fn capture_codex(&self, sessions: &Path, history: &Path) -> std::io::Result<bool> {
        if !sessions.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Native Codex sessions directory is missing",
            ));
        }
        for path in [&self.store, &self.cursor] {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        // Paths are individual arguments, never shell code. The operator selects
        // the source on the CLI; browser requests cannot start capture processes.
        let status = Command::new(&self.codex_binary)
            .arg("--sessions")
            .arg(sessions)
            .arg("--history")
            .arg(history)
            .arg("--state")
            .arg(&self.cursor)
            .arg("--store")
            .arg(&self.store)
            .arg("--repo")
            .arg("kirzner")
            .arg("--verbose")
            .status()?;
        Ok(status.success())
    }
}
