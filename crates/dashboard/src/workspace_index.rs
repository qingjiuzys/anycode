//! Load workspace project paths from `~/.anycode/workspace/projects/index.json`
//! and optional discovery sources for dashboard scan.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

const MAX_SESSION_DISCOVERY: usize = 80;

/// `~/.anycode/workspace/projects/index.json`
#[must_use]
pub fn workspace_index_path() -> PathBuf {
    anycode_home()
        .join("workspace")
        .join("projects")
        .join("index.json")
}

/// Paths registered in the workspace index (resilient to trailing corrupt JSON).
#[must_use]
pub fn load_workspace_paths() -> Vec<String> {
    let index_path = workspace_index_path();
    let raw = match std::fs::read_to_string(&index_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    parse_index_paths(&raw).unwrap_or_default()
}

/// Merge index paths with recent REPL session `workspace_root` values.
#[must_use]
pub fn collect_scan_workspace_paths() -> Vec<String> {
    let mut paths = load_workspace_paths();
    let mut seen: HashSet<String> = paths.iter().cloned().collect();
    for path in discover_paths_from_sessions() {
        if seen.insert(path.clone()) {
            paths.push(path);
        }
    }
    paths
}

/// Recent session snapshots under `~/.anycode/sessions`.
#[must_use]
pub fn discover_paths_from_sessions() -> Vec<String> {
    let sessions_root = anycode_home().join("sessions");
    let Ok(read) = std::fs::read_dir(&sessions_root) else {
        return Vec::new();
    };

    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|ent| ent.path())
        .filter(|p| {
            p.extension().is_some_and(|ext| ext == "json")
                && p.file_name()
                    .is_some_and(|name| !name.to_string_lossy().starts_with('.'))
        })
        .collect();
    files.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    files.reverse();
    files.truncate(MAX_SESSION_DISCOVERY);

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for path in files {
        let Some(root) = read_session_workspace_root(&path) else {
            continue;
        };
        if !is_usable_project_root(&root) {
            continue;
        }
        if seen.insert(root.clone()) {
            out.push(root);
        }
    }
    out
}

fn anycode_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".anycode")
}

fn parse_index_paths(raw: &str) -> Option<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct Index {
        #[serde(default)]
        projects: Vec<Entry>,
    }
    #[derive(serde::Deserialize)]
    struct Entry {
        path: String,
    }

    if let Ok(idx) = serde_json::from_str::<Index>(raw) {
        return Some(idx.projects.into_iter().map(|p| p.path).collect());
    }

    let trimmed = raw.trim();
    for end in (1..=trimmed.len()).rev() {
        if let Ok(idx) = serde_json::from_str::<Index>(&trimmed[..end]) {
            return Some(idx.projects.into_iter().map(|p| p.path).collect());
        }
    }
    None
}

fn read_session_workspace_root(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let root = value.get("workspace_root")?.as_str()?.trim();
    if root.is_empty() {
        return None;
    }
    Some(root.to_string())
}

fn is_usable_project_root(path: &str) -> bool {
    if path.contains("/.tmp") || path.contains("/var/folders/") {
        return false;
    }
    let p = Path::new(path);
    if !p.is_dir() {
        return false;
    }
    p.join(".git").exists()
        || p.join("Cargo.toml").exists()
        || p.join("package.json").exists()
        || p.join(".anycode").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_index_tolerates_trailing_garbage() {
        let raw = r#"{
  "projects": [
    { "path": "/tmp/demo", "last_seen": "2026-05-24T00:00:00Z" }
  ]
}   }
  ]
}"#;
        let paths = parse_index_paths(raw).expect("paths");
        assert_eq!(paths, vec!["/tmp/demo".to_string()]);
    }

    #[test]
    fn collect_scan_deduplicates_index_and_sessions() {
        let paths = collect_scan_workspace_paths();
        let unique: HashSet<_> = paths.iter().collect();
        assert_eq!(unique.len(), paths.len());
    }
}
