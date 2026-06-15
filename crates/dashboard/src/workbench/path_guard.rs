//! Resolve project-relative paths and reject escapes outside the workspace root.

use anyhow::{bail, Context, Result};
use std::path::{Component, Path, PathBuf};

const DEFAULT_SKIP_DIRS: &[&str] = &[".git", "node_modules", "target"];

/// Canonical project root; fails when missing or unsafe.
pub fn resolve_project_root(root_path: &str) -> Result<PathBuf> {
    let root = Path::new(root_path.trim());
    if root.as_os_str().is_empty() {
        bail!("project root_path is empty");
    }
    if !root.is_dir() {
        bail!("project root not found: {}", root.display());
    }
    crate::project_root::validate_root_path(root)?;
    std::fs::canonicalize(root).with_context(|| format!("canonicalize {}", root.display()))
}

/// Resolve `rel` under `root` and ensure the result stays inside `root`.
pub fn resolve_under_root(root: &Path, rel: &str) -> Result<PathBuf> {
    let root_canon = if root.is_dir() {
        std::fs::canonicalize(root).context("canonicalize project root")?
    } else {
        bail!("project root not found");
    };
    let rel = rel.trim().trim_start_matches(['/', '\\']);
    if rel.is_empty() {
        return Ok(root_canon);
    }
    let suffix = normalize_relative(rel)?;
    let joined = root_canon.join(&suffix);
    if joined.exists() {
        let canon = std::fs::canonicalize(&joined)
            .with_context(|| format!("resolve path {}", joined.display()))?;
        if !canon.starts_with(&root_canon) {
            bail!("path escapes project root");
        }
        return Ok(canon);
    }
    if !joined.starts_with(&root_canon) {
        bail!("path escapes project root");
    }
    Ok(joined)
}

fn normalize_relative(path: &str) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => out.push(part),
            Component::ParentDir => {
                if out.as_os_str().is_empty() {
                    bail!("path escapes project root");
                }
                out.pop();
            }
            _ => {}
        }
    }
    Ok(out)
}

#[must_use]
pub fn should_skip_entry(name: &str, is_dir: bool) -> bool {
    if !is_dir {
        return false;
    }
    DEFAULT_SKIP_DIRS.contains(&name)
}

#[must_use]
pub fn relative_path(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| abs.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_parent_escape() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("proj");
        std::fs::create_dir_all(&root).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let err = resolve_under_root(&root, "../outside").unwrap_err();
        assert!(err.to_string().contains("escapes") || err.to_string().contains("ancestor"));
    }

    #[test]
    fn resolves_nested_path() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("proj");
        let nested = root.join("src/lib");
        std::fs::create_dir_all(&nested).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let resolved = resolve_under_root(&root, "src/lib").unwrap();
        assert!(resolved.ends_with("src/lib"));
        assert!(resolved.starts_with(&root));
    }

    #[test]
    fn skip_dirs() {
        assert!(should_skip_entry("node_modules", true));
        assert!(!should_skip_entry("node_modules", false));
    }
}
