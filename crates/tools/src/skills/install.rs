//! Install skills from a local directory or git source into `~/.anycode/skills/<id>/`.
//!
//! Git sources support the same shapes as the open `npx skills` CLI:
//! - `owner/repo`
//! - `owner/repo:path/to/skill`
//! - `https://github.com/owner/repo/tree/branch/path/to/skill`

use super::SkillCatalog;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SkillInstallResult {
    pub id: String,
    pub dest: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitSkillSource {
    repo_clone_url: String,
    subpath: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedSource {
    Local(PathBuf),
    Zip(PathBuf),
    Git(GitSkillSource),
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
        if !SkillCatalog::is_valid_skill_id(&id) {
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

/// Copy a skill directory or install from git (single skill or skill bundle layout).
pub fn install_skill(source: &str, dest_root: &Path) -> anyhow::Result<SkillInstallResult> {
    let source = source.trim();
    if source.is_empty() {
        anyhow::bail!("source must not be empty");
    }
    fs::create_dir_all(dest_root)?;
    match parse_skill_source(source)? {
        ParsedSource::Git(git) => install_from_git(&git, dest_root),
        ParsedSource::Zip(path) => install_from_zip(&path, dest_root),
        ParsedSource::Local(path) => install_from_local(&path, dest_root),
    }
}

fn parse_skill_source(source: &str) -> anyhow::Result<ParsedSource> {
    if source.ends_with(".zip") && Path::new(source).is_file() {
        return Ok(ParsedSource::Zip(PathBuf::from(source)));
    }
    if let Some(git) = parse_github_tree_url(source) {
        return Ok(ParsedSource::Git(git));
    }
    if let Some(git) = parse_github_shorthand(source) {
        return Ok(ParsedSource::Git(git));
    }
    if looks_like_git_url(source) {
        return Ok(ParsedSource::Git(GitSkillSource {
            repo_clone_url: normalize_clone_url(source),
            subpath: None,
        }));
    }
    Ok(ParsedSource::Local(PathBuf::from(source)))
}

/// `https://github.com/owner/repo/tree/branch/path/to/skill`
fn parse_github_tree_url(source: &str) -> Option<GitSkillSource> {
    let rest = source.strip_prefix("https://github.com/")?;
    let (repo_part, path_part) = rest.split_once("/tree/")?;
    let mut repo_segments = repo_part.split('/');
    let owner = repo_segments.next()?;
    let repo = repo_segments.next()?.trim_end_matches(".git");
    if repo_segments.next().is_some() {
        return None;
    }
    let mut path_segments = path_part.splitn(2, '/');
    let _branch = path_segments.next()?;
    let subpath = path_segments.next()?.trim_end_matches('/').to_string();
    if subpath.is_empty() {
        return None;
    }
    Some(GitSkillSource {
        repo_clone_url: format!("https://github.com/{owner}/{repo}.git"),
        subpath: Some(subpath),
    })
}

/// `owner/repo` or `owner/repo:skills/foo` (Agent Skills / skills.sh style).
fn parse_github_shorthand(source: &str) -> Option<GitSkillSource> {
    if source.contains("://")
        || source.contains(' ')
        || source.starts_with('/')
        || source.starts_with('.')
        || source.ends_with(".zip")
    {
        return None;
    }
    let (repo_part, subpath) = if let Some((left, right)) = source.split_once(':') {
        let sub = right.trim().trim_matches('/');
        if sub.is_empty() {
            return None;
        }
        (left, Some(sub.to_string()))
    } else {
        (source, None)
    };
    let mut segments = repo_part.split('/');
    let owner = segments.next()?;
    let repo = segments.next()?.trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() || segments.next().is_some() {
        return None;
    }
    Some(GitSkillSource {
        repo_clone_url: format!("https://github.com/{owner}/{repo}.git"),
        subpath,
    })
}

fn looks_like_git_url(s: &str) -> bool {
    s.starts_with("https://")
        || s.starts_with("git@")
        || s.starts_with("ssh://")
        || s.ends_with(".git")
}

fn normalize_clone_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if url.ends_with(".git") {
        url.to_string()
    } else if url.starts_with("https://github.com/") || url.starts_with("http://github.com/") {
        format!("{url}.git")
    } else {
        url.to_string()
    }
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

    let dirs = discover_skill_dirs(src);
    if dirs.is_empty() {
        anyhow::bail!("no SKILL.md found under {}", src.display());
    }
    let mut last: Option<SkillInstallResult> = None;
    for sub in dirs {
        let id = sub
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid skill directory"))?;
        if !SkillCatalog::is_valid_skill_id(id) {
            continue;
        }
        let dest = dest_root.join(id);
        copy_skill_tree(&sub, &dest)?;
        last = Some(SkillInstallResult {
            id: id.to_string(),
            dest,
        });
    }
    last.ok_or_else(|| anyhow::anyhow!("no valid skills found under {}", src.display()))
}

/// Find skill roots: direct children and `skills/*` (common catalog layout).
fn discover_skill_dirs(src: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_skill_dirs_in(src, &mut out);
    let skills_sub = src.join("skills");
    if skills_sub.is_dir() {
        collect_skill_dirs_in(&skills_sub, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect_skill_dirs_in(parent: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = fs::read_dir(parent) else {
        return;
    };
    for ent in read.flatten() {
        if !ent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let sub = ent.path();
        if sub.join("SKILL.md").is_file() {
            out.push(sub);
        }
    }
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

fn install_from_git(git: &GitSkillSource, dest_root: &Path) -> anyhow::Result<SkillInstallResult> {
    let tmp = tempfile::tempdir()?;
    let tmp_path = tmp.path();

    if let Some(sub) = &git.subpath {
        if try_sparse_clone(&git.repo_clone_url, tmp_path, sub).is_ok() {
            let skill_src = tmp_path.join(sub);
            if skill_src.join("SKILL.md").is_file() {
                return install_from_local(&skill_src, dest_root);
            }
        }
        git_clone_depth_1(&git.repo_clone_url, tmp_path)?;
        return install_from_local(&tmp_path.join(sub), dest_root);
    }

    git_clone_depth_1(&git.repo_clone_url, tmp_path)?;
    install_from_local(tmp_path, dest_root)
}

fn git_clone_depth_1(url: &str, dest: &Path) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url, dest.to_str().unwrap()])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("git clone failed for {url}");
    }
}

fn try_sparse_clone(url: &str, dest: &Path, subpath: &str) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            url,
            dest.to_str().unwrap(),
        ])
        .status()?;
    if !status.success() {
        anyhow::bail!("git sparse clone failed");
    }
    let status = Command::new("git")
        .args(["sparse-checkout", "set", subpath])
        .current_dir(dest)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("git sparse-checkout failed");
    }
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

    #[test]
    fn parses_github_tree_url() {
        let git = parse_github_tree_url(
            "https://github.com/vercel-labs/agent-skills/tree/main/skills/web-design-guidelines",
        )
        .unwrap();
        assert_eq!(
            git.repo_clone_url,
            "https://github.com/vercel-labs/agent-skills.git"
        );
        assert_eq!(git.subpath.as_deref(), Some("skills/web-design-guidelines"));
    }

    #[test]
    fn parses_owner_repo_colon_path() {
        let git = parse_github_shorthand("anthropics/skills:skills/pdf").unwrap();
        assert_eq!(
            git.repo_clone_url,
            "https://github.com/anthropics/skills.git"
        );
        assert_eq!(git.subpath.as_deref(), Some("skills/pdf"));
    }

    #[test]
    fn parses_owner_repo_shorthand() {
        let git = parse_github_shorthand("vercel-labs/agent-skills").unwrap();
        assert_eq!(
            git.repo_clone_url,
            "https://github.com/vercel-labs/agent-skills.git"
        );
        assert!(git.subpath.is_none());
    }

    #[test]
    fn discovers_skills_subdirectory_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let skill = tmp.path().join("skills").join("demo-skill");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: demo-skill\n---\n").unwrap();
        let found = discover_skill_dirs(tmp.path());
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("demo-skill"));
    }
}
