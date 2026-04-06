//! User workspace `~/.anycode/workspace`: default WeChat cwd and project registry (OpenClaw-style).

use crate::app_config::Config;
use crate::i18n::tr;
use anycode_core::RuntimeMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 登记表中最多保留的路径条数。
const MAX_PROJECTS: usize = 200;

/// `~/.anycode/workspace`
pub fn root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("workspace")
}

/// 规范路径字符串，供 `config.env`、会话默认值；会 `create_dir_all` 根目录。
pub fn canonical_root_string() -> String {
    let r = root();
    let _ = fs::create_dir_all(&r);
    r.canonicalize().unwrap_or(r).to_string_lossy().to_string()
}

/// 创建 `workspace`、`projects/`，并写入 `README.md`（若不存在）。
pub fn ensure_layout() -> anyhow::Result<()> {
    let r = root();
    fs::create_dir_all(r.join("projects"))?;
    let readme = r.join("README.md");
    if !readme.is_file() {
        let body = tr("workspace-readme");
        fs::write(&readme, format!("{body}\n"))?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkspaceProject {
    pub path: String,
    pub last_seen: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_profile: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Index {
    #[serde(default)]
    projects: Vec<WorkspaceProject>,
}

fn index_path() -> PathBuf {
    root().join("projects").join("index.json")
}

fn load_index() -> Index {
    let ip = index_path();
    if !ip.is_file() {
        return Index::default();
    }
    fs::read_to_string(&ip)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// 将 `path` 登记到 `projects/index.json`（去重、按 `last_seen` 排序、截断）；失败仅打 debug 日志。
pub fn touch_project_dir(path: PathBuf) {
    if let Err(e) = touch_project_dir_inner(path) {
        tracing::debug!(error = %e, "workspace::touch_project_dir");
    }
}

fn touch_project_dir_inner(path: PathBuf) -> anyhow::Result<()> {
    ensure_layout()?;
    let path_str = path
        .canonicalize()
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let ip = index_path();
    let mut idx: Index = load_index();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    idx.projects.retain(|p| p.path != path_str);
    idx.projects.push(WorkspaceProject {
        path: path_str,
        last_seen: now,
        label: None,
        default_mode: None,
        channel_profile: None,
    });
    idx.projects.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    idx.projects.truncate(MAX_PROJECTS);

    let tmp = ip.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        serde_json::to_writer_pretty(&mut f, &idx)?;
        f.flush()?;
    }
    fs::rename(&tmp, &ip)?;
    Ok(())
}

pub fn recent_projects(limit: usize) -> Vec<String> {
    load_index()
        .projects
        .into_iter()
        .take(limit)
        .map(|entry| entry.path)
        .collect()
}

pub fn list_projects(limit: usize) -> Vec<WorkspaceProject> {
    load_index().projects.into_iter().take(limit).collect()
}

/// 在登记项目中按「路径前缀最长匹配」找到当前目录所属项目（用于默认模式 / 通道元数据）。
pub fn project_for_directory(working_dir: &Path) -> Option<WorkspaceProject> {
    let wd = fs::canonicalize(working_dir).unwrap_or_else(|_| working_dir.to_path_buf());
    best_project_match(&load_index().projects, &wd)
}

fn best_project_match(
    projects: &[WorkspaceProject],
    working_dir: &Path,
) -> Option<WorkspaceProject> {
    let mut best: Option<(usize, WorkspaceProject)> = None;
    for p in projects {
        let base = Path::new(&p.path);
        let base_canon = fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
        if working_dir.starts_with(&base_canon) {
            let len = base_canon.as_os_str().len();
            if best
                .as_ref()
                .map(|(prev_len, _)| len > *prev_len)
                .unwrap_or(true)
            {
                best = Some((len, p.clone()));
            }
        }
    }
    best.map(|(_, proj)| proj)
}

/// 将登记项目中的 `default_mode`、`label`、`channel_profile` 叠加到内存中的 [`Config`]（不改变磁盘上的 `config.json`）。
pub fn apply_project_overlays(config: &mut Config, working_dir: &Path) {
    let Some(proj) = project_for_directory(working_dir) else {
        config.runtime.workspace_project_label = None;
        config.runtime.workspace_channel_profile = None;
        return;
    };
    config.runtime.workspace_project_label = proj.label.clone();
    config.runtime.workspace_channel_profile = proj.channel_profile.clone();
    if let Some(ref dm) = proj.default_mode {
        if let Some(mode) = RuntimeMode::parse(dm) {
            config.runtime.default_mode = mode;
        }
    }
}

pub fn update_project_metadata(
    path: PathBuf,
    label: Option<String>,
    default_mode: Option<String>,
    channel_profile: Option<String>,
) -> anyhow::Result<()> {
    ensure_layout()?;
    let path_str = path
        .canonicalize()
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let ip = index_path();
    let mut idx: Index = load_index();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut found = false;
    for project in &mut idx.projects {
        if project.path == path_str {
            if let Some(label) = label.clone() {
                project.label = Some(label);
            }
            if let Some(default_mode) = default_mode.clone() {
                project.default_mode = Some(default_mode);
            }
            if let Some(channel_profile) = channel_profile.clone() {
                project.channel_profile = Some(channel_profile);
            }
            project.last_seen = now.clone();
            found = true;
            break;
        }
    }
    if !found {
        idx.projects.push(WorkspaceProject {
            path: path_str,
            last_seen: now,
            label,
            default_mode,
            channel_profile,
        });
    }
    idx.projects.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    idx.projects.truncate(MAX_PROJECTS);
    let tmp = ip.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        serde_json::to_writer_pretty(&mut f, &idx)?;
        f.flush()?;
    }
    fs::rename(&tmp, &ip)?;
    Ok(())
}

pub fn current_workspace_status(limit: usize) -> String {
    let current = std::env::current_dir()
        .ok()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(canonical_root_string);
    let recent = recent_projects(limit);
    if recent.is_empty() {
        format!("current_workspace: {current}\nrecent_projects: (none)")
    } else {
        format!(
            "current_workspace: {current}\nrecent_projects:\n- {}",
            recent.join("\n- ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn best_project_match_prefers_longest_prefix() {
        let projects = vec![
            WorkspaceProject {
                path: "/a".into(),
                last_seen: "t".into(),
                label: None,
                default_mode: None,
                channel_profile: None,
            },
            WorkspaceProject {
                path: "/a/b".into(),
                last_seen: "t".into(),
                label: Some("inner".into()),
                default_mode: None,
                channel_profile: None,
            },
        ];
        let wd = Path::new("/a/b/c");
        let m = best_project_match(&projects, wd);
        assert_eq!(m.as_ref().map(|p| p.path.as_str()), Some("/a/b"));
        assert_eq!(m.as_ref().and_then(|p| p.label.as_deref()), Some("inner"));
    }

    #[test]
    fn index_serde_roundtrip() {
        let mut idx = Index::default();
        idx.projects.push(WorkspaceProject {
            path: "/tmp/proj".into(),
            last_seen: "2020-01-01T00:00:00Z".into(),
            label: None,
            default_mode: None,
            channel_profile: None,
        });
        let j = serde_json::to_string(&idx).unwrap();
        let back: Index = serde_json::from_str(&j).unwrap();
        assert_eq!(back.projects.len(), 1);
        assert_eq!(back.projects[0].path, "/tmp/proj");
    }
}
