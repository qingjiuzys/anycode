//! Unified asset center: aggregates artifacts, skills, and workflows.

use crate::asset_index::{add_artifact_link, get_artifact_detail};
use crate::db::DashboardDb;
use crate::schema::{
    ArtifactRecord, AssetActionRequest, AssetActionResult, AssetDetail, AssetItem,
    SkillDetailRecord, SkillRecord, WorkflowScanResult,
};
use anycode_tools::workflows::{default_workflow_candidates, load_workflow_from_file};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::Row;
use std::path::{Path, PathBuf};

const MEDIA_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "mp4", "mov", "avi", "webm", "mkv",
    "mp3", "wav", "ogg", "flac", "aac", "m4a", "pdf",
];

pub fn skill_asset_id(skill_id: &str) -> String {
    format!("skill_{skill_id}")
}

pub fn is_skill_asset_id(asset_id: &str) -> bool {
    asset_id.starts_with("skill_")
}

pub fn skill_id_from_asset_id(asset_id: &str) -> Option<&str> {
    asset_id.strip_prefix("skill_")
}

pub fn asset_kind_for_artifact(artifact: &ArtifactRecord) -> String {
    match artifact.kind.as_str() {
        "report" => "report".into(),
        "workflow" => "workflow".into(),
        "media" => "media".into(),
        "notebook" => "deliverable".into(),
        "file" => {
            if path_is_media(&artifact.path) {
                "media".into()
            } else {
                "deliverable".into()
            }
        }
        _ => "deliverable".into(),
    }
}

pub fn path_is_media(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| MEDIA_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn default_source_type_for_artifact(kind: &str, meta: &Value) -> String {
    if let Some(st) = meta.get("source_type").and_then(|v| v.as_str()) {
        if !st.is_empty() {
            return st.to_string();
        }
    }
    match kind {
        "report" => "report_archive".into(),
        "workflow" => "workflow_scan".into(),
        _ => "agent_created".into(),
    }
}

pub fn default_reuse_state(meta: &Value, is_final: bool) -> String {
    meta.get("reuse_state")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(if is_final { "candidate" } else { "candidate" })
        .to_string()
}

pub fn artifact_to_asset_item(
    artifact: &ArtifactRecord,
    meta: &Value,
    is_final: bool,
) -> AssetItem {
    let tags = meta
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    AssetItem {
        id: artifact.id.clone(),
        title: artifact.title.clone(),
        subtitle: artifact.path.clone(),
        asset_kind: asset_kind_for_artifact(artifact),
        backend_type: "artifact".into(),
        backend_id: artifact.id.clone(),
        project_id: artifact.project_id.clone(),
        project_name: artifact.project_name.clone(),
        session_id: artifact.session_id.clone(),
        trust_level: artifact.trust_level.clone(),
        source_type: if !is_final {
            meta.get("source_type")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("workspace_scan")
                .to_string()
        } else {
            default_source_type_for_artifact(&artifact.kind, meta)
        },
        reuse_state: default_reuse_state(meta, is_final),
        path: Some(artifact.path.clone()),
        category: None,
        note: meta
            .get("note")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        tags,
        updated_at: artifact.updated_at.clone(),
        verified_by_gate_name: artifact.verified_by_gate_name.clone(),
        session_trusted_status: artifact.session_trusted_status.clone(),
        skill_enabled: None,
    }
}

pub fn skill_to_asset_item(skill: &SkillRecord, project_id: Option<&str>) -> AssetItem {
    AssetItem {
        id: skill_asset_id(&skill.id),
        title: skill.name.clone(),
        subtitle: skill.source_path.clone(),
        asset_kind: "skill".into(),
        backend_type: "skill".into(),
        backend_id: skill.id.clone(),
        project_id: project_id.map(str::to_string),
        project_name: None,
        session_id: None,
        trust_level: if skill.enabled.unwrap_or(true) {
            "trusted".into()
        } else {
            "needs_verify".into()
        },
        source_type: "skill_scan".into(),
        reuse_state: "reusable".into(),
        path: Some(skill.source_path.clone()),
        category: skill.category.clone(),
        note: None,
        tags: Vec::new(),
        updated_at: None,
        verified_by_gate_name: None,
        session_trusted_status: None,
        skill_enabled: skill.enabled,
    }
}

pub async fn fetch_artifact_meta(db: &DashboardDb, artifact_id: &str) -> Result<(Value, bool)> {
    let row = sqlx::query("SELECT metadata_json, is_final FROM artifacts WHERE id = ? LIMIT 1")
        .bind(artifact_id)
        .fetch_optional(db.pool())
        .await?;
    let Some(r) = row else {
        return Ok((json!({}), true));
    };
    let raw: String = r.get("metadata_json");
    let is_final: i64 = r.get("is_final");
    let meta = serde_json::from_str(&raw).unwrap_or(json!({}));
    Ok((meta, is_final != 0))
}

pub async fn patch_artifact_metadata(
    db: &DashboardDb,
    artifact_id: &str,
    patch: Value,
) -> Result<Value> {
    let (mut meta, _) = fetch_artifact_meta(db, artifact_id).await?;
    if let (Some(existing), Some(patch_obj)) = (meta.as_object_mut(), patch.as_object()) {
        for (k, v) in patch_obj {
            existing.insert(k.clone(), v.clone());
        }
    } else {
        meta = patch;
    }
    sqlx::query(
        "UPDATE artifacts SET metadata_json = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(meta.to_string())
    .bind(artifact_id)
    .execute(db.pool())
    .await?;
    Ok(meta)
}

#[allow(clippy::too_many_arguments)]
pub async fn list_unified_assets(
    db: &DashboardDb,
    project_id: Option<&str>,
    session_id: Option<&str>,
    asset_kind: Option<&str>,
    source_type: Option<&str>,
    reuse_state: Option<&str>,
    trust_level: Option<&str>,
    unverified_only: bool,
    blocked_session_only: bool,
    final_only: bool,
    include_skills: bool,
    limit: i64,
) -> Result<Vec<AssetItem>> {
    let filter_kind = asset_kind.filter(|k| *k != "all" && !k.is_empty());
    let exclude_kind = if filter_kind == Some("deliverable") {
        Some("report")
    } else {
        None
    };
    let artifact_kind = match filter_kind {
        Some("deliverable") | Some("media") => None,
        Some("report") => Some("report"),
        Some("workflow") => Some("workflow"),
        Some("skill") => None,
        Some(other) => Some(other),
        None => None,
    };

    let artifacts = db
        .list_artifacts(
            project_id,
            session_id,
            artifact_kind,
            exclude_kind,
            trust_level,
            unverified_only,
            blocked_session_only,
            final_only,
            limit.max(200),
        )
        .await?;

    let mut items = Vec::new();
    for art in &artifacts {
        let (meta, is_final) = fetch_artifact_meta(db, &art.id).await?;
        let item = artifact_to_asset_item(art, &meta, is_final);
        if let Some(k) = filter_kind {
            if item.asset_kind != k {
                continue;
            }
        }
        if let Some(st) = source_type {
            if !st.is_empty() && item.source_type != st {
                continue;
            }
        }
        if let Some(rs) = reuse_state {
            if !rs.is_empty() && item.reuse_state != rs {
                continue;
            }
        }
        items.push(item);
    }

    if include_skills
        && filter_kind
            .map(|k| k == "skill" || k == "all")
            .unwrap_or(true)
    {
        let skills = if let Some(pid) = project_id {
            db.list_skills_for_project(pid).await?
        } else {
            db.list_skills(limit.max(200)).await?
        };
        for skill in skills {
            let item = skill_to_asset_item(&skill, project_id);
            if let Some(st) = source_type {
                if !st.is_empty() && item.source_type != st {
                    continue;
                }
            }
            if let Some(rs) = reuse_state {
                if !rs.is_empty() && item.reuse_state != rs {
                    continue;
                }
            }
            items.push(item);
        }
    }

    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    items.truncate(limit as usize);
    Ok(items)
}

pub async fn get_unified_asset_detail(
    db: &DashboardDb,
    asset_id: &str,
) -> Result<Option<AssetDetail>> {
    if let Some(skill_id) = skill_id_from_asset_id(asset_id) {
        let skill = get_skill_detail(db, skill_id).await?;
        let Some(skill) = skill else {
            return Ok(None);
        };
        let item = skill_to_asset_item_from_detail(&skill);
        return Ok(Some(AssetDetail {
            asset: item,
            artifact: None,
            skill: Some(skill),
            promotion_draft_path: None,
        }));
    }

    let Some(artifact_detail) = get_artifact_detail(db, asset_id).await? else {
        return Ok(None);
    };
    let (meta, is_final) = fetch_artifact_meta(db, asset_id).await?;
    let item = artifact_to_asset_item(&artifact_detail.artifact, &meta, is_final);
    let promotion_draft_path = meta
        .get("promotion_draft_path")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(Some(AssetDetail {
        asset: item,
        artifact: Some(artifact_detail),
        skill: None,
        promotion_draft_path,
    }))
}

fn skill_to_asset_item_from_detail(skill: &SkillDetailRecord) -> AssetItem {
    let enabled = skill.projects.iter().find(|p| p.enabled).map(|_| true);
    skill_to_asset_item(
        &SkillRecord {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            description_zh: skill.description_zh.clone(),
            source_path: skill.source_path.clone(),
            category: skill.category.clone(),
            projects_count: skill.projects_count,
            enabled,
        },
        None,
    )
}

async fn get_skill_detail(db: &DashboardDb, skill_id: &str) -> Result<Option<SkillDetailRecord>> {
    crate::governance::skills_governance::get_skill_detail(db, skill_id).await
}

pub async fn mark_asset_reusable(
    db: &DashboardDb,
    asset_id: &str,
    req: &AssetActionRequest,
) -> Result<AssetActionResult> {
    ensure_artifact_asset(asset_id)?;
    let patch = json!({
        "reuse_state": "reusable",
        "note": req.note,
        "tags": req.tags,
    });
    patch_artifact_metadata(db, asset_id, patch).await?;
    let detail = get_unified_asset_detail(db, asset_id)
        .await?
        .ok_or_else(|| anyhow!("asset not found"))?;
    Ok(AssetActionResult {
        ok: true,
        asset: detail.asset,
        draft_path: None,
    })
}

pub async fn archive_asset(
    db: &DashboardDb,
    asset_id: &str,
    req: &AssetActionRequest,
) -> Result<AssetActionResult> {
    ensure_artifact_asset(asset_id)?;
    let patch = json!({
        "reuse_state": "archived",
        "note": req.note,
        "tags": req.tags,
    });
    patch_artifact_metadata(db, asset_id, patch).await?;
    let detail = get_unified_asset_detail(db, asset_id)
        .await?
        .ok_or_else(|| anyhow!("asset not found"))?;
    Ok(AssetActionResult {
        ok: true,
        asset: detail.asset,
        draft_path: None,
    })
}

pub async fn promote_skill_draft(db: &DashboardDb, asset_id: &str) -> Result<AssetActionResult> {
    ensure_artifact_asset(asset_id)?;
    let detail = get_unified_asset_detail(db, asset_id)
        .await?
        .ok_or_else(|| anyhow!("asset not found"))?;
    let artifact = detail
        .artifact
        .as_ref()
        .ok_or_else(|| anyhow!("artifact detail missing"))?;
    let project_id = artifact
        .artifact
        .project_id
        .as_deref()
        .ok_or_else(|| anyhow!("project_id required"))?;
    let project = db
        .get_project(project_id)
        .await?
        .ok_or_else(|| anyhow!("project not found"))?;
    let root = PathBuf::from(&project.root_path);
    let slug = slugify(&artifact.artifact.title);
    let draft_dir = root.join(".anycode/drafts/skills").join(&slug);
    std::fs::create_dir_all(&draft_dir).context("create skill draft dir")?;
    let skill_md = draft_dir.join("SKILL.md");
    let body = read_artifact_body(
        &root,
        &artifact.artifact.path,
        artifact.report_markdown.as_deref(),
    );
    let content = format!(
        "---\nname: {slug}\ndescription: Promoted from asset `{}`\ncategory: other\n---\n\n{body}\n",
        artifact.artifact.path
    );
    std::fs::write(&skill_md, content).context("write skill draft")?;
    let draft_path = skill_md.to_string_lossy().to_string();
    patch_artifact_metadata(
        db,
        asset_id,
        json!({ "promotion_draft_path": draft_path, "reuse_state": "reusable" }),
    )
    .await?;
    let _ = add_artifact_link(db, asset_id, "skill_draft", None, Some(&draft_path)).await;
    let updated = get_unified_asset_detail(db, asset_id)
        .await?
        .ok_or_else(|| anyhow!("asset not found"))?;
    Ok(AssetActionResult {
        ok: true,
        asset: updated.asset,
        draft_path: Some(draft_path),
    })
}

pub async fn promote_workflow_draft(db: &DashboardDb, asset_id: &str) -> Result<AssetActionResult> {
    ensure_artifact_asset(asset_id)?;
    let detail = get_unified_asset_detail(db, asset_id)
        .await?
        .ok_or_else(|| anyhow!("asset not found"))?;
    let artifact = detail
        .artifact
        .as_ref()
        .ok_or_else(|| anyhow!("artifact detail missing"))?;
    let project_id = artifact
        .artifact
        .project_id
        .as_deref()
        .ok_or_else(|| anyhow!("project_id required"))?;
    let project = db
        .get_project(project_id)
        .await?
        .ok_or_else(|| anyhow!("project not found"))?;
    let root = PathBuf::from(&project.root_path);
    let slug = slugify(&artifact.artifact.title);
    let draft_dir = root.join(".anycode/drafts/workflows");
    std::fs::create_dir_all(&draft_dir).context("create workflow draft dir")?;
    let draft_path_buf = draft_dir.join(format!("{slug}.yml"));
    let body = read_artifact_body(
        &root,
        &artifact.artifact.path,
        artifact.report_markdown.as_deref(),
    );
    let prompt = body.lines().take(40).collect::<Vec<_>>().join("\n");
    let yaml = format!(
        "name: {slug}\ntrigger: manual\nsteps:\n  - id: step1\n    prompt: |\n{}\n",
        indent_block(&prompt, 6)
    );
    std::fs::write(&draft_path_buf, yaml).context("write workflow draft")?;
    let draft_path = draft_path_buf.to_string_lossy().to_string();
    patch_artifact_metadata(
        db,
        asset_id,
        json!({ "promotion_draft_path": draft_path, "reuse_state": "reusable" }),
    )
    .await?;
    let _ = add_artifact_link(db, asset_id, "workflow_draft", None, Some(&draft_path)).await;
    let updated = get_unified_asset_detail(db, asset_id)
        .await?
        .ok_or_else(|| anyhow!("asset not found"))?;
    Ok(AssetActionResult {
        ok: true,
        asset: updated.asset,
        draft_path: Some(draft_path),
    })
}

fn ensure_artifact_asset(asset_id: &str) -> Result<()> {
    if is_skill_asset_id(asset_id) {
        return Err(anyhow!(
            "skill assets cannot be modified via artifact actions"
        ));
    }
    Ok(())
}

fn read_artifact_body(root: &Path, path: &str, report_md: Option<&str>) -> String {
    if let Some(md) = report_md.filter(|s| !s.is_empty()) {
        return md.to_string();
    }
    let full = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        root.join(path)
    };
    std::fs::read_to_string(&full).unwrap_or_else(|_| format!("Asset at `{path}`"))
}

fn slugify(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "asset-draft".into()
    } else {
        trimmed.to_string()
    }
}

fn indent_block(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    text.lines()
        .map(|l| format!("{pad}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn scan_project_workflows(
    db: &DashboardDb,
    project_id: &str,
) -> Result<WorkflowScanResult> {
    let project = db
        .get_project(project_id)
        .await?
        .ok_or_else(|| anyhow!("project not found"))?;
    let root = PathBuf::from(&project.root_path);
    if !root.is_dir() {
        return Err(anyhow!("project root missing"));
    }
    let mut registered = 0usize;
    let mut paths = Vec::new();
    for candidate in default_workflow_candidates(&root) {
        if !candidate.is_file() {
            continue;
        }
        let workflow = load_workflow_from_file(&candidate)
            .with_context(|| format!("parse workflow {}", candidate.display()))?;
        let abs = candidate.to_string_lossy().to_string();
        let title = if workflow.name.is_empty() {
            candidate
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("workflow")
                .to_string()
        } else {
            workflow.name.clone()
        };
        let artifact_id = db
            .upsert_workflow_artifact(project_id, "", &abs, &title, workflow.steps.len())
            .await?;
        paths.push(abs);
        registered += 1;
        let _ = artifact_id;
    }
    Ok(WorkflowScanResult { registered, paths })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_media_by_extension() {
        let art = ArtifactRecord {
            id: "a".into(),
            path: "/tmp/demo.png".into(),
            kind: "file".into(),
            title: "demo".into(),
            trust_level: "needs_verify".into(),
            verified_by_gate_id: None,
            session_id: None,
            project_id: None,
            project_name: None,
            verified_by_gate_name: None,
            session_trusted_status: None,
            updated_at: None,
        };
        assert_eq!(asset_kind_for_artifact(&art), "media");
    }

    #[test]
    fn skill_asset_id_roundtrip() {
        assert_eq!(skill_id_from_asset_id("skill_foo"), Some("foo"));
        assert_eq!(skill_asset_id("foo"), "skill_foo");
    }
}
