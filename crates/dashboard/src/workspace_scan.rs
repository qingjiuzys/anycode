//! Fallback artifact registration by scanning workspace file mtimes after task_end.

use crate::db::DashboardDb;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_SCAN_FILES: usize = 500;
const MAX_WALK_FILES: usize = 8000;

const SKIP_FILE_EXTENSIONS: &[&str] = &[
    "tsbuildinfo",
    "pyc",
    "log",
    "tmp",
    "o",
    "a",
    "rlib",
    "rmeta",
    "d",
];

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

#[derive(Clone)]
struct ScanCandidate {
    path: PathBuf,
    mtime: SystemTime,
    depth: usize,
    priority: i32,
}

/// Register artifacts for files under `project_root` modified after `since` (best-effort).
pub async fn scan_and_register_artifacts(
    db: &DashboardDb,
    project_id: &str,
    session_id: &str,
    project_root: &Path,
    since: SystemTime,
) -> Result<usize> {
    let mut registered = 0usize;
    let mut candidates: Vec<ScanCandidate> = Vec::new();
    let mut seen = HashSet::new();

    collect_root_deliverables(project_root, since, &mut candidates, &mut seen);
    for entry in walk_files(project_root)? {
        push_walk_candidate(&entry, project_root, since, &mut candidates, &mut seen);
    }

    candidates.sort_by(|a, b| {
        a.depth
            .cmp(&b.depth)
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| b.mtime.cmp(&a.mtime))
    });

    for candidate in candidates.into_iter().take(MAX_SCAN_FILES) {
        let rel = candidate
            .path
            .strip_prefix(project_root)
            .unwrap_or(&candidate.path)
            .to_string_lossy()
            .replace('\\', "/");
        let title = candidate
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&rel)
            .to_string();
        let abs = candidate.path.display().to_string();
        let kind = artifact_kind_for_path(&candidate.path);
        if db
            .upsert_artifact_scanned(project_id, session_id, &abs, kind, &title)
            .await
            .is_ok()
        {
            registered += 1;
        } else {
            tracing::debug!(path = %abs, session_id = %session_id, "artifact scan upsert skipped");
        }
    }
    Ok(registered)
}

fn collect_root_deliverables(
    project_root: &Path,
    since: SystemTime,
    out: &mut Vec<ScanCandidate>,
    seen: &mut HashSet<PathBuf>,
) {
    let read_dir = match std::fs::read_dir(project_root) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        push_walk_candidate(&path, project_root, since, out, seen);
    }
}

fn push_walk_candidate(
    path: &Path,
    project_root: &Path,
    since: SystemTime,
    out: &mut Vec<ScanCandidate>,
    seen: &mut HashSet<PathBuf>,
) {
    if !path.is_file() {
        return;
    }
    let canonical = path.to_path_buf();
    if !seen.insert(canonical.clone()) {
        return;
    }
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    let Ok(mtime) = meta.modified() else {
        return;
    };
    if mtime <= since {
        return;
    }
    let rel = path
        .strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if should_skip_rel(&rel) {
        return;
    }
    let priority = deliverable_priority(path, &rel);
    if priority == 0 {
        return;
    }
    let depth = rel.matches('/').count();
    out.push(ScanCandidate {
        path: canonical,
        mtime,
        depth,
        priority,
    });
}

fn deliverable_priority(path: &Path, rel: &str) -> i32 {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if ext.is_empty() {
        return 0;
    }
    if SKIP_FILE_EXTENSIONS.contains(&ext.as_str()) {
        return 0;
    }
    let lower = rel.to_lowercase();
    if lower.contains("/docs-src/")
        || lower.contains("/docs-site/")
        || lower.contains("/node_modules/")
        || lower.ends_with("/readme.md")
        || lower.ends_with("/changelog.md")
    {
        return 0;
    }
    if matches!(
        ext.as_str(),
        "pdf"
            | "pptx"
            | "ppt"
            | "docx"
            | "doc"
            | "xlsx"
            | "xls"
            | "ipynb"
            | "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "gif"
            | "mp4"
            | "mov"
            | "webm"
            | "csv"
    ) {
        return 100;
    }
    if ext == "md" || ext == "txt" {
        let depth = rel.matches('/').count();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if depth <= 2
            || name.contains("brief")
            || name.contains("report")
            || name.contains("outline")
            || name.contains("summary")
            || lower.contains("/output/")
            || lower.contains("/outputs/")
            || lower.contains("/reports/")
        {
            return 90;
        }
        return 0;
    }
    if ext == "rs" || ext == "ts" || ext == "tsx" || ext == "js" || ext == "py" {
        return 1;
    }
    0
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                return !SKIP_DIR_NAMES.contains(&name.as_ref());
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            out.push(entry.into_path());
            if out.len() >= MAX_WALK_FILES {
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
    let file_name = rel.rsplit('/').next().unwrap_or(rel);
    if file_name.starts_with("~$") {
        return true;
    }
    let lower = rel.to_lowercase();
    lower.ends_with(".log")
        || lower.ends_with(".tmp")
        || lower.ends_with(".pyc")
        || lower.contains("/target/")
        || lower.contains("/node_modules/")
        || lower.contains("/__pycache__/")
        || lower.contains("/build/")
        || lower.contains("/dist/")
        || lower.contains("/.git/")
        || lower.ends_with(".tsbuildinfo")
        || lower.ends_with(".fingerprint")
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
        Some(ext) if ext == "yml" || ext == "yaml" => {
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.starts_with("workflow"))
            {
                "workflow"
            } else {
                "file"
            }
        }
        Some(ext)
            if matches!(
                ext.as_str(),
                "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "webp"
                    | "svg"
                    | "bmp"
                    | "ico"
                    | "mp4"
                    | "mov"
                    | "avi"
                    | "webm"
                    | "mkv"
                    | "mp3"
                    | "wav"
                    | "ogg"
                    | "flac"
                    | "aac"
                    | "m4a"
            ) =>
        {
            "media"
        }
        Some(ext) if ext == "pdf" => "media",
        Some(ext) if matches!(ext.as_str(), "pptx" | "ppt") => "presentation",
        Some(ext) if matches!(ext.as_str(), "docx" | "doc" | "xlsx" | "xls") => "document",
        Some(ext)
            if matches!(ext.as_str(), "md" | "txt")
                && path.to_string_lossy().contains("report") =>
        {
            "report"
        }
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
        assert!(should_skip_rel("crates/foo.tsbuildinfo"));
    }

    #[test]
    fn deliverable_priority_prefers_pdf() {
        assert_eq!(
            deliverable_priority(Path::new("brief.pdf"), "brief.pdf"),
            100
        );
        assert_eq!(
            deliverable_priority(Path::new("foo.tsbuildinfo"), "foo.tsbuildinfo"),
            0
        );
        assert_eq!(
            deliverable_priority(
                Path::new("crates/account-portal/public/docs-src/en/guide/agents.md"),
                "crates/account-portal/public/docs-src/en/guide/agents.md",
            ),
            0
        );
        assert_eq!(deliverable_priority(Path::new("brief.md"), "brief.md"), 90);
    }
}
