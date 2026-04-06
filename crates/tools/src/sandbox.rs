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

/// 将用户给出的路径解析为绝对路径，并保证在 `workdir` 之下（沙箱写/读前调用）。
pub fn resolve_under_workdir(workdir: &str, user_path: &str) -> Result<PathBuf, CoreError> {
    let root = Path::new(workdir);
    let root_abs = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(CoreError::IoError)?
            .join(root)
    };
    let root_canon = root_abs.canonicalize().map_err(CoreError::IoError)?;

    let candidate = if Path::new(user_path).is_absolute() {
        PathBuf::from(user_path)
    } else {
        root_canon.join(user_path)
    };
    let candidate_lex = lexical_normalize(candidate);
    if !path_has_prefix(&candidate_lex, &root_canon) {
        return Err(CoreError::PermissionDenied(format!(
            "path escapes sandbox (must be under {}): {:?}",
            root_canon.display(),
            candidate_lex
        )));
    }
    Ok(candidate_lex)
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
}
