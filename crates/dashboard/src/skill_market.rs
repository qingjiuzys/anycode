//! Curated skill market entries (anyCode starter pack + official catalog metadata).

use crate::skill_meta::parse_frontmatter_text;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarketEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_zh: Option<String>,
    pub category: String,
    pub source: String,
    /// `anycode` | `official`
    pub badge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarketResponse {
    pub entries: Vec<SkillMarketEntry>,
}

/// Built-in market catalog (install via `POST /api/skills/import`).
#[must_use]
pub fn list_market_entries() -> SkillMarketResponse {
    let mut entries = official_catalog_entries();
    entries.extend(anycode_starter_entries());
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    SkillMarketResponse { entries }
}

/// Curated official skills (metadata-only; install source is a GitHub subpath or skills.sh slug).
fn official_catalog_entries() -> Vec<SkillMarketEntry> {
    vec![
        SkillMarketEntry {
            id: "web-research".into(),
            name: "Web research".into(),
            description: "Structured web search, source triage, and citation-ready summaries for agent tasks.".into(),
            description_zh: Some(
                "结构化网页检索、来源筛选与可引用摘要，适用于调研与事实核查类任务。".into(),
            ),
            category: "data".into(),
            source: "anthropics/skills:skills/web-research".into(),
            badge: "official".into(),
        },
        SkillMarketEntry {
            id: "code-review".into(),
            name: "Code review".into(),
            description: "Review diffs for correctness, security, and maintainability with actionable feedback.".into(),
            description_zh: Some(
                "审查代码变更的正确性、安全性与可维护性，输出可执行的改进建议。".into(),
            ),
            category: "quality".into(),
            source: "anthropics/skills:skills/code-review".into(),
            badge: "official".into(),
        },
        SkillMarketEntry {
            id: "git-workflow".into(),
            name: "Git workflow".into(),
            description: "Branch hygiene, commit messages, PR prep, and safe git operations for coding agents.".into(),
            description_zh: Some(
                "分支规范、提交信息、PR 准备与安全 git 操作，适合编码 Agent 日常协作。".into(),
            ),
            category: "quality".into(),
            source: "anthropics/skills:skills/git-workflow".into(),
            badge: "official".into(),
        },
        SkillMarketEntry {
            id: "office-docx".into(),
            name: "Office DOCX".into(),
            description: "Create and edit Word documents with structured sections, tables, and export-ready formatting.".into(),
            description_zh: Some(
                "创建与编辑 Word 文档，支持章节结构、表格与可导出排版。".into(),
            ),
            category: "business".into(),
            source: "anthropics/skills:skills/docx".into(),
            badge: "official".into(),
        },
        SkillMarketEntry {
            id: "readonly-db".into(),
            name: "Read-only DB".into(),
            description: "Inspect schemas and run read-only SQL against snapshots — no live destructive writes.".into(),
            description_zh: Some(
                "查看数据库 schema 并对快照执行只读 SQL，禁止对 live 库做破坏性写入。".into(),
            ),
            category: "data".into(),
            source: "anthropics/skills:skills/readonly-db".into(),
            badge: "official".into(),
        },
    ]
}

fn anycode_starter_entries() -> Vec<SkillMarketEntry> {
    let Some(starter) = anycode_tools::resolve_skills_starter_dir() else {
        return vec![];
    };
    let Ok(read_dir) = std::fs::read_dir(&starter) else {
        return vec![];
    };
    let mut out = Vec::new();
    for ent in read_dir.flatten() {
        if !ent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let skill_md = ent.path().join("SKILL.md");
        if !skill_md.is_file() {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&skill_md) else {
            continue;
        };
        let fm = parse_frontmatter_text(&raw);
        let id = ent.file_name().to_string_lossy().to_string();
        let source = ent.path().display().to_string();
        out.push(SkillMarketEntry {
            id: id.clone(),
            name: if fm.name.is_empty() { id } else { fm.name },
            description: fm.description,
            description_zh: fm.description_zh,
            category: fm.category,
            source,
            badge: "anycode".into(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_lists_official_and_anycode_entries() {
        let m = list_market_entries();
        assert!(m.entries.iter().any(|e| e.badge == "official"));
        assert!(m.entries.iter().any(|e| e.id == "web-research"));
        assert!(m.entries.iter().any(|e| e.id == "code-review"));
        assert!(m
            .entries
            .iter()
            .filter(|e| e.badge == "anycode")
            .next()
            .is_some());
    }

    #[test]
    fn official_entries_have_zh_descriptions() {
        for entry in official_catalog_entries() {
            assert_eq!(entry.badge, "official");
            assert!(entry.description_zh.as_ref().is_some_and(|s| !s.is_empty()));
        }
    }
}
