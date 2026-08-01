//! 轻量沙箱：路径必须落在任务工作目录下（词法归一化 + 根目录 canonicalize）。
//! 不替代 OS 级容器隔离。

use anycode_core::prelude::*;
use std::path::{Component, Path, PathBuf};

fn lexical_normalize(path: PathBuf) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => {
                out = PathBuf::new();
                out.push(c.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

fn path_has_prefix(path: &Path, prefix: &Path) -> bool {
    let mut ip = path.components();
    let mut pp = prefix.components();
    loop {
        match (pp.next(), ip.next()) {
            (None, _) => return true,
            (Some(a), Some(b)) if a == b => continue,
            _ => return false,
        }
    }
}

const ANYCODE_WORKSPACE_MARKER: &str = "/.anycode/workspace";

/// Weak local models often pass the default `~/.anycode/workspace` instead of the task cwd.
/// When detected, remap to `.` or a relative suffix under the real workdir.
fn remap_anycode_workspace_hallucination(user_path: &str) -> Option<String> {
    let trimmed = user_path.trim();
    if trimmed.is_empty() || trimmed == "." {
        return None;
    }

    let expanded = if let Some(rest) = trimmed.strip_prefix("~/") {
        dirs::home_dir().map(|h| h.join(rest).to_string_lossy().to_string())?
    } else if trimmed == "~" {
        dirs::home_dir().map(|h| h.to_string_lossy().to_string())?
    } else {
        trimmed.to_string()
    };

    let normalized = expanded.replace('\\', "/");
    let idx = normalized.find(ANYCODE_WORKSPACE_MARKER)?;
    let suffix = normalized[idx + ANYCODE_WORKSPACE_MARKER.len()..]
        .trim_start_matches('/')
        .to_string();
    Some(if suffix.is_empty() {
        ".".to_string()
    } else {
        suffix
    })
}

/// 将用户给出的路径解析为绝对路径，并保证在 `workdir` 之下（沙箱写/读前调用）。
pub fn resolve_under_workdir(workdir: &str, user_path: &str) -> Result<PathBuf, CoreError> {
    resolve_under_workdir_with_extra_read_roots(workdir, user_path, &[])
}

/// Skill / catalog directories that read-only tools may access even when sandboxed.
pub fn skill_read_roots_for_workdir(workdir: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".anycode").join("skills"));
    }
    let wd = Path::new(workdir);
    let wd_abs = if wd.is_absolute() {
        wd.to_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(wd)
    } else {
        wd.to_path_buf()
    };
    roots.push(wd_abs.join("skills"));
    roots.push(wd_abs.join(".anycode").join("skills"));
    roots
}

/// Resolve a path under the task workdir, or under any extra read-only root (e.g. skills).
pub fn resolve_under_workdir_with_extra_read_roots(
    workdir: &str,
    user_path: &str,
    extra_roots: &[PathBuf],
) -> Result<PathBuf, CoreError> {
    let user_path =
        remap_anycode_workspace_hallucination(user_path).unwrap_or_else(|| user_path.to_string());
    let root = Path::new(workdir);
    let root_abs = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(CoreError::IoError)?
            .join(root)
    };
    let root_canon = root_abs.canonicalize().map_err(CoreError::IoError)?;

    let candidate = if Path::new(&user_path).is_absolute() {
        PathBuf::from(&user_path)
    } else {
        root_canon.join(&user_path)
    };
    let candidate_lex = lexical_normalize(candidate);

    if path_has_prefix(&candidate_lex, &root_canon) {
        return Ok(candidate_lex);
    }

    for extra in extra_roots {
        let extra_canon = if extra.is_absolute() {
            extra.canonicalize().ok()
        } else {
            root_canon.join(extra).canonicalize().ok()
        };
        if let Some(extra_canon) = extra_canon {
            if path_has_prefix(&candidate_lex, &extra_canon) {
                return Ok(candidate_lex);
            }
        }
    }

    Err(CoreError::PermissionDenied(format!(
        "path escapes sandbox (must be under {}): {:?}",
        root_canon.display(),
        candidate_lex
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn allows_file_inside_workdir() {
        let tmp = TempDir::new().unwrap();
        let w = tmp.path().to_str().unwrap();
        let p = tmp.path().join("a.txt");
        fs::write(&p, "x").unwrap();
        let r = resolve_under_workdir(w, "a.txt").unwrap();
        assert!(r.ends_with("a.txt"));
    }

    #[test]
    fn dot_resolves_to_workdir() {
        let tmp = TempDir::new().unwrap();
        let w = tmp.path().to_str().unwrap();
        let r = resolve_under_workdir(w, ".").unwrap();
        assert_eq!(r, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn remaps_default_anycode_workspace_to_workdir() {
        let tmp = TempDir::new().unwrap();
        let w = tmp.path().to_str().unwrap();
        let home = dirs::home_dir().expect("home");
        let hallucinated = home
            .join(".anycode")
            .join("workspace")
            .to_string_lossy()
            .to_string();
        let r = resolve_under_workdir(w, &hallucinated).unwrap();
        assert_eq!(r, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn remaps_anycode_workspace_file_to_relative_under_workdir() {
        let tmp = TempDir::new().unwrap();
        let w = tmp.path().to_str().unwrap();
        let home = dirs::home_dir().expect("home");
        let hallucinated = home
            .join(".anycode")
            .join("workspace")
            .join("demo.md")
            .to_string_lossy()
            .to_string();
        let r = resolve_under_workdir(w, &hallucinated).unwrap();
        assert_eq!(r.file_name().and_then(|s| s.to_str()), Some("demo.md"));
        assert_eq!(
            r.parent().and_then(|p| p.canonicalize().ok()),
            tmp.path().canonicalize().ok()
        );
    }

    #[test]
    fn remaps_tilde_anycode_workspace() {
        let tmp = TempDir::new().unwrap();
        let w = tmp.path().to_str().unwrap();
        let r = resolve_under_workdir(w, "~/.anycode/workspace").unwrap();
        assert_eq!(r, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn rejects_escape_via_dotdot() {
        let parent = TempDir::new().unwrap();
        let work = parent.path().join("work");
        let other = parent.path().join("other");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("secret.txt"), "x").unwrap();
        let w = work.to_str().unwrap();
        assert!(resolve_under_workdir(w, "../other/secret.txt").is_err());
    }

    #[test]
    fn allows_skill_dir_via_extra_read_roots() {
        let parent = TempDir::new().unwrap();
        let work = parent.path().join("work");
        let skills = parent.path().join("skills");
        let skill = skills.join("anycode-ppt");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# skill").unwrap();
        let w = work.to_str().unwrap();
        let skill_path = skill.canonicalize().unwrap();
        let extra = vec![skills.canonicalize().unwrap()];
        let r =
            resolve_under_workdir_with_extra_read_roots(w, skill_path.to_str().unwrap(), &extra)
                .unwrap();
        assert_eq!(r, skill_path);
    }

    #[test]
    fn write_tools_still_reject_skill_dir_without_extra_roots() {
        let parent = TempDir::new().unwrap();
        let work = parent.path().join("work");
        let skills = parent.path().join("skills");
        let skill = skills.join("demo");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&skill).unwrap();
        let w = work.to_str().unwrap();
        let skill_path = skill.canonicalize().unwrap();
        assert!(resolve_under_workdir(w, skill_path.to_str().unwrap()).is_err());
    }
}
