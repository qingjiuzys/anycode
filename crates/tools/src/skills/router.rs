//! Capability → skill resolution (pure function; no side effects).

use super::effective::SkillsGovernance;
use super::{load_skill_instructions, SkillCatalog, SkillMeta};

const MAX_PRELOAD: usize = 2;
const MAX_INSTRUCTION_BUDGET: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct SkillResolutionContext {
    pub agent_type: String,
    pub project_root: Option<std::path::PathBuf>,
    pub platform: String,
    pub production_skills_enabled: bool,
}

impl Default for SkillResolutionContext {
    fn default() -> Self {
        Self {
            agent_type: "general-purpose".into(),
            project_root: None,
            platform: std::env::consts::OS.to_string(),
            production_skills_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillMatchStatus {
    Selected,
    Unresolved,
    Denied,
    Disabled,
    DependencyUnavailable,
}

#[derive(Debug, Clone)]
pub struct SelectedSkill {
    pub capability: String,
    pub skill_id: String,
    pub status: SkillMatchStatus,
    pub instruction_excerpt: String,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillResolution {
    pub selected: Vec<SelectedSkill>,
    pub prompt_segment: String,
    pub denied_skill_ids: Vec<String>,
}

fn platform_ok(meta: &SkillMeta, platform: &str) -> bool {
    meta.platforms.is_empty()
        || meta
            .platforms
            .iter()
            .any(|p| p.eq_ignore_ascii_case(platform))
}

/// Resolve required capabilities to governed skills.
///
/// Sort key (unique): exact capability → governance allow → platform/deps →
/// priority desc → skill id asc.
pub fn resolve_capabilities(
    required: &[String],
    catalog: &SkillCatalog,
    governance: &SkillsGovernance,
    context: &SkillResolutionContext,
) -> SkillResolution {
    let mut out = SkillResolution::default();
    if !context.production_skills_enabled {
        for cap in required {
            out.selected.push(SelectedSkill {
                capability: cap.clone(),
                skill_id: String::new(),
                status: SkillMatchStatus::Disabled,
                instruction_excerpt: String::new(),
                candidates: vec![],
            });
        }
        // Deny all production skill ids so models cannot guess around the arm.
        out.denied_skill_ids = catalog.metas().iter().map(|m| m.id.clone()).collect();
        return out;
    }

    let mut used = 0usize;
    let mut budget = 0usize;
    for cap in required {
        let mut matches: Vec<&SkillMeta> = catalog
            .metas()
            .iter()
            .filter(|m| m.provides_capabilities.iter().any(|c| c == cap))
            .collect();
        matches.sort_by(|a, b| {
            let ga = governance.is_allowed(&context.agent_type, &a.id);
            let gb = governance.is_allowed(&context.agent_type, &b.id);
            gb.cmp(&ga)
                .then_with(|| {
                    platform_ok(b, &context.platform).cmp(&platform_ok(a, &context.platform))
                })
                .then_with(|| b.priority.cmp(&a.priority))
                .then_with(|| a.id.cmp(&b.id))
        });
        let candidates: Vec<String> = matches.iter().map(|m| m.id.clone()).collect();
        let Some(best) = matches.first().copied() else {
            out.selected.push(SelectedSkill {
                capability: cap.clone(),
                skill_id: String::new(),
                status: SkillMatchStatus::Unresolved,
                instruction_excerpt: String::new(),
                candidates,
            });
            continue;
        };
        if !governance.is_allowed(&context.agent_type, &best.id) {
            out.selected.push(SelectedSkill {
                capability: cap.clone(),
                skill_id: best.id.clone(),
                status: SkillMatchStatus::Denied,
                instruction_excerpt: String::new(),
                candidates,
            });
            out.denied_skill_ids.push(best.id.clone());
            continue;
        }
        if !platform_ok(best, &context.platform) {
            out.selected.push(SelectedSkill {
                capability: cap.clone(),
                skill_id: best.id.clone(),
                status: SkillMatchStatus::DependencyUnavailable,
                instruction_excerpt: String::new(),
                candidates,
            });
            continue;
        }
        if used >= MAX_PRELOAD || budget >= MAX_INSTRUCTION_BUDGET {
            out.selected.push(SelectedSkill {
                capability: cap.clone(),
                skill_id: best.id.clone(),
                status: SkillMatchStatus::Selected,
                instruction_excerpt: String::new(),
                candidates,
            });
            continue;
        }
        let root = catalog
            .resolve_skill_root(&best.id, context.project_root.as_deref())
            .unwrap_or_else(|| best.root_dir.clone());
        let mut excerpt = load_skill_instructions(&root).unwrap_or_default();
        let remaining = MAX_INSTRUCTION_BUDGET.saturating_sub(budget);
        if excerpt.len() > remaining {
            let boundary = crate::skills::floor_char_boundary(&excerpt, remaining);
            excerpt.truncate(boundary);
            excerpt.push_str("\n… [truncated]");
        }
        budget += excerpt.len();
        used += 1;
        out.selected.push(SelectedSkill {
            capability: cap.clone(),
            skill_id: best.id.clone(),
            status: SkillMatchStatus::Selected,
            instruction_excerpt: excerpt,
            candidates,
        });
    }

    let mut lines = Vec::new();
    for sel in &out.selected {
        if sel.status != SkillMatchStatus::Selected || sel.skill_id.is_empty() {
            continue;
        }
        if lines.is_empty() {
            lines.push("## Selected Skills".to_string());
            lines.push(
                "Use the Skill tool with these ids when executing the production SOP.".into(),
            );
        }
        lines.push(format!(
            "### skill `{}` for capability `{}`",
            sel.skill_id, sel.capability
        ));
        if !sel.instruction_excerpt.is_empty() {
            lines.push(sel.instruction_excerpt.clone());
        }
        lines.push(String::new());
    }
    out.prompt_segment = lines.join("\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::SkillCatalog;
    use std::fs;
    use std::path::PathBuf;

    fn write_skill(root: &std::path::Path, id: &str, caps: &str, priority: i32) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {id}\ndescription: test\nprovides_capabilities: [{caps}]\npriority: {priority}\n---\n\n# {id}\n\nDo work.\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn picks_higher_priority_then_id() {
        let temp = tempfile::tempdir().unwrap();
        write_skill(temp.path(), "web-a", "web.implement", 10);
        write_skill(temp.path(), "web-b", "web.implement", 50);
        let catalog = SkillCatalog::scan(&[temp.path().to_path_buf()], None, 120_000, false);
        let gov = SkillsGovernance::default();
        let ctx = SkillResolutionContext::default();
        let res = resolve_capabilities(&["web.implement".into()], &catalog, &gov, &ctx);
        assert_eq!(res.selected[0].skill_id, "web-b");
        assert_eq!(res.selected[0].status, SkillMatchStatus::Selected);
        assert!(!res.prompt_segment.is_empty());
    }

    #[test]
    fn office_pptx_prefers_commercial_delivery() {
        let temp = tempfile::tempdir().unwrap();
        write_skill(temp.path(), "office-pptx", "presentation.export.pptx", 50);
        write_skill(
            temp.path(),
            "presentation-design",
            "presentation.export.pptx",
            120,
        );
        write_skill(
            temp.path(),
            "presentation-commercial-delivery",
            "presentation.export.pptx",
            130,
        );
        let catalog = SkillCatalog::scan(&[temp.path().to_path_buf()], None, 120_000, false);
        let gov = SkillsGovernance::default();
        let ctx = SkillResolutionContext::default();
        let res = resolve_capabilities(&["presentation.export.pptx".into()], &catalog, &gov, &ctx);
        assert_eq!(res.selected[0].skill_id, "presentation-commercial-delivery");
    }

    #[test]
    fn office_pptx_author_prefers_design() {
        let temp = tempfile::tempdir().unwrap();
        write_skill(temp.path(), "office-pptx", "presentation.author", 50);
        write_skill(
            temp.path(),
            "presentation-design",
            "presentation.author",
            120,
        );
        let catalog = SkillCatalog::scan(&[temp.path().to_path_buf()], None, 120_000, false);
        let gov = SkillsGovernance::default();
        let ctx = SkillResolutionContext::default();
        let res = resolve_capabilities(&["presentation.author".into()], &catalog, &gov, &ctx);
        assert_eq!(res.selected[0].skill_id, "presentation-design");
    }

    #[test]
    fn disabled_arm_denies_all() {
        let temp = tempfile::tempdir().unwrap();
        write_skill(temp.path(), "web-a", "web.implement", 10);
        let catalog = SkillCatalog::scan(&[PathBuf::from(temp.path())], None, 120_000, false);
        let gov = SkillsGovernance::default();
        let mut ctx = SkillResolutionContext::default();
        ctx.production_skills_enabled = false;
        let res = resolve_capabilities(&["web.implement".into()], &catalog, &gov, &ctx);
        assert_eq!(res.selected[0].status, SkillMatchStatus::Disabled);
        assert!(res.denied_skill_ids.contains(&"web-a".into()));
    }
}
