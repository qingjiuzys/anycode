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
    pub category: String,
}

/// Normalize legacy or unknown category slugs to the canonical 9+1 set.
#[must_use]
pub fn normalize_category(raw: &str) -> String {
    let c = raw.trim().to_lowercase();
    if SKILL_CATEGORIES.contains(&c.as_str()) {
        return c;
    }
    match c.as_str() {
        "office" | "docs" => "business".into(),
        "dev" => "quality".into(),
        "data" => "data".into(),
        "other" | "" => "other".into(),
        _ => "other".into(),
    }
}

pub fn parse_frontmatter_text(raw: &str) -> SkillFrontmatter {
    let mut out = SkillFrontmatter {
        category: "other".into(),
        ..Default::default()
    };
    if !raw.starts_with("---") {
        return out;
    }
    let Some(end) = raw[3..].find("\n---") else {
        return out;
    };
    for line in raw[3..3 + end].lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim().trim_matches('"');
        match key {
            "name" => out.name = val.to_string(),
            "description" => out.description = val.to_string(),
            "description_zh" if !val.is_empty() => out.description_zh = Some(val.to_string()),
            "category" if !val.is_empty() => out.category = normalize_category(val),
            _ => {}
        }
    }
    out
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
