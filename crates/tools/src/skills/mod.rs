//! Skill discovery: `SKILL.md` frontmatter + multi-root scan (Agent Skills–style).

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

const SKILL_FILE: &str = "SKILL.md";

/// Max captured bytes per stdout/stderr stream for `Skill` tool results.
pub const MAX_SKILL_OUTPUT_BYTES: usize = 256 * 1024;

/// Parsed YAML frontmatter from `SKILL.md`.
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
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
            && id.chars().all(|c| {
                c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'
            })
    }

    /// Merge order: iterate `roots` in order; **later** roots overwrite same `id` (user dir should come last).
    pub fn scan(
        roots: &[PathBuf],
        allowlist: Option<&[String]>,
        run_timeout_ms: u64,
        minimal_env: bool,
    ) -> Self {
        let allow: Option<std::collections::HashSet<&str>> =
            allowlist.map(|v| v.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect());

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
                        root_dir: skill_dir,
                        has_run,
                        model: fm.model.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                        mode: fm.mode.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                        channel_capabilities: fm
                            .channel_capabilities
                            .into_iter()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                        approval: fm.approval.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
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
        if self.skills.is_empty() {
            return None;
        }
        let mut lines: Vec<String> = vec![
            "## Available skills".to_string(),
            String::new(),
            "These are loaded from your skill directories. To execute a skill that ships a `run` script, call the **Skill** tool with `{\"name\": \"<id>\", \"args\": [...]}`.".to_string(),
            String::new(),
        ];
        for s in &self.skills {
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
