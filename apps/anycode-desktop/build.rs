//! Stage dashboard-ui dist into `resources/dashboard-ui/` before Tauri bundles resources.
//! Dev fast path: set `ANYCODE_DESKTOP_SKIP_UI_STAGE=1` (see `scripts/sync-desktop-dev.sh`).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

fn dist_stamp(src: &Path) -> Option<String> {
    let index = src.join("index.html");
    let meta = fs::metadata(&index).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("{}:{}", meta.len(), modified))
}

fn should_stage_ui(src: &Path, dst: &Path) -> bool {
    if env::var("ANYCODE_DESKTOP_SKIP_UI_STAGE")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        return false;
    }
    if !src.join("index.html").is_file() {
        return false;
    }
    let fp_path = dst
        .parent()
        .unwrap_or(dst)
        .join(".ui-stage-fingerprint");
    let Some(current) = dist_stamp(src) else {
        return true;
    };
    if dst.join("index.html").is_file() {
        if let Ok(prev) = fs::read_to_string(&fp_path) {
            if prev.trim() == current {
                return false;
            }
        }
    }
    true
}

fn write_fingerprint(src: &Path, dst: &Path) {
    let fp_path = dst
        .parent()
        .unwrap_or(dst)
        .join(".ui-stage-fingerprint");
    if let Some(stamp) = dist_stamp(src) {
        let _ = fs::write(fp_path, stamp);
    }
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    fn recurse(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let from = entry.path();
            let to = dst.join(entry.file_name());
            if ty.is_dir() {
                recurse(&from, &to)?;
            } else if ty.is_file() {
                fs::copy(&from, &to)?;
            }
        }
        Ok(())
    }
    recurse(src, dst)
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("../../crates/dashboard-ui/dist");
    let dst = manifest.join("resources/dashboard-ui");

    println!(
        "cargo:rerun-if-changed={}",
        src.join("index.html").display()
    );
    println!("cargo:rerun-if-env-changed=ANYCODE_BUILD_DASHBOARD_UI");
    println!("cargo:rerun-if-env-changed=ANYCODE_DESKTOP_SKIP_UI_STAGE");

    if src.join("index.html").is_file() {
        if should_stage_ui(&src, &dst) {
            if let Err(e) = copy_tree(&src, &dst) {
                eprintln!("cargo:warning=failed to stage dashboard-ui: {e}");
            } else {
                write_fingerprint(&src, &dst);
            }
        }
        let loading = manifest.join("loading.html");
        let dist_loading = src.join("loading.html");
        if loading.is_file() {
            let _ = fs::copy(&loading, &dist_loading);
        }
    } else {
        eprintln!(
            "cargo:warning=dashboard-ui dist missing at {}; run ./scripts/build-dashboard-ui.sh",
            src.display()
        );
    }
    tauri_build::build()
}
