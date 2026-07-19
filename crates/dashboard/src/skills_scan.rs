//! Scan local SKILL.md trees into SQLite (`skills` + `project_skills`).

use crate::db::DashboardDb;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ScannedSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub description_zh: Option<String>,
    pub name_zh: Option<String>,
    pub version: String,
    pub category: Option<String>,
    pub permissions: Value,
    pub source_path: String,
    pub project_roots: Vec<String>,
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

pub fn count_skill_scan_roots(workspace_paths: &[String]) -> usize {
    skill_roots(workspace_paths)
        .into_iter()
        .filter(|p| p.is_dir())
        .count()
}

fn skill_roots(workspace_paths: &[String]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(h) = home_dir() {
        roots.push(h.join(".anycode/skills"));
    }
    for wp in workspace_paths {
        let p = PathBuf::from(wp);
        roots.push(p.join("skills"));
        roots.push(p.join(".anycode/skills"));
    }
    roots
}

/// Discover skill directories containing `SKILL.md`.
pub fn discover_skills(workspace_paths: &[String]) -> Vec<ScannedSkill> {
    let mut found: HashMap<String, ScannedSkill> = HashMap::new();
    for root in skill_roots(workspace_paths) {
        if !root.is_dir() {
            continue;
        }
        let Ok(read) = std::fs::read_dir(&root) else {
            continue;
        };
        for ent in read.flatten() {
            let dir = ent.path();
            if !dir.is_dir() {
                continue;
            }
            let skill_md = dir.join("SKILL.md");
            if !skill_md.is_file() {
                continue;
            }
            let id = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() || !anycode_tools::SkillCatalog::is_valid_skill_id(&id) {
                continue;
            }
            let Some(manifest) = anycode_tools::parse_skill_manifest_file(&skill_md) else {
                continue;
            };
            if manifest.name.trim() != id {
                continue;
            }
            let mut project_roots = Vec::new();
            for wp in workspace_paths {
                let wp_path = PathBuf::from(wp);
                if dir.starts_with(&wp_path) {
                    project_roots.push(wp.clone());
                }
            }
            found.insert(
                id.clone(),
                ScannedSkill {
                    id,
                    name: manifest.name,
                    description: manifest.description,
                    description_zh: manifest.description_zh,
                    name_zh: manifest.name_zh,
                    version: manifest.version.unwrap_or_default(),
                    category: Some(crate::skill_meta::normalize_category(
                        manifest.category.as_deref().unwrap_or("other"),
                    )),
                    permissions: manifest.permissions.unwrap_or_else(|| json!({})),
                    source_path: dir.to_string_lossy().to_string(),
                    project_roots,
                },
            );
        }
    }
    let mut out: Vec<_> = found.into_values().collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub async fn sync_skills_to_db(db: &DashboardDb, workspace_paths: &[String]) -> Result<usize> {
    let skills = discover_skills(workspace_paths);
    let mut n = 0usize;
    for s in &skills {
        db.upsert_skill(
            &s.id,
            &s.name,
            &s.description,
            s.description_zh.as_deref(),
            s.name_zh.as_deref(),
            &s.version,
            &s.source_path,
            s.category.as_deref(),
            &s.permissions,
        )
        .await?;
        n += 1;
        for root in &s.project_roots {
            if let Some(pid) = db.find_project_id_by_root(root).await? {
                db.link_project_skill(&pid, &s.id, true).await?;
            }
        }
        // Global ~/.anycode/skills apply to all projects when not under a workspace tree.
        if s.project_roots.is_empty() {
            for pid in db.list_project_ids().await? {
                db.link_project_skill(&pid, &s.id, true).await?;
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_project_dot_anycode_skills() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join(".anycode/skills/flutter-prd");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: flutter-prd\ndescription: Flutter PRD\n---\n",
        )
        .unwrap();
        let wp = dir.path().display().to_string();
        let found: Vec<_> = discover_skills(&[wp]).into_iter().map(|s| s.id).collect();
        assert!(found.contains(&"flutter-prd".to_string()));
    }

    #[test]
    fn parses_skill_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("demo-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: Demo\ndescription: A test skill\ndescription_zh: 测试\ncategory: office\n---\n\n# Demo\n",
        )
        .unwrap();
        let found = discover_skills(&[]);
        // Not in global skills dir — parse via skill_meta directly
        let fm = crate::skill_meta::parse_skill_md(&skill_dir.join("SKILL.md")).unwrap();
        assert_eq!(fm.name, "Demo");
        assert_eq!(fm.description, "A test skill");
        assert_eq!(fm.description_zh.as_deref(), Some("测试"));
        assert_eq!(fm.category, "business");
        let _ = found;
    }

    #[test]
    fn later_workspace_root_overrides_same_skill_id() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for (root, description) in [(&first, "first"), (&second, "second")] {
            let skill = root.path().join("skills/demo-skill");
            std::fs::create_dir_all(&skill).unwrap();
            std::fs::write(
                skill.join("SKILL.md"),
                format!("---\nname: demo-skill\ndescription: {description}\nversion: 1.0.0\n---\n"),
            )
            .unwrap();
        }
        let found = discover_skills(&[
            first.path().display().to_string(),
            second.path().display().to_string(),
        ]);
        let skill = found.iter().find(|skill| skill.id == "demo-skill").unwrap();
        assert_eq!(skill.description, "second");
        assert!(skill
            .source_path
            .starts_with(&second.path().display().to_string()));
    }
}
