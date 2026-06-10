use super::embedded;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplateManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub description: String,
    #[serde(default)]
    pub description_zh: Option<String>,
    #[serde(default = "default_default_dir")]
    pub default_dir: String,
    #[serde(default)]
    pub flutter: Option<FlutterTemplateMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlutterTemplateMeta {
    #[serde(default = "default_platforms")]
    pub platforms: Vec<String>,
    #[serde(default = "default_org")]
    pub default_org: String,
}

fn default_default_dir() -> String {
    "my_flutter_app".into()
}

fn default_platforms() -> Vec<String> {
    vec!["ios".into(), "android".into(), "web".into()]
}

fn default_org() -> String {
    "com.example.app".into()
}

fn dev_templates_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../project-templates")
}

fn bundled_cache_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode").join("project-templates"))
}

/// Resolve `project-templates/` (repo, env, or materialized bundle under `~/.anycode`).
#[must_use]
pub fn resolve_project_templates_root() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("ANYCODE_PROJECT_TEMPLATES") {
        let p = PathBuf::from(raw);
        if p.is_dir() {
            return Some(p);
        }
    }
    let dev = dev_templates_root();
    if dev.is_dir() {
        return Some(dev);
    }
    if !embedded::has_bundled_templates() {
        return None;
    }
    let cache = bundled_cache_root()?;
    if cache.join("flutter-app/manifest.json").is_file() {
        return Some(cache);
    }
    if embedded::materialize_to(&cache).is_ok() && cache.is_dir() {
        return Some(cache);
    }
    None
}

pub fn list_project_templates() -> Result<Vec<ProjectTemplateManifest>> {
    let root = resolve_project_templates_root()
        .ok_or_else(|| anyhow::anyhow!("project-templates directory not found"))?;
    let mut out = Vec::new();
    for ent in fs::read_dir(&root)? {
        let ent = ent?;
        if !ent.file_type()?.is_dir() {
            continue;
        }
        let manifest_path = ent.path().join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&manifest_path)
            .with_context(|| format!("read {}", manifest_path.display()))?;
        let m: ProjectTemplateManifest = serde_json::from_str(&raw)
            .with_context(|| format!("parse {}", manifest_path.display()))?;
        out.push(m);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn template_dir(template_id: &str) -> Result<PathBuf> {
    let root = resolve_project_templates_root()
        .ok_or_else(|| anyhow::anyhow!("project-templates directory not found"))?;
    let dir = root.join(template_id);
    if dir.join("manifest.json").is_file() {
        Ok(dir)
    } else {
        anyhow::bail!("unknown project template: {template_id}")
    }
}

pub fn load_manifest(template_id: &str) -> Result<ProjectTemplateManifest> {
    let dir = template_dir(template_id)?;
    let raw = fs::read_to_string(dir.join("manifest.json"))?;
    Ok(serde_json::from_str(&raw)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_flutter_app_template() {
        let list = list_project_templates().expect("templates");
        assert!(list.iter().any(|t| t.id == "flutter-app"));
    }

    #[test]
    fn bundled_templates_materialize() {
        assert!(embedded::has_bundled_templates());
        let dir = tempfile::tempdir().unwrap();
        embedded::materialize_to(dir.path()).expect("materialize");
        assert!(dir.path().join("flutter-app/manifest.json").is_file());
    }
}
