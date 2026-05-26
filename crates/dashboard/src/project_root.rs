//! Project workspace root validation and optional creation.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::Component;
use std::path::{Path, PathBuf};

pub fn ensure_project_root(root: &Path, create: bool) -> Result<PathBuf> {
    validate_root_path(root)?;
    if root.is_dir() {
        return std::fs::canonicalize(root)
            .with_context(|| format!("project root {}", root.display()));
    }
    if create {
        std::fs::create_dir_all(root)
            .with_context(|| format!("create project root {}", root.display()))?;
        return std::fs::canonicalize(root)
            .with_context(|| format!("project root {}", root.display()));
    }
    bail!(
        "project root does not exist: {}. Create the directory on disk first, or enable create_root when registering the project.",
        root.display()
    );
}

pub fn normalize_project_root(root: &Path) -> Result<PathBuf> {
    validate_root_path(root)?;
    if root.is_dir() {
        return std::fs::canonicalize(root)
            .with_context(|| format!("project root {}", root.display()));
    }
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .context("resolve current directory")?
            .join(root)
    };
    Ok(normalize_absolute_path(&absolute))
}

#[must_use]
pub fn project_id_for_root(root_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root_path.as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("proj_{}", &hex[..32])
}

/// Ensure a project root exists for web chat / conversation flows (auto-create when safe).
pub fn ensure_project_root_for_chat(root: &Path) -> Result<(PathBuf, bool)> {
    validate_root_path(root)?;
    if root.is_dir() {
        let resolved = std::fs::canonicalize(root)
            .with_context(|| format!("project root {}", root.display()))?;
        return Ok((resolved, false));
    }
    let resolved = ensure_project_root(root, true)?;
    Ok((resolved, true))
}

pub fn validate_root_path(root: &Path) -> Result<()> {
    if root.as_os_str().is_empty() {
        bail!("root_path is required");
    }
    if is_dangerous_root_path(root) {
        bail!(
            "refusing to use or create unsafe project root: {}",
            root.display()
        );
    }
    Ok(())
}

#[must_use]
pub fn is_dangerous_root_path(root: &Path) -> bool {
    if root.as_os_str().is_empty() {
        return true;
    }
    if root == Path::new("/") || root == Path::new("\\") {
        return true;
    }
    if let Ok(home) = std::env::var("HOME").map(PathBuf::from) {
        if root == home {
            return true;
        }
    }
    for blocked in [
        "/",
        "/bin",
        "/etc",
        "/sbin",
        "/usr",
        "/var",
        "/System",
        "/Library",
        "/Applications",
        "/private",
        "/dev",
        "/proc",
        "/sys",
        "C:\\",
        "C:\\Windows",
        "C:\\Program Files",
        "C:\\Program Files (x86)",
    ] {
        if root == Path::new(blocked) {
            return true;
        }
    }
    false
}

fn normalize_absolute_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_missing_root_without_create() {
        let err =
            ensure_project_root(Path::new("/nonexistent/anycode/project/root"), false).unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn creates_root_when_requested() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested/app");
        let resolved = ensure_project_root(&nested, true).unwrap();
        assert!(resolved.is_dir());
    }

    #[test]
    fn rejects_home_and_root_paths() {
        assert!(is_dangerous_root_path(Path::new("/")));
        if let Ok(home) = std::env::var("HOME") {
            assert!(is_dangerous_root_path(Path::new(&home)));
        }
    }

    #[test]
    fn chat_flow_creates_nested_root() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("chat/nested");
        let (resolved, created) = ensure_project_root_for_chat(&nested).unwrap();
        assert!(created);
        assert!(resolved.is_dir());
    }

    #[test]
    fn project_id_is_stable_hash_of_normalized_root() {
        let id = project_id_for_root("/tmp/demo");
        assert!(id.starts_with("proj_"));
        assert_eq!(id.len(), 37);
        assert_eq!(id, project_id_for_root("/tmp/demo"));
    }
}
