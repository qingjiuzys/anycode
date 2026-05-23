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
        println!(
            "cargo:warning=dashboard-ui dist missing at {}; run ./scripts/build-dashboard-ui.sh or set ANYCODE_BUILD_DASHBOARD_UI=1",
            dist.display()
        );
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
