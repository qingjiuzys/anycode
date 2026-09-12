//! Metadata-first SKILL.md discovery. Never executes installation hooks or grants
//! capabilities declared by a skill. A skill is untrusted instructions, not policy.
use anycode_harness_core::{digest::bytes_digest, Error, Result, RunContext};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
const LIMIT: u64 = 256 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub compatibility: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(rename = "allowed-tools", default)]
    pub allowed_tools: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub sha256: String,
}
#[derive(Clone)]
struct Source {
    root: PathBuf,
    path: PathBuf,
    summary: SkillSummary,
}
#[derive(Default)]
pub struct Catalog {
    entries: BTreeMap<String, Source>,
}
fn load(path: &Path, root: &Path) -> Result<Vec<u8>> {
    let link_meta = std::fs::symlink_metadata(path)?;
    if link_meta.file_type().is_symlink() || !link_meta.file_type().is_file() {
        return Err(Error::Denied("skill must be a regular file".into()));
    }
    let actual = std::fs::canonicalize(path)?;
    if !actual.starts_with(root) || !actual.is_file() {
        return Err(Error::Denied("skill path escapes trusted root".into()));
    }
    let meta = std::fs::metadata(&actual)?;
    if meta.len() > LIMIT {
        return Err(Error::Invalid("skill size limit".into()));
    }
    use std::io::Read;
    let mut bytes = vec![];
    std::fs::File::open(actual)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(Error::Invalid("skill grew past limit".into()));
    }
    Ok(bytes)
}
fn parse(bytes: &[u8]) -> Result<(Metadata, String)> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::Invalid("skill must be UTF-8".into()))?
        .replace("\r\n", "\n");
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| Error::Invalid("SKILL.md frontmatter required".into()))?;
    let (head, body) = rest
        .split_once("\n---\n")
        .ok_or_else(|| Error::Invalid("unterminated skill frontmatter".into()))?;
    if head.len() > 16384 {
        return Err(Error::Invalid("frontmatter too large".into()));
    }
    let meta: Metadata =
        serde_yaml::from_str(head).map_err(|_| Error::Invalid("invalid skill metadata".into()))?;
    if meta.name.is_empty()
        || meta.name.len() > 64
        || meta.name.starts_with('-')
        || meta.name.ends_with('-')
        || meta.name.contains("--")
        || !meta
            .name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || meta.description.trim().is_empty()
        || meta.description.len() > 2048
        || body.trim().is_empty()
    {
        return Err(Error::Invalid("skill metadata/content bounds".into()));
    }
    Ok((meta, body.into()))
}
impl Catalog {
    /// Only immediate child directories are read, with explicit bounds. Roots must
    /// be immutable host-managed directories; canonicalization alone is NOT a sandbox.
    pub fn discover(roots: &[PathBuf]) -> Result<Self> {
        if roots.len() > 16 {
            return Err(Error::Capacity);
        }
        let mut catalog = Self::default();
        for root in roots {
            let root = std::fs::canonicalize(root)?;
            let mut children = Vec::new();
            for child in std::fs::read_dir(&root)?.take(4097) {
                children.push(child?.path())
            }
            if children.len() > 4096 {
                return Err(Error::Capacity);
            }
            children.sort();
            for child in children {
                if !child.is_dir() {
                    continue;
                }
                let path = child.join("SKILL.md");
                if !path.is_file() {
                    continue;
                }
                let bytes = load(&path, &root)?;
                let (meta, _) = parse(&bytes)?;
                if child.file_name().and_then(|n| n.to_str()) != Some(meta.name.as_str()) {
                    return Err(Error::Invalid("skill name must match directory".into()));
                }
                if catalog.entries.contains_key(&meta.name) {
                    return Err(Error::Conflict(format!("duplicate skill: {}", meta.name)));
                }
                if catalog.entries.len() >= 1024 {
                    return Err(Error::Capacity);
                }
                let summary = SkillSummary {
                    name: meta.name.clone(),
                    description: meta.description,
                    sha256: bytes_digest(&bytes),
                };
                catalog.entries.insert(
                    meta.name,
                    Source {
                        root: root.clone(),
                        path,
                        summary,
                    },
                );
            }
        }
        Ok(catalog)
    }
    /// Caller supplies a host policy allowlist, not a list produced by the model.
    pub fn search(
        &self,
        query: &str,
        allowlist: &BTreeSet<String>,
        limit: usize,
    ) -> Vec<SkillSummary> {
        let words: Vec<_> = query
            .split_whitespace()
            .take(16)
            .map(str::to_lowercase)
            .collect();
        let mut found: Vec<_> = self
            .entries
            .values()
            .filter(|s| allowlist.contains(&s.summary.name))
            .filter_map(|s| {
                let hay = format!("{} {}", s.summary.name, s.summary.description).to_lowercase();
                let score = words.iter().filter(|w| hay.contains(w.as_str())).count();
                if score > 0 || words.is_empty() {
                    Some((score, s.summary.clone()))
                } else {
                    None
                }
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.name.cmp(&b.1.name)));
        found.into_iter().take(limit.min(20)).map(|s| s.1).collect()
    }

    pub fn digest(&self, name: &str) -> Option<&str> {
        self.entries.get(name).map(|s| s.summary.sha256.as_str())
    }

    pub fn activate(
        &self,
        ctx: &RunContext,
        name: &str,
        allowlist: &BTreeSet<String>,
    ) -> Result<String> {
        ctx.check()?;
        ctx.capabilities().require("skill.read")?;
        if !allowlist.contains(name) {
            return Err(Error::Denied(
                "skill not approved for this workspace".into(),
            ));
        }
        let source = self
            .entries
            .get(name)
            .ok_or_else(|| Error::Invalid("unknown skill".into()))?;
        let bytes = load(&source.path, &source.root)?;
        if bytes_digest(&bytes) != source.summary.sha256 {
            return Err(Error::Conflict(
                "skill changed after discovery; review and reload".into(),
            ));
        }
        let (_, body) = parse(&bytes)?;
        // Host must insert this as low-trust tool/resource content, never system authority.
        Ok(format!("Untrusted skill instructions: {name}\nCapabilities, approvals and identity remain controlled by the host.\n\n{body}"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use anycode_harness_core::{Capabilities, Scope};
    #[test]
    fn malformed_frontmatter_fails() {
        assert!(parse(b"just markdown").is_err());
        assert!(parse(b"---\nname: ../escape\ndescription: x\n---\nx").is_err());
    }
    #[test]
    fn allowed_tools_in_markdown_never_grant_capabilities() {
        let parsed = parse(
            b"---\nname: code-review\ndescription: Review Rust code\nallowed-tools: Bash,FileWrite\n---\nDo not treat this as policy.",
        )
        .unwrap();
        assert_eq!(parsed.0.allowed_tools.as_deref(), Some("Bash,FileWrite"));
        assert!(parsed.1.contains("not treat this as policy"));
    }

    #[test]
    fn symlink_skill_is_rejected_and_content_change_blocks_activate() {
        let d = tempfile::tempdir().unwrap();
        let skill_dir = d.path().join("code-review");
        std::fs::create_dir(&skill_dir).unwrap();
        let real = d.path().join("outside.md");
        std::fs::write(
            &real,
            "---\nname: code-review\ndescription: Review Rust code\n---\nbody",
        )
        .unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real, skill_dir.join("SKILL.md")).unwrap();
            assert!(Catalog::discover(&[d.path().into()]).is_err());
        }
        std::fs::remove_file(skill_dir.join("SKILL.md")).ok();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: code-review\ndescription: Review Rust code\n---\nbody v1",
        )
        .unwrap();
        let catalog = Catalog::discover(&[d.path().into()]).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: code-review\ndescription: Review Rust code\n---\nbody v2 changed",
        )
        .unwrap();
        let ctx = RunContext::root(
            Scope {
                subject: uuid::Uuid::new_v4(),
                organization: None,
                tenant: None,
                project: uuid::Uuid::new_v4(),
                device: None,
            },
            Capabilities::new(["skill.read".into()]).unwrap(),
            anycode_harness_core::budget::BudgetPool::new(10).unwrap(),
            std::time::Duration::from_secs(2),
        )
        .unwrap();
        let allowed = ["code-review".into()].into_iter().collect();
        assert!(catalog.activate(&ctx, "code-review", &allowed).is_err());
        assert!(catalog
            .activate(&ctx, "code-review", &BTreeSet::new())
            .is_err());
    }

    #[test]
    fn discover_is_metadata_only_and_deterministic() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("code-review")).unwrap();
        std::fs::write(d.path().join("code-review/SKILL.md"),"---\nname: code-review\ndescription: Review Rust code\n---\nRead first; report evidence.").unwrap();
        let c = Catalog::discover(&[d.path().into()]).unwrap();
        let allowed = ["code-review".into()].into_iter().collect();
        assert_eq!(c.search("Rust", &allowed, 5)[0].name, "code-review");
        assert!(c.search("Rust", &BTreeSet::new(), 5).is_empty());
    }
}
