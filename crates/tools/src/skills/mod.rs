//! Skill discovery: `SKILL.md` frontmatter + multi-root scan (Agent Skills–style).

mod effective;
pub mod install;
pub mod vet;
pub use effective::SkillsGovernance;
pub use install::{
    install_skill, install_starter_skills, resolve_skills_starter_dir, SkillInstallResult,
};
pub use vet::{vet_skill_by_id, vet_skill_dir, SkillVetReport};

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

const SKILL_FILE: &str = "SKILL.md";

/// Max bytes returned when loading a documentation-only skill body via the Skill tool.
pub const MAX_SKILL_INSTRUCTION_BYTES: usize = 64 * 1024;

/// Max captured bytes per stdout/stderr stream for `Skill` tool results.
pub const MAX_SKILL_OUTPUT_BYTES: usize = 256 * 1024;

/// Parsed YAML frontmatter from `SKILL.md`.
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    description_zh: Option<String>,
    /// Grouping hint (e.g. office/docs/dev/data/other); passed through, not validated.
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    channel_capabilities: Vec<String>,
    #[serde(default)]
    approval: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub id: String,
    pub description: String,
    pub description_zh: Option<String>,
    /// Grouping hint (e.g. office/docs/dev/data/other).
    pub category: Option<String>,
    pub root_dir: PathBuf,
    pub has_run: bool,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub channel_capabilities: Vec<String>,
    pub approval: Option<String>,
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
                let Ok(ft) = ent.file_type() else {
                    continue;
                };
                if !ft.is_dir() {
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
                let skill_dir = ent.path();
                let md_path = skill_dir.join(SKILL_FILE);
                if !md_path.is_file() {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&md_path) else {
                    warn!(target: "anycode_tools", "skill: unreadable {}", md_path.display());
                    continue;
                };
                let Some(fm) = parse_skill_frontmatter(&text) else {
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
                        category: fm
                            .category
                            .map(|s| s.trim().to_string())
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

    /// Resolve install root: catalog first, then `<cwd>/skills/<id>`, then `<cwd>/.anycode/skills/<id>`.
    pub fn resolve_skill_root(&self, id: &str, task_cwd: Option<&Path>) -> Option<PathBuf> {
        if !Self::is_valid_skill_id(id) {
            return None;
        }
        if let Some(i) = self.by_id.get(id) {
            return Some(self.skills[*i].root_dir.clone());
        }
        let cwd = task_cwd?;
        for rel in [Path::new("skills"), Path::new(".anycode/skills")] {
            let dir = cwd.join(rel).join(id);
            let md = dir.join(SKILL_FILE);
            if md.is_file() {
                return fs::canonicalize(&dir).ok().or(Some(dir));
            }
        }
        None
    }
}

fn parse_skill_frontmatter(md: &str) -> Option<SkillFrontmatter> {
    let t = md.trim_start();
    let rest = t.strip_prefix("---")?.trim_start();
    let end = rest.find("\n---")?;
    let yaml = &rest[..end];
    serde_yaml::from_str::<SkillFrontmatter>(yaml).ok()
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

/// Truncate combined stdout+stderr style output for tool results.
pub fn truncate_skill_output(mut s: String, max: usize) -> String {
    if s.len() <= max {
        return s;
    }
    let mut t = s.drain(..max).collect::<String>();
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
}
