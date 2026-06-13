//! Curated skill market entries (anyCode starter pack).

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
    /// `anycode` | `community`
    pub badge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarketResponse {
    pub entries: Vec<SkillMarketEntry>,
}

/// Built-in market catalog (install via `POST /api/skills/import`).
#[must_use]
pub fn list_market_entries() -> SkillMarketResponse {
    SkillMarketResponse {
        entries: anycode_starter_entries(),
    }
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
    fn market_lists_starter_entries() {
        let m = list_market_entries();
        assert!(!m.entries.is_empty());
        assert!(m.entries.iter().all(|e| e.badge == "anycode"));
    }
}
