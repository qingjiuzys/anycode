//! Skill discovery: `SKILL.md` frontmatter + multi-root scan (Agent Skills–style).

mod effective;
pub mod install;
pub mod router;
pub mod vet;
pub use effective::SkillsGovernance;
pub use install::{
    ensure_office_starter_skills, install_skill, install_starter_skills,
    resolve_skills_starter_dir, SkillInstallResult, OFFICE_STARTER_SKILL_IDS,
};
pub use router::{
    resolve_capabilities, SelectedSkill, SkillMatchStatus, SkillResolution, SkillResolutionContext,
};
pub use vet::{vet_skill_by_id, vet_skill_dir, SkillVetReport};

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::warn;

const SKILL_FILE: &str = "SKILL.md";

/// Max bytes returned when loading a documentation-only skill body via the Skill tool.
pub const MAX_SKILL_INSTRUCTION_BYTES: usize = 64 * 1024;

/// Max captured bytes per stdout/stderr stream for `Skill` tool results.
pub const MAX_SKILL_OUTPUT_BYTES: usize = 256 * 1024;

/// Parsed YAML frontmatter from `SKILL.md`.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub description_zh: Option<String>,
    #[serde(default)]
    pub name_zh: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    /// Grouping hint (e.g. office/docs/dev/data/other); passed through, not validated.
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub channel_capabilities: Vec<String>,
    #[serde(default)]
    pub approval: Option<String>,
    #[serde(default)]
    pub permissions: Option<serde_json::Value>,
    /// Capabilities this skill provides (e.g. web.implement).
    #[serde(default)]
    pub provides_capabilities: Vec<String>,
    /// Higher wins when multiple skills provide the same capability.
    #[serde(default)]
    pub priority: Option<i32>,
    /// Empty = all platforms; otherwise e.g. darwin, linux.
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub id: String,
    pub description: String,
    pub description_zh: Option<String>,
    pub version: Option<String>,
    /// Grouping hint (e.g. office/docs/dev/data/other).
    pub category: Option<String>,
    pub root_dir: PathBuf,
    pub has_run: bool,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub channel_capabilities: Vec<String>,
    pub approval: Option<String>,
    pub permissions: Option<serde_json::Value>,
    pub provides_capabilities: Vec<String>,
    pub priority: i32,
    pub platforms: Vec<String>,
}

/// Snapshot of discovered skills (startup scan + optional cwd resolution at tool run).
#[derive(Debug, Clone)]
pub struct SkillCatalog {
    skills: Vec<SkillMeta>,
    by_id: HashMap<String, usize>,
    pub run_timeout_ms: u64,
    pub minimal_env: bool,
    /// Roots used for the last scan (low → high precedence when merging).
    pub roots_scanned: Vec<PathBuf>,
}

/// How long [`SkillCatalog::scan_cached`] results stay fresh.
const SCAN_CACHE_TTL: Duration = Duration::from_secs(30);

struct ScanCacheEntry {
    roots: Vec<PathBuf>,
    run_timeout_ms: u64,
    minimal_env: bool,
    at: Instant,
    catalog: SkillCatalog,
}

static SCAN_CACHE: Mutex<Option<ScanCacheEntry>> = Mutex::new(None);

impl SkillCatalog {
    pub fn empty() -> Self {
        Self {
            skills: Vec::new(),
            by_id: HashMap::new(),
            run_timeout_ms: 120_000,
            minimal_env: false,
            roots_scanned: Vec::new(),
        }
    }

    /// Skill id: letters, digits, `.`, `_`, `-` only.
    pub fn is_valid_skill_id(id: &str) -> bool {
        !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    }

    /// Merge order: iterate `roots` in order; **later** roots overwrite same `id` (user dir should come last).
    pub fn scan(
        roots: &[PathBuf],
        allowlist: Option<&[String]>,
        run_timeout_ms: u64,
        minimal_env: bool,
    ) -> Self {
        let allow: Option<std::collections::HashSet<&str>> = allowlist.map(|v| {
            v.iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        });

        let mut map: HashMap<String, SkillMeta> = HashMap::new();
        let mut roots_scanned = Vec::new();

        for root in roots {
            let root = root.clone();
            if !root.is_dir() {
                continue;
            }
            roots_scanned.push(root.clone());
            let Ok(entries) = fs::read_dir(&root) else {
                continue;
            };
            for ent in entries.flatten() {
                let skill_dir = ent.path();
                // Follow symlinks so project `skills/<id> -> …` installs are discovered.
                let Ok(meta) = fs::metadata(&skill_dir) else {
                    continue;
                };
                if !meta.is_dir() {
                    continue;
                }
                let id = ent.file_name().to_string_lossy().to_string();
                if !Self::is_valid_skill_id(&id) {
                    continue;
                }
                if let Some(ref a) = allow {
                    if !a.contains(id.as_str()) {
                        continue;
                    }
                }
                let md_path = skill_dir.join(SKILL_FILE);
                if !md_path.is_file() {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&md_path) else {
                    warn!(target: "anycode_tools", "skill: unreadable {}", md_path.display());
                    continue;
                };
                let Some(fm) = parse_skill_manifest_text(&text) else {
                    warn!(target: "anycode_tools", "skill: bad frontmatter {}", md_path.display());
                    continue;
                };
                let fm_name = fm.name.trim();
                if fm_name != id.as_str() {
                    warn!(
                        target: "anycode_tools",
                        "skill: directory `{}` != frontmatter name `{}`, skipped",
                        id,
                        fm_name
                    );
                    continue;
                }
                let runner = skill_dir.join("run");
                let has_run = runner.is_file();
                map.insert(
                    id.clone(),
                    SkillMeta {
                        id,
                        description: fm.description.trim().to_string(),
                        description_zh: fm
                            .description_zh
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty()),
                        version: fm
                            .version
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty()),
                        category: fm
                            .category
                            .map(|s| normalize_skill_category(&s))
                            .filter(|s| !s.is_empty()),
                        root_dir: skill_dir,
                        has_run,
                        model: fm
                            .model
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty()),
                        mode: fm
                            .mode
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty()),
                        channel_capabilities: fm
                            .channel_capabilities
                            .into_iter()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                        approval: fm
                            .approval
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty()),
                        permissions: fm.permissions,
                        provides_capabilities: fm
                            .provides_capabilities
                            .into_iter()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                        priority: fm.priority.unwrap_or(0),
                        platforms: fm
                            .platforms
                            .into_iter()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                    },
                );
            }
        }

        let mut ids: Vec<_> = map.keys().cloned().collect();
        ids.sort();
        let mut skills = Vec::new();
        let mut by_id = HashMap::new();
        for id in ids {
            let meta = map.remove(&id).unwrap();
            by_id.insert(meta.id.clone(), skills.len());
            skills.push(meta);
        }

        Self {
            skills,
            by_id,
            run_timeout_ms,
            minimal_env,
            roots_scanned,
        }
    }

    /// Like [`Self::scan`], but shares the result across calls for a short TTL.
    ///
    /// Task triggers re-scan the catalog on every conversation turn; with ~20
    /// installed skills that is 20+ `SKILL.md` reads and YAML parses per
    /// message. Caching for [`SCAN_CACHE_TTL`] turns that into one scan per
    /// 30 s per process while still picking up newly installed skills
    /// promptly. Only applies when no allowlist filter is requested.
    pub fn scan_cached(roots: &[PathBuf], run_timeout_ms: u64, minimal_env: bool) -> Self {
        if let Ok(guard) = SCAN_CACHE.lock() {
            if let Some(entry) = guard.as_ref() {
                if entry.roots == roots
                    && entry.run_timeout_ms == run_timeout_ms
                    && entry.minimal_env == minimal_env
                    && entry.at.elapsed() < SCAN_CACHE_TTL
                {
                    return entry.catalog.clone();
                }
            }
        }
        let catalog = Self::scan(roots, None, run_timeout_ms, minimal_env);
        if let Ok(mut guard) = SCAN_CACHE.lock() {
            *guard = Some(ScanCacheEntry {
                roots: roots.to_vec(),
                run_timeout_ms,
                minimal_env,
                at: Instant::now(),
                catalog: catalog.clone(),
            });
        }
        catalog
    }

    /// Drop the result cached by [`Self::scan_cached`] (call after skill
    /// install/uninstall so subsequent turns see the change immediately).
    pub fn invalidate_scan_cache() {
        if let Ok(mut guard) = SCAN_CACHE.lock() {
            *guard = None;
        }
    }

    pub fn metas(&self) -> &[SkillMeta] {
        &self.skills
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Markdown block for system prompt (no leading `#` title — inserted under agent loop section).
    pub fn render_prompt_subsection(&self) -> Option<String> {
        self.render_prompt_subsection_allowlist(None)
    }

    /// Short contract when skills are enabled but not fully enumerated in the system prompt.
    pub fn render_prompt_skills_contract() -> String {
        [
            "## Skills",
            "",
            "Use **SkillSearch** to discover skills allowed for this project and agent profile.",
            "To load documentation or run a skill, call **Skill** with `{\"name\": \"<id>\"}` and optional `args` when the skill provides a `run` script.",
            "Do not assume a skill exists until SkillSearch or Skill confirms it.",
        ]
        .join("\n")
    }

    /// 若 `allow` 为 `Some`，仅列出 id 在该集合中的技能（用于按 agent 裁剪提示，避免全量目录灌入）。
    pub fn render_prompt_subsection_allowlist(&self, allow: Option<&[String]>) -> Option<String> {
        let allow_set: Option<std::collections::HashSet<&str>> = allow.map(|v| {
            v.iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        });
        let iter: Box<dyn Iterator<Item = &SkillMeta>> = if let Some(ref a) = allow_set {
            Box::new(self.skills.iter().filter(|s| a.contains(s.id.as_str())))
        } else {
            Box::new(self.skills.iter())
        };
        let filtered: Vec<&SkillMeta> = iter.collect();
        if filtered.is_empty() {
            return None;
        }
        let mut lines: Vec<String> = vec![
            "## Available skills".to_string(),
            String::new(),
            "These are loaded from your skill directories. For skills with a `run` script, call the **Skill** tool with `{\"name\": \"<id>\", \"args\": [...]}`. For documentation-only skills (no `run`), call **Skill** with `{\"name\": \"<id>\"}` to load the full `SKILL.md` instructions.".to_string(),
            String::new(),
        ];
        for s in filtered {
            let run_hint = if s.has_run {
                " — has `run`"
            } else {
                " — documentation only (no `run`)"
            };
            let mut hints: Vec<String> = Vec::new();
            if let Some(mode) = &s.mode {
                hints.push(format!("mode={mode}"));
            }
            if let Some(model) = &s.model {
                hints.push(format!("model={model}"));
            }
            if !s.channel_capabilities.is_empty() {
                hints.push(format!(
                    "channel_capabilities={}",
                    s.channel_capabilities.join("|")
                ));
            }
            if let Some(approval) = &s.approval {
                hints.push(format!("approval={approval}"));
            }
            let extra = if hints.is_empty() {
                String::new()
            } else {
                format!(" [{}]", hints.join(", "))
            };
            lines.push(format!(
                "- **{}**: {}{}{}",
                s.id, s.description, run_hint, extra
            ));
            if let Some(zh) = &s.description_zh {
                lines.push(format!("  - 中文：{zh}"));
            }
            if !s.has_run {
                if let Some(excerpt) = skill_doc_excerpt(&s.root_dir) {
                    lines.push(format!("  - preview: {excerpt}"));
                }
            }
        }
        Some(lines.join("\n"))
    }

    /// Resolve install root: project-local roots override the startup/global catalog.
    pub fn resolve_skill_root(&self, id: &str, task_cwd: Option<&Path>) -> Option<PathBuf> {
        if !Self::is_valid_skill_id(id) {
            return None;
        }
        if let Some(cwd) = task_cwd {
            for rel in [Path::new("skills"), Path::new(".anycode/skills")] {
                let dir = cwd.join(rel).join(id);
                let md = dir.join(SKILL_FILE);
                if md.is_file() {
                    return fs::canonicalize(&dir).ok().or(Some(dir));
                }
            }
        }
        self.by_id.get(id).map(|i| self.skills[*i].root_dir.clone())
    }
}

pub fn parse_skill_manifest_text(md: &str) -> Option<SkillManifest> {
    let t = md.trim_start();
    let rest = t.strip_prefix("---")?.trim_start();
    let end = rest.find("\n---")?;
    let yaml = &rest[..end];
    serde_yaml::from_str::<SkillManifest>(yaml).ok()
}

pub fn parse_skill_manifest_file(path: &Path) -> Option<SkillManifest> {
    let text = fs::read_to_string(path).ok()?;
    parse_skill_manifest_text(&text)
}

pub fn normalize_skill_category(raw: &str) -> String {
    let category = raw.trim().to_lowercase();
    match category.as_str() {
        "library-ref" | "verification" | "data" | "business" | "scaffolding" | "quality"
        | "cicd" | "runbook" | "infra" | "other" => category,
        "office" | "docs" => "business".into(),
        "dev" => "quality".into(),
        _ => "other".into(),
    }
}

/// Markdown body after YAML frontmatter (trimmed). Used for documentation-only skills.
pub fn extract_skill_body(md: &str) -> String {
    let t = md.trim_start();
    let Some(rest) = t.strip_prefix("---") else {
        return md.trim().to_string();
    };
    let Some(end) = rest.find("\n---") else {
        return md.trim().to_string();
    };
    let body = rest[end + 4..].trim_start_matches(['\r', '\n']);
    body.trim().to_string()
}

fn skill_doc_excerpt(root: &Path) -> Option<String> {
    let body = load_skill_instructions(root)?;
    let line = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))?;
    let mut s: String = line.chars().take(120).collect();
    if line.chars().count() > 120 {
        s.push('…');
    }
    Some(s)
}

/// Load `SKILL.md` instructions from a skill directory (body only, no frontmatter).
pub fn load_skill_instructions(root: &Path) -> Option<String> {
    let md_path = root.join(SKILL_FILE);
    let text = fs::read_to_string(&md_path).ok()?;
    let body = extract_skill_body(&text);
    if body.is_empty() {
        return None;
    }
    Some(truncate_skill_output(body, MAX_SKILL_INSTRUCTION_BYTES))
}

/// Build search roots: `extra_dirs` (low precedence) then `~/.anycode/skills` if present.
pub fn default_skill_roots(extra_dirs: &[PathBuf], home: Option<&Path>) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = extra_dirs.to_vec();
    if let Some(h) = home {
        let u = h.join(".anycode/skills");
        roots.push(u);
    }
    roots
}

/// Largest index `<= i` that lies on a UTF-8 char boundary (Rust 1.85-compatible
/// stand-in for `str::floor_char_boundary`).
pub fn floor_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    let mut b = i;
    while !s.is_char_boundary(b) {
        b -= 1;
    }
    b
}

/// Truncate combined stdout+stderr style output for tool results.
pub fn truncate_skill_output(mut s: String, max: usize) -> String {
    if s.len() <= max {
        return s;
    }
    let boundary = floor_char_boundary(&s, max);
    let mut t = s.drain(..boundary).collect::<String>();
    t.push_str("\n… [truncated]");
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_skill_body_strips_frontmatter() {
        let md = "---\nname: demo\ndescription: x\n---\n\n# Title\n\nDo the thing.\n";
        assert_eq!(extract_skill_body(md), "# Title\n\nDo the thing.");
    }

    #[test]
    fn parses_version_permissions_and_normalized_category() {
        let manifest = parse_skill_manifest_text(
            "---\nname: demo\ndescription: Demo\nversion: 1.2.0\ncategory: office\npermissions:\n  network: false\n---\n",
        )
        .unwrap();
        assert_eq!(manifest.version.as_deref(), Some("1.2.0"));
        assert_eq!(normalize_skill_category("office"), "business");
        assert_eq!(
            manifest.permissions.unwrap().get("network"),
            Some(&serde_json::Value::Bool(false))
        );
    }

    #[test]
    fn project_skill_overrides_catalog_skill() {
        let temp = tempfile::tempdir().unwrap();
        let global = temp.path().join("global");
        let project = temp.path().join("project");
        let global_skill = global.join("demo");
        let project_skill = project.join(".anycode/skills/demo");
        fs::create_dir_all(&global_skill).unwrap();
        fs::create_dir_all(&project_skill).unwrap();
        fs::write(
            global_skill.join(SKILL_FILE),
            "---\nname: demo\ndescription: global\n---\n",
        )
        .unwrap();
        fs::write(
            project_skill.join(SKILL_FILE),
            "---\nname: demo\ndescription: project\n---\n",
        )
        .unwrap();
        let catalog = SkillCatalog::scan(&[global], None, 1_000, true);
        assert_eq!(
            catalog.resolve_skill_root("demo", Some(&project)),
            fs::canonicalize(project_skill).ok()
        );
    }

    #[test]
    fn scan_follows_symlinked_skill_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real-skill");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(
            real.join(SKILL_FILE),
            "---\nname: link-skill\ndescription: via symlink\n---\nbody\n",
        )
        .unwrap();
        let named = dir.path().join("skills_root");
        std::fs::create_dir_all(&named).unwrap();
        let target = named.join("link-skill");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &target).unwrap();
        #[cfg(not(unix))]
        {
            fs::rename(&real, &target).unwrap();
        }
        let catalog = SkillCatalog::scan(&[named], None, 1_000, false);
        assert!(
            catalog.metas().iter().any(|s| s.id == "link-skill"),
            "expected symlink/dir skill to be discovered"
        );
    }
}
