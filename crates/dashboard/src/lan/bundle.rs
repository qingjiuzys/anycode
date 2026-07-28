//! Handoff bundle export and import.

use crate::db::DashboardDb;
use crate::lan::handoff::HandoffKind;
use crate::observability::session_transcript::session_transcript;
use crate::report::{session_report, ReportOptions};
use crate::schema::{CreateSessionRequest, ProjectDetail, SessionDetail, UpsertProjectRequest};
use anyhow::{bail, Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::Builder;
use walkdir::WalkDir;

const MANIFEST_NAME: &str = "manifest.json";
const SCHEMA_VERSION: &str = "handoff_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub schema_version: String,
    pub kind: HandoffKind,
    pub generated_at: String,
    pub source_instance_id: String,
    pub source_device_name: String,
    pub project: BundleProject,
    pub sessions: Vec<BundleSession>,
    #[serde(default)]
    pub memory_files: Vec<String>,
    #[serde(default)]
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleProject {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSession {
    pub detail: SessionDetail,
    pub transcript_json: String,
    #[serde(default)]
    pub report_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BundleExportOptions {
    pub kind: HandoffKind,
    pub project_id: String,
    pub session_id: Option<String>,
    pub source_instance_id: String,
    pub source_device_name: String,
    pub max_bytes: u64,
}

pub async fn export_bundle(
    db: &DashboardDb,
    memory_root: &Path,
    opts: BundleExportOptions,
) -> Result<PathBuf> {
    let project = db
        .get_project(&opts.project_id)
        .await?
        .context("project not found")?;
    let root = PathBuf::from(&project.root_path);
    if !root.is_dir() {
        bail!("project root missing: {}", project.root_path);
    }

    let sessions = match opts.kind {
        HandoffKind::Project => {
            let summaries = db.list_sessions(&opts.project_id, 200).await?;
            let mut out = Vec::new();
            for s in summaries {
                if let Some(d) = db.get_session(&s.id).await? {
                    out.push(d);
                }
            }
            out
        }
        HandoffKind::Session => {
            let sid = opts
                .session_id
                .as_deref()
                .context("session_id required for session handoff")?;
            vec![db.get_session(sid).await?.context("session not found")?]
        }
    };

    let mut bundle_sessions = Vec::new();
    for detail in &sessions {
        let transcript = session_transcript(db, &detail.id).await?;
        let transcript_json = serde_json::to_string_pretty(&transcript)?;
        let report = session_report(db, &detail.id, ReportOptions::default(), false)
            .await
            .ok();
        let report_json = report
            .map(|r| serde_json::to_string_pretty(&r))
            .transpose()?;
        bundle_sessions.push(BundleSession {
            detail: detail.clone(),
            transcript_json,
            report_json,
        });
    }

    let memory_files = collect_memory_files(memory_root, &project)?;
    let artifacts = db
        .list_artifacts(
            Some(&opts.project_id),
            opts.session_id.as_deref(),
            None,
            None,
            None,
            false,
            false,
            false,
            500,
        )
        .await?;
    let artifact_paths: Vec<String> = artifacts
        .into_iter()
        .map(|a| a.path)
        .filter(|p| !p.is_empty())
        .collect();

    let manifest = BundleManifest {
        schema_version: SCHEMA_VERSION.into(),
        kind: opts.kind,
        generated_at: chrono::Utc::now().to_rfc3339(),
        source_instance_id: opts.source_instance_id,
        source_device_name: opts.source_device_name,
        project: BundleProject {
            id: project.id.clone(),
            name: project.name.clone(),
            root_path: project.root_path.clone(),
            description: project.description.clone(),
        },
        sessions: bundle_sessions,
        memory_files: memory_files
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        artifact_paths: artifact_paths.clone(),
    };

    redact_secrets_in_manifest(&manifest)?;

    let staging = std::env::temp_dir().join(format!(
        "anycode-handoff-{}.tar.gz",
        uuid::Uuid::new_v4().simple()
    ));
    write_tarball(
        &staging,
        &manifest,
        &root,
        opts.kind == HandoffKind::Project,
        &memory_files,
        &artifact_paths,
        &root,
        opts.max_bytes,
    )?;
    Ok(staging)
}

fn redact_secrets_in_manifest(manifest: &BundleManifest) -> Result<()> {
    let json = serde_json::to_string(manifest)?;
    let forbidden = ["api_key", "credentials", "secret", "password", "token_hash"];
    let lower = json.to_lowercase();
    for word in forbidden {
        if lower.contains(word) {
            bail!("bundle manifest contains forbidden secret marker: {word}");
        }
    }
    Ok(())
}

fn collect_memory_files(memory_root: &Path, project: &ProjectDetail) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for sub in ["project", "feedback", "user", "reference"] {
        let dir = memory_root.join(sub);
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Ok(text) = fs::read_to_string(path) {
                if text.contains(&project.id) || text.contains(&project.root_path) {
                    out.push(path.to_path_buf());
                }
            }
        }
    }
    Ok(out)
}

fn write_tarball(
    dest: &Path,
    manifest: &BundleManifest,
    workspace_root: &Path,
    include_workspace: bool,
    memory_files: &[PathBuf],
    artifact_paths: &[String],
    project_root: &Path,
    max_bytes: u64,
) -> Result<()> {
    let file = File::create(dest).context("create bundle file")?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    let manifest_bytes = serde_json::to_vec_pretty(manifest)?;
    tar.append_data(
        &mut tar::Header::new_gnu(),
        MANIFEST_NAME,
        &manifest_bytes[..],
    )?;

    if include_workspace {
        add_dir_to_tar(
            &mut tar,
            workspace_root,
            "workspace",
            project_root,
            max_bytes,
        )?;
    }

    for mem in memory_files {
        if let Ok(rel) = mem.strip_prefix(dirs::home_dir().unwrap_or_default().join(".anycode")) {
            let name = format!("memories/{}", rel.display());
            tar.append_path_with_name(mem, &name)?;
        }
    }

    for rel in artifact_paths {
        let src = project_root.join(rel);
        if src.is_file() {
            let name = format!("artifacts/{}", rel);
            tar.append_path_with_name(&src, &name)?;
        }
    }

    for session in &manifest.sessions {
        let tpath = format!("transcripts/{}.json", session.detail.id);
        tar.append_data(
            &mut tar::Header::new_gnu(),
            &tpath,
            session.transcript_json.as_bytes(),
        )?;
        if let Some(report) = &session.report_json {
            let rpath = format!("reports/{}.json", session.detail.id);
            tar.append_data(&mut tar::Header::new_gnu(), &rpath, report.as_bytes())?;
        }
    }

    tar.finish()?;
    let enc = tar.into_inner()?;
    let mut file = enc.finish()?;
    file.flush()?;

    let size = fs::metadata(dest)?.len();
    if size > max_bytes {
        let _ = fs::remove_file(dest);
        bail!("bundle size {} exceeds limit {} bytes", size, max_bytes);
    }
    Ok(())
}

fn add_dir_to_tar<W: Write>(
    tar: &mut Builder<W>,
    dir: &Path,
    prefix: &str,
    project_root: &Path,
    max_bytes: u64,
) -> Result<()> {
    let mut total: u64 = 0;
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let rel = path.strip_prefix(project_root).unwrap_or(path);
        let rel_str = rel.display().to_string();
        if should_skip_path(&rel_str) {
            continue;
        }
        let meta = fs::metadata(path)?;
        total += meta.len();
        if total > max_bytes {
            bail!("workspace exceeds bundle size limit");
        }
        let name = format!("{prefix}/{}", rel.display());
        tar.append_path_with_name(path, &name)?;
    }
    Ok(())
}

fn should_skip_path(rel: &str) -> bool {
    const SKIP: &[&str] = &[
        "node_modules/",
        ".git/",
        "target/",
        "dist/",
        "build/",
        ".anycode/credentials",
    ];
    SKIP.iter().any(|s| rel.contains(s))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub kind: HandoffKind,
    pub target_root_path: Option<String>,
    pub target_project_id: Option<String>,
}

pub struct ImportResult {
    pub project_id: String,
    pub root_path: String,
    pub sessions_imported: usize,
}

pub async fn import_bundle(
    db: &DashboardDb,
    memory_root: &Path,
    bundle_path: &Path,
    opts: ImportOptions,
) -> Result<ImportResult> {
    let staging = extract_tarball(bundle_path)?;
    let manifest_path = staging.join(MANIFEST_NAME);
    let manifest: BundleManifest =
        serde_json::from_slice(&fs::read(manifest_path).context("read manifest")?)?;

    let root_path = resolve_import_root(db, &manifest, &opts).await?;
    if opts.kind == HandoffKind::Project {
        let ws_src = staging.join("workspace");
        if ws_src.is_dir() {
            copy_tree_merge(&ws_src, Path::new(&root_path))?;
        }
    }

    let project = db
        .upsert_project(UpsertProjectRequest {
            root_path: root_path.clone(),
            name: Some(manifest.project.name.clone()),
            description: Some(manifest.project.description.clone()),
            create_root: Some(true),
            template_id: None,
            app_title: None,
            bundle_org: None,
        })
        .await?;
    let new_project_id = project.id.clone();

    let mut imported = 0usize;
    for session in &manifest.sessions {
        let _new_id = import_session(db, &new_project_id, session).await?;
        imported += 1;
    }

    let mem_src = staging.join("memories");
    if mem_src.is_dir() {
        copy_tree_merge(&mem_src, memory_root)?;
    }

    let art_src = staging.join("artifacts");
    if art_src.is_dir() {
        copy_tree_merge(&art_src, Path::new(&root_path))?;
    }

    Ok(ImportResult {
        project_id: new_project_id,
        root_path,
        sessions_imported: imported,
    })
}

async fn resolve_import_root(
    db: &DashboardDb,
    manifest: &BundleManifest,
    opts: &ImportOptions,
) -> Result<String> {
    if let Some(path) = opts
        .target_root_path
        .as_ref()
        .filter(|p| !p.trim().is_empty())
    {
        return Ok(path.clone());
    }
    if let Some(pid) = opts
        .target_project_id
        .as_ref()
        .filter(|p| !p.trim().is_empty())
    {
        let project = db
            .get_project(pid)
            .await?
            .context("target project not found")?;
        return Ok(project.root_path);
    }
    if opts.kind == HandoffKind::Project {
        let base = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anycode-handoffs")
            .join(sanitize_dir_name(&manifest.project.name));
        fs::create_dir_all(&base)?;
        return Ok(base.display().to_string());
    }
    bail!("target_root_path or target_project_id required for session handoff");
}

async fn import_session(
    db: &DashboardDb,
    project_id: &str,
    session: &BundleSession,
) -> Result<String> {
    let req = CreateSessionRequest {
        project_id: project_id.into(),
        kind: session.detail.kind.clone(),
        task_id: None,
        title: session.detail.title.clone(),
        prompt_preview: Some(session.detail.prompt_preview.clone()),
        agent_type: Some(session.detail.agent_type.clone()),
        model: Some(session.detail.model.clone()),
        metadata_json: Some(session.detail.metadata_json.clone()),
    };
    let created = db.create_session(req).await?;
    if !session.detail.summary.is_empty() {
        let _ = db
            .finish_session(&created.id, "completed", Some(&session.detail.summary))
            .await;
    }
    Ok(created.id)
}

fn extract_tarball(path: &Path) -> Result<PathBuf> {
    let staging = std::env::temp_dir().join(format!(
        "anycode-handoff-import-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&staging)?;
    let file = File::open(path)?;
    let dec = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(dec);
    archive.unpack(&staging)?;
    Ok(staging)
}

fn copy_tree_merge(src: &Path, dst: &Path) -> Result<()> {
    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let rel = path.strip_prefix(src)?;
        let target = dst.join(rel);
        if target.exists() {
            let stem = target
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = target.extension().and_then(|s| s.to_str()).unwrap_or("");
            let parent = target.parent().unwrap_or(dst);
            let alt = if ext.is_empty() {
                parent.join(format!("{stem}-imported"))
            } else {
                parent.join(format!("{stem}-imported.{ext}"))
            };
            if let Some(parent) = alt.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &alt)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}

fn sanitize_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_root::project_id_for_root;

    #[test]
    fn manifest_redaction_blocks_api_key() {
        let manifest = BundleManifest {
            schema_version: SCHEMA_VERSION.into(),
            kind: HandoffKind::Project,
            generated_at: chrono::Utc::now().to_rfc3339(),
            source_instance_id: "x".into(),
            source_device_name: "x".into(),
            project: BundleProject {
                id: "p".into(),
                name: "n".into(),
                root_path: "/tmp".into(),
                description: "d with api_key leak".into(),
            },
            sessions: vec![],
            memory_files: vec![],
            artifact_paths: vec![],
        };
        assert!(redact_secrets_in_manifest(&manifest).is_err());
    }

    #[test]
    fn project_id_remapped_on_new_root() {
        let a = project_id_for_root("/tmp/project-a");
        let b = project_id_for_root("/tmp/project-b");
        assert_ne!(a, b);
    }
}
