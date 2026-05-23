//! Scan local SKILL.md trees into SQLite (`skills` + `project_skills`).

use crate::db::DashboardDb;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ScannedSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub project_roots: Vec<String>,
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
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
    let mut out = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
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
            if id.is_empty() || !seen_ids.insert(id.clone()) {
                continue;
            }
            let (name, description) =
                parse_skill_md(&skill_md).unwrap_or((id.clone(), String::new()));
            let mut project_roots = Vec::new();
            for wp in workspace_paths {
                let wp_path = PathBuf::from(wp);
                if dir.starts_with(&wp_path) {
                    project_roots.push(wp.clone());
                }
            }
            out.push(ScannedSkill {
                id,
                name,
                description,
                source_path: dir.to_string_lossy().to_string(),
                project_roots,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn parse_skill_md(path: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut description = None;
    if text.starts_with("---") {
        if let Some(end) = text[3..].find("\n---") {
            let front = &text[3..3 + end];
            for line in front.lines() {
                let Some((k, v)) = line.split_once(':') else {
                    continue;
                };
                let key = k.trim();
                let val = v.trim().trim_matches('"');
                if key == "name" {
                    name = Some(val.to_string());
                } else if key == "description" {
                    description = Some(val.to_string());
                }
            }
        }
    }
    let fallback_name = path.parent()?.file_name()?.to_str()?.to_string();
    Some((
        name.unwrap_or(fallback_name),
        description.unwrap_or_default(),
    ))
}

pub async fn sync_skills_to_db(db: &DashboardDb, workspace_paths: &[String]) -> Result<usize> {
    let skills = discover_skills(workspace_paths);
    let mut n = 0usize;
    for s in &skills {
        db.upsert_skill(&s.id, &s.name, &s.description, &s.source_path)
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
    fn parses_skill_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("demo-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: Demo\ndescription: A test skill\n---\n\n# Demo\n",
        )
        .unwrap();
        let (name, desc) = parse_skill_md(&skill_dir.join("SKILL.md")).unwrap();
        assert_eq!(name, "Demo");
        assert_eq!(desc, "A test skill");
    }
}
