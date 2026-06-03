//! Install skills from a local directory or shallow git clone into `~/.anycode/skills/<id>/`.

use super::SkillCatalog;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SkillInstallResult {
    pub id: String,
    pub dest: PathBuf,
}

/// Resolve bundled `skills-starter/` (repo dev) or `ANYCODE_SKILLS_STARTER`.
#[must_use]
pub fn resolve_skills_starter_dir() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("ANYCODE_SKILLS_STARTER") {
        let p = PathBuf::from(raw);
        if p.is_dir() {
            return Some(p);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills-starter");
    if manifest.is_dir() {
        return Some(manifest);
    }
    None
}

/// Install all skills under the starter pack into `dest_root` (typically `~/.anycode/skills`).
pub fn install_starter_skills(dest_root: &Path) -> anyhow::Result<Vec<SkillInstallResult>> {
    let starter = resolve_skills_starter_dir()
        .ok_or_else(|| anyhow::anyhow!("skills-starter directory not found"))?;
    fs::create_dir_all(dest_root)?;
    let mut installed = Vec::new();
    for ent in fs::read_dir(&starter)? {
        let ent = ent?;
        if !ent.file_type()?.is_dir() {
            continue;
        }
        let sub = ent.path();
        if !sub.join("SKILL.md").is_file() {
            continue;
        }
        let id = ent.file_name().to_string_lossy().to_string();
        if !super::SkillCatalog::is_valid_skill_id(&id) {
            continue;
        }
        let dest = dest_root.join(&id);
        copy_skill_tree(&sub, &dest)?;
        installed.push(SkillInstallResult { id, dest });
    }
    if installed.is_empty() {
        anyhow::bail!("no skills found under {}", starter.display());
    }
    Ok(installed)
}

/// Copy a skill directory or clone a git repo containing `SKILL.md` at repo root or single skill subdir.
pub fn install_skill(source: &str, dest_root: &Path) -> anyhow::Result<SkillInstallResult> {
    let source = source.trim();
    if source.is_empty() {
        anyhow::bail!("source must not be empty");
    }
    fs::create_dir_all(dest_root)?;
    if looks_like_git_url(source) {
        install_from_git(source, dest_root)
    } else if source.ends_with(".zip") {
        install_from_zip(Path::new(source), dest_root)
    } else {
        install_from_local(Path::new(source), dest_root)
    }
}

fn looks_like_git_url(s: &str) -> bool {
    s.starts_with("https://")
        || s.starts_with("git@")
        || s.starts_with("ssh://")
        || s.ends_with(".git")
}

fn install_from_local(src: &Path, dest_root: &Path) -> anyhow::Result<SkillInstallResult> {
    if !src.is_dir() {
        anyhow::bail!("not a directory: {}", src.display());
    }
    let skill_md = src.join("SKILL.md");
    if skill_md.is_file() {
        let id = src
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid source path"))?;
        if !SkillCatalog::is_valid_skill_id(id) {
            anyhow::bail!("invalid skill id {:?}", id);
        }
        let dest = dest_root.join(id);
        copy_skill_tree(src, &dest)?;
        return Ok(SkillInstallResult {
            id: id.to_string(),
            dest,
        });
    }
    // Directory of skills (copy each subdir with SKILL.md)
    let mut last: Option<SkillInstallResult> = None;
    for ent in fs::read_dir(src)? {
        let ent = ent?;
        if !ent.file_type()?.is_dir() {
            continue;
        }
        let sub = ent.path();
        if sub.join("SKILL.md").is_file() {
            let id = ent.file_name().to_string_lossy().to_string();
            if !SkillCatalog::is_valid_skill_id(&id) {
                continue;
            }
            let dest = dest_root.join(&id);
            copy_skill_tree(&sub, &dest)?;
            last = Some(SkillInstallResult { id, dest });
        }
    }
    last.ok_or_else(|| anyhow::anyhow!("no SKILL.md found under {}", src.display()))
}

fn install_from_zip(archive: &Path, dest_root: &Path) -> anyhow::Result<SkillInstallResult> {
    if !archive.is_file() {
        anyhow::bail!("not a file: {}", archive.display());
    }
    let tmp = tempfile::tempdir()?;
    let file = fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    zip.extract(tmp.path())?;
    install_from_local(tmp.path(), dest_root)
}

fn install_from_git(url: &str, dest_root: &Path) -> anyhow::Result<SkillInstallResult> {
    let tmp = tempfile::tempdir()?;
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url, tmp.path().to_str().unwrap()])
        .status()?;
    if !status.success() {
        anyhow::bail!("git clone failed for {url}");
    }
    install_from_local(tmp.path(), dest_root)
}

fn copy_skill_tree(src: &Path, dest: &Path) -> anyhow::Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest.parent().unwrap_or(dest))?;
    copy_dir_recursive(src, dest)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dest)?;
    for ent in fs::read_dir(src)? {
        let ent = ent?;
        let ty = ent.file_type()?;
        let from = ent.path();
        let to = dest.join(ent.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)?;
            #[cfg(unix)]
            if ent.file_name() == "run" {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&to)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&to, perms)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_local_skill_copies_skill_md() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("demo-skill");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("SKILL.md"),
            "---\nname: demo-skill\ndescription: test\n---\n",
        )
        .unwrap();
        let dest_root = tmp.path().join("skills");
        let r = install_from_local(&src, &dest_root).unwrap();
        assert_eq!(r.id, "demo-skill");
        assert!(r.dest.join("SKILL.md").is_file());
    }
}
