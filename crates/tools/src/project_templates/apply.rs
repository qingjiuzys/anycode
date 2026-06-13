use super::manifest::{load_manifest, template_dir};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct ApplyTemplateOptions {
    pub project_name: Option<String>,
    pub app_title: Option<String>,
    pub bundle_org: Option<String>,
    /// Overwrite when target exists and only contains safe-to-replace files.
    pub force: bool,
    /// Run `flutter create` when Flutter is on PATH (opt-in; default is agent-first skeleton).
    pub run_flutter_create: bool,
}

#[derive(Debug, Clone)]
pub struct ApplyTemplateResult {
    pub template_id: String,
    pub root: PathBuf,
    pub project_name: String,
}

#[derive(Serialize)]
struct FlutterProjectMeta {
    project_name: String,
    app_title: String,
    bundle_org: String,
    platforms: Vec<String>,
}

pub fn apply_project_template(
    template_id: &str,
    target: &Path,
    opts: ApplyTemplateOptions,
) -> Result<ApplyTemplateResult> {
    let manifest = load_manifest(template_id)?;
    let dir = template_dir(template_id)?;
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());

    if target.exists() {
        ensure_empty_or_force(&target, opts.force)?;
    } else if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let project_name = normalize_project_name(
        opts.project_name
            .as_deref()
            .or(target.file_name().and_then(|s| s.to_str()))
            .unwrap_or(&manifest.default_dir),
    );
    let app_title = opts
        .app_title
        .clone()
        .unwrap_or_else(|| project_name.replace('_', " "));
    let bundle_org = opts
        .bundle_org
        .clone()
        .or_else(|| manifest.flutter.as_ref().map(|f| f.default_org.clone()))
        .unwrap_or_else(|| "com.example.app".into());

    let vars = HashMap::from([
        ("project_name".into(), project_name.clone()),
        ("app_title".into(), app_title.clone()),
        ("bundle_org".into(), bundle_org.clone()),
    ]);

    if template_id == "flutter-app" {
        apply_flutter_app(&dir, &target, &manifest, &vars, &opts)?;
    } else {
        anyhow::bail!("template {template_id} has no apply handler");
    }

    Ok(ApplyTemplateResult {
        template_id: template_id.to_string(),
        root: target,
        project_name,
    })
}

fn wants_flutter_create(opts: &ApplyTemplateOptions) -> bool {
    opts.run_flutter_create || std::env::var_os("ANYCODE_TEMPLATE_RUN_FLUTTER_CREATE").is_some()
}

fn flutter_on_path() -> bool {
    Command::new("flutter")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn apply_flutter_app(
    template_dir: &Path,
    target: &Path,
    manifest: &super::manifest::ProjectTemplateManifest,
    vars: &HashMap<String, String>,
    opts: &ApplyTemplateOptions,
) -> Result<()> {
    fs::create_dir_all(target)?;
    let flutter = manifest
        .flutter
        .as_ref()
        .context("flutter-app manifest needs flutter section")?;

    copy_tree(&template_dir.join("skeleton"), target, vars)?;
    if template_dir.join("overlay").is_dir() {
        copy_tree(&template_dir.join("overlay"), target, vars)?;
    }
    if template_dir.join("skills").is_dir() {
        copy_tree(
            &template_dir.join("skills"),
            &target.join(".anycode/skills"),
            vars,
        )?;
    } else if template_dir.join(".anycode").is_dir() {
        copy_tree(
            &template_dir.join(".anycode"),
            &target.join(".anycode"),
            vars,
        )?;
    }

    let meta = FlutterProjectMeta {
        project_name: vars.get("project_name").cloned().unwrap_or_default(),
        app_title: vars.get("app_title").cloned().unwrap_or_default(),
        bundle_org: vars
            .get("bundle_org")
            .cloned()
            .unwrap_or_else(|| flutter.default_org.clone()),
        platforms: flutter.platforms.clone(),
    };
    let anycode_dir = target.join(".anycode");
    fs::create_dir_all(&anycode_dir)?;
    fs::write(
        anycode_dir.join("flutter-project.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

    if wants_flutter_create(opts) && flutter_on_path() {
        run_flutter_create(target, &meta)?;
        run_flutter_pub_get(target)?;
    }

    Ok(())
}

fn run_flutter_create(target: &Path, meta: &FlutterProjectMeta) -> Result<()> {
    let platforms = meta.platforms.join(",");
    let status = Command::new("flutter")
        .arg("create")
        .arg(".")
        .arg("--project-name")
        .arg(&meta.project_name)
        .arg("--org")
        .arg(&meta.bundle_org)
        .arg(format!("--platforms={platforms}"))
        .current_dir(target)
        .status()
        .context("spawn flutter create")?;
    if !status.success() {
        anyhow::bail!("flutter create failed with status {status}");
    }
    Ok(())
}

fn run_flutter_pub_get(target: &Path) -> Result<()> {
    let status = Command::new("flutter")
        .args(["pub", "get"])
        .current_dir(target)
        .status()
        .context("spawn flutter pub get")?;
    if !status.success() {
        anyhow::bail!("flutter pub get failed");
    }
    Ok(())
}

fn ensure_empty_or_force(path: &Path, force: bool) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let entries: Vec<_> = fs::read_dir(path)?.filter_map(Result::ok).collect();
    if entries.is_empty() {
        return Ok(());
    }
    if force {
        for ent in entries {
            let p = ent.path();
            if ent.file_type()?.is_dir() {
                fs::remove_dir_all(&p)?;
            } else {
                fs::remove_file(&p)?;
            }
        }
        return Ok(());
    }
    anyhow::bail!(
        "target directory is not empty: {} (use --force to replace)",
        path.display()
    );
}

fn normalize_project_name(raw: &str) -> String {
    let mut s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    s = s.trim_matches('_').to_string();
    if s.is_empty() {
        "my_flutter_app".into()
    } else if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("app_{s}")
    } else {
        s
    }
}

fn copy_tree(src: &Path, dst: &Path, vars: &HashMap<String, String>) -> Result<()> {
    if !src.is_dir() {
        anyhow::bail!("missing template tree: {}", src.display());
    }
    for ent in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let rel = ent.path().strip_prefix(src).context("strip_prefix")?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out_rel = rel
            .to_str()
            .map(|s| render_str(s, vars))
            .context("path utf8")?;
        let out_path = dst.join(out_rel);
        if ent.file_type().is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        let content = fs::read(ent.path())?;
        let text = String::from_utf8_lossy(&content);
        let rendered = render_str(&text, vars);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(out_path, rendered.as_bytes())?;
    }
    Ok(())
}

fn render_str(input: &str, vars: &HashMap<String, String>) -> String {
    let mut out = input.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_flutter_skeleton_without_sdk() {
        let tmp = tempfile::tempdir().unwrap();
        let r = apply_project_template(
            "flutter-app",
            tmp.path(),
            ApplyTemplateOptions {
                project_name: Some("demo_app".into()),
                app_title: Some("演示".into()),
                ..Default::default()
            },
        )
        .expect("apply");
        assert!(r.root.join("pubspec.yaml").is_file());
        assert!(r.root.join("README.md").is_file());
        assert!(r
            .root
            .join(".anycode/skills/flutter-prd/SKILL.md")
            .is_file());
        assert!(r.root.join(".anycode/flutter-project.json").is_file());
        assert!(!r.root.join("ios").exists());
    }
}
