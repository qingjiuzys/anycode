//! Shared SKILL.md frontmatter parsing and category normalization.

use std::path::Path;

/// Anthropic-style 9 categories plus fallback.
pub const SKILL_CATEGORIES: &[&str] = &[
    "library-ref",
    "verification",
    "data",
    "business",
    "scaffolding",
    "quality",
    "cicd",
    "runbook",
    "infra",
    "other",
];

#[derive(Debug, Clone, Default)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    pub description_zh: Option<String>,
    pub name_zh: Option<String>,
    pub category: String,
}

/// Normalize legacy or unknown category slugs to the canonical 9+1 set.
#[must_use]
pub fn normalize_category(raw: &str) -> String {
    anycode_tools::normalize_skill_category(raw)
}

pub fn parse_frontmatter_text(raw: &str) -> SkillFrontmatter {
    let Some(manifest) = anycode_tools::parse_skill_manifest_text(raw) else {
        return SkillFrontmatter {
            category: "other".into(),
            ..Default::default()
        };
    };
    SkillFrontmatter {
        name: manifest.name,
        description: manifest.description,
        description_zh: manifest.description_zh,
        name_zh: manifest.name_zh,
        category: normalize_category(manifest.category.as_deref().unwrap_or("other")),
    }
}

pub fn parse_skill_md(path: &Path) -> Option<SkillFrontmatter> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut fm = parse_frontmatter_text(&text);
    if fm.name.is_empty() {
        fm.name = path.parent()?.file_name()?.to_str()?.to_string();
    }
    Some(fm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_legacy_office_to_business() {
        assert_eq!(normalize_category("office"), "business");
        assert_eq!(normalize_category("docs"), "business");
        assert_eq!(normalize_category("dev"), "quality");
    }

    #[test]
    fn keeps_canonical_categories() {
        assert_eq!(normalize_category("library-ref"), "library-ref");
        assert_eq!(normalize_category("cicd"), "cicd");
    }

    #[test]
    fn parses_description_zh() {
        let raw =
            "---\nname: demo\ndescription: English\ndescription_zh: 中文\ncategory: data\n---\n";
        let fm = parse_frontmatter_text(raw);
        assert_eq!(fm.description_zh.as_deref(), Some("中文"));
        assert_eq!(fm.category, "data");
    }
}
