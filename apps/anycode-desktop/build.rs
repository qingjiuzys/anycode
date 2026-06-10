//! Stage dashboard-ui dist into `resources/dashboard-ui/` before Tauri bundles resources.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("../../crates/dashboard-ui/dist");
    let dst = manifest.join("resources/dashboard-ui");
    if src.join("index.html").is_file() {
        let _ = std::fs::remove_dir_all(&dst);
        let status = Command::new("cp")
            .args(["-R", src.to_str().unwrap(), dst.to_str().unwrap()])
            .status();
        if let Ok(s) = status {
            if !s.success() {
                eprintln!("cargo:warning=failed to copy dashboard-ui dist into resources/");
            }
        }
    } else {
        eprintln!(
            "cargo:warning=dashboard-ui dist missing at {}; run ./scripts/build-dashboard-ui.sh",
            src.display()
        );
    }
    tauri_build::build()
}
