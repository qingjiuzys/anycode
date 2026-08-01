//! Curated skill market entries (anyCode starter pack + official catalog metadata).

use crate::skill_meta::parse_frontmatter_text;
use anycode_tools::skills::install::ANYCODE_STARTER_SOURCE_PREFIX;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarketEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_zh: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_zh: Option<String>,
    pub category: String,
    pub source: String,
    /// `anycode` | `official`
    pub badge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarketResponse {
    pub entries: Vec<SkillMarketEntry>,
}

/// Built-in market catalog (install via `POST /api/skills/market/install`).
#[must_use]
pub fn list_market_entries() -> SkillMarketResponse {
    let mut entries = official_catalog_entries();
    entries.extend(anycode_starter_entries());
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    SkillMarketResponse { entries }
}

#[must_use]
pub fn find_market_entry(id: &str) -> Option<SkillMarketEntry> {
    let id = id.trim();
    if id.is_empty() {
        return None;
    }
    list_market_entries()
        .entries
        .into_iter()
        .find(|entry| entry.id == id)
}

pub fn install_market_entry(
    id: &str,
    dest_root: &Path,
) -> anyhow::Result<anycode_tools::SkillInstallResult> {
    let entry = find_market_entry(id)
        .ok_or_else(|| anyhow::anyhow!("skill store entry not found: {id}"))?;
    anycode_tools::install_skill(entry.source.trim(), dest_root)
}

/// Curated official skills from `anthropics/skills` (install via GitHub subpath).
fn official_catalog_entries() -> Vec<SkillMarketEntry> {
    vec![
        SkillMarketEntry {
            id: "pdf".into(),
            name: "PDF".into(),
            description: "Create, edit, and analyze PDF documents with structured workflows."
                .into(),
            description_zh: Some("创建、编辑与分析 PDF 文档的结构化工作流。".into()),
            name_zh: Some("PDF 文档".into()),
            category: "office".into(),
            source: "anthropics/skills:skills/pdf".into(),
            badge: "official".into(),
        },
        SkillMarketEntry {
            id: "docx".into(),
            name: "DOCX".into(),
            description: "Create and edit Word documents with structured sections, tables, and export-ready formatting.".into(),
            description_zh: Some(
                "创建与编辑 Word 文档，支持章节结构、表格与可导出排版。".into(),
            ),
            name_zh: Some("Word 文档".into()),
            category: "office".into(),
            source: "anthropics/skills:skills/docx".into(),
            badge: "official".into(),
        },
        SkillMarketEntry {
            id: "pptx".into(),
            name: "PPTX".into(),
            description: "Build and edit slide decks with consistent layout and speaker-ready structure.".into(),
            description_zh: Some("制作与编辑演示文稿，保持版式一致并便于演讲使用。".into()),
            name_zh: Some("PPT 演示".into()),
            category: "office".into(),
            source: "anthropics/skills:skills/pptx".into(),
            badge: "official".into(),
        },
        SkillMarketEntry {
            id: "xlsx".into(),
            name: "XLSX".into(),
            description: "Work with spreadsheets: tables, formulas, and analysis-ready exports.".into(),
            description_zh: Some("处理电子表格：表格、公式与分析导出。".into()),
            name_zh: Some("Excel 表格".into()),
            category: "office".into(),
            source: "anthropics/skills:skills/xlsx".into(),
            badge: "official".into(),
        },
        // frontend-design / webapp-testing ship in skills-starter (anycode badge) after bake-off.
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
        let source = format!("{ANYCODE_STARTER_SOURCE_PREFIX}{id}");
        out.push(SkillMarketEntry {
            id: id.clone(),
            name: if fm.name.is_empty() { id } else { fm.name },
            description: fm.description,
            description_zh: fm.description_zh,
            name_zh: fm.name_zh,
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
        assert!(m.entries.iter().any(|e| e.id == "pdf"));
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

    #[test]
    fn anycode_entries_use_starter_source_token() {
        let starter = anycode_starter_entries();
        if starter.is_empty() {
            return;
        }
        assert!(starter
            .iter()
            .all(|e| e.source.starts_with(ANYCODE_STARTER_SOURCE_PREFIX)));
        assert!(!starter[0].source.starts_with('/'));
    }

    #[test]
    fn find_market_entry_by_id() {
        let entry = find_market_entry("pdf").expect("pdf");
        assert_eq!(entry.badge, "official");
    }
}
