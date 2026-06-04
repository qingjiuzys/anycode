//! Ensure dashboard-ui dist exists when embedding UI in release builds.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dist = manifest_dir.join("../dashboard-ui/dist/index.html");
    let ui_src = manifest_dir.join("../dashboard-ui/src");

    println!("cargo:rerun-if-changed={}", ui_src.display());
    println!("cargo:rerun-if-changed={}", dist.display());
    println!("cargo:rerun-if-env-changed=ANYCODE_BUILD_DASHBOARD_UI");

    let embed = std::env::var("CARGO_FEATURE_EMBEDDED_UI").is_ok();
    if !embed {
        return;
    }

    if dist.is_file() {
        return;
    }

    if std::env::var("ANYCODE_SKIP_DASHBOARD_UI_BUILD").is_ok() {
        return;
    }

    let build_ui = std::env::var("ANYCODE_BUILD_DASHBOARD_UI")
        .map(|v| v == "1" || v == "true")
        .unwrap_or_else(|_| {
            std::env::var("PROFILE")
                .map(|p| p == "release")
                .unwrap_or(false)
        });

    if build_ui {
        try_build_ui(&manifest_dir);
    }

    if !dist.is_file() {
        write_embed_stub_dist(dist.parent().expect("dist parent"));
        println!(
            "cargo:warning=dashboard-ui dist missing; wrote compile-time stub at {} (run ./scripts/build-dashboard-ui.sh for real UI)",
            dist.display()
        );
    }
}

/// Minimal `dist/` so `rust-embed` compiles when UI has not been built (clippy/test on fresh checkouts).
fn write_embed_stub_dist(dist_dir: &Path) {
    let _ = std::fs::create_dir_all(dist_dir.join("assets"));
    let index = dist_dir.join("index.html");
    if !index.is_file() {
        let _ = std::fs::write(
            &index,
            r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>anycode</title></head><body><p>Dashboard UI not built. Run ./scripts/build-dashboard-ui.sh</p></body></html>"#,
        );
    }
    let stub_js = dist_dir.join("assets/stub.js");
    if !stub_js.is_file() {
        let _ = std::fs::write(stub_js, "// stub asset for rust-embed");
    }
}

fn try_build_ui(manifest_dir: &Path) {
    let script = manifest_dir.join("../../scripts/build-dashboard-ui.sh");
    if !script.is_file() {
        return;
    }
    let status = Command::new("bash").arg(script).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => println!("cargo:warning=build-dashboard-ui.sh exited with status {s}"),
        Err(e) => println!("cargo:warning=failed to run build-dashboard-ui.sh: {e}"),
    }
}
