//! Fallback artifact registration by scanning workspace file mtimes after task_end.

use crate::db::DashboardDb;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_SCAN_FILES: usize = 200;

const SKIP_DIR_NAMES: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".dart_tool",
    "build",
    "dist",
    ".venv",
    "__pycache__",
    ".anycode",
];

/// Register artifacts for files under `project_root` modified after `since` (best-effort).
pub async fn scan_and_register_artifacts(
    db: &DashboardDb,
    project_id: &str,
    session_id: &str,
    project_root: &Path,
    since: SystemTime,
) -> Result<usize> {
    let mut registered = 0usize;
    let mut seen = 0usize;
    for entry in walk_files(project_root)? {
        if seen >= MAX_SCAN_FILES {
            break;
        }
        seen += 1;
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        if mtime <= since {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(project_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        if should_skip_rel(&rel) {
            continue;
        }
        let title = entry
            .path()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&rel)
            .to_string();
        let abs = entry.path().display().to_string();
        let kind = artifact_kind_for_path(entry.path());
        if db
            .upsert_artifact_scanned(project_id, session_id, &abs, kind, &title)
            .await
            .is_ok()
        {
            registered += 1;
        }
    }
    Ok(registered)
}

fn walk_files(root: &Path) -> Result<Vec<walkdir::DirEntry>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if SKIP_DIR_NAMES.contains(&name.as_ref()) {
                continue;
            }
        }
        if entry.file_type().is_file() {
            out.push(entry);
            if out.len() >= MAX_SCAN_FILES * 4 {
                break;
            }
        }
    }
    Ok(out)
}

fn should_skip_rel(rel: &str) -> bool {
    if rel.starts_with('.') {
        return true;
    }
    let lower = rel.to_lowercase();
    lower.ends_with(".log")
        || lower.ends_with(".tmp")
        || lower.contains("/target/")
        || lower.contains("/node_modules/")
}

/// Parse session `started_at` (RFC3339 or SQLite `datetime`) into [`SystemTime`].
pub fn parse_session_started_at(started_at: &str) -> SystemTime {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(started_at) {
        return system_time_from_secs(dt.timestamp(), dt.timestamp_subsec_nanos());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(started_at, "%Y-%m-%d %H:%M:%S") {
        let secs = naive.and_utc().timestamp();
        return system_time_from_secs(secs, naive.and_utc().timestamp_subsec_nanos());
    }
    SystemTime::now()
}

fn system_time_from_secs(secs: i64, nanos: u32) -> SystemTime {
    if secs < 0 {
        return SystemTime::UNIX_EPOCH;
    }
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64) + Duration::from_nanos(nanos as u64)
}

fn artifact_kind_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
    {
        Some(ext) if ext == "ipynb" => "notebook",
        Some(ext) if matches!(ext.as_str(), "md" | "txt" | "pdf") => "report",
        _ => "file",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_hidden_and_build_dirs() {
        assert!(should_skip_rel(".env"));
        assert!(should_skip_rel("foo/target/bar.rs"));
    }
}
