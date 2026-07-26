//! Managed runtimes (Python / Node) under `~/.anycode/runtimes`.
//!
//! anyCode skills and tools frequently shell out to `python3` / `node`.
//! Desktop apps launched from a GUI shell have a sparse PATH and end users
//! rarely manage interpreter versions, so the runtime guarantees both
//! interpreters the same way current AI tooling does (Claude Code's native
//! installer bundles its runtime; Anthropic's skills rely on uv-managed
//! Python): managed copies under `~/.anycode/runtimes`, injected at the
//! front of the process PATH so every spawned tool/skill inherits them.
//! Provisioning itself is delegated to `scripts/provision-runtimes.sh`
//! (uv-managed CPython + official Node.js tarball), with system
//! interpreters as fallback.

use std::path::PathBuf;
use tracing::info;

/// Root directory for managed runtimes (override: `ANYCODE_RUNTIMES_DIR`).
#[must_use]
pub fn runtimes_root() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("ANYCODE_RUNTIMES_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    dirs::home_dir().map(|h| h.join(".anycode/runtimes"))
}

/// Existing managed `bin` directories in PATH precedence order
/// (python → node → uv/tooling bin).
#[must_use]
pub fn runtime_bin_dirs() -> Vec<PathBuf> {
    let Some(root) = runtimes_root() else {
        return Vec::new();
    };
    ["python/bin", "node/bin", "bin"]
        .iter()
        .map(|p| root.join(p))
        .filter(|p| p.is_dir())
        .collect()
}

/// Prepend managed runtime bin dirs to the process `PATH` (idempotent).
/// All subsequently spawned children (Bash tool, skill `run` scripts,
/// cron tasks) resolve `python3` / `node` to the managed copies first.
pub fn prepend_runtime_paths() {
    let bins = runtime_bin_dirs();
    if bins.is_empty() {
        return;
    }
    let new_segments: Vec<PathBuf> = bins.clone();
    let current = std::env::var("PATH").unwrap_or_default();
    let current_segments: Vec<PathBuf> = std::env::split_paths(&current).collect();
    if new_segments
        .iter()
        .all(|s| current_segments.iter().any(|c| c == s))
    {
        return;
    }
    let mut combined = new_segments.clone();
    combined.extend(current_segments);
    let Ok(joined) = std::env::join_paths(combined) else {
        return;
    };
    std::env::set_var("PATH", joined);
    info!(
        target: "anycode_bootstrap",
        dirs = ?new_segments,
        "managed runtime paths prepended to PATH"
    );
}

/// Locate the runtime provisioning script (installed or dev checkout).
#[must_use]
pub fn resolve_provision_script() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ANYCODE_PROVISION_RUNTIMES") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let mut candidates = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".anycode/scripts/provision-runtimes.sh"));
    }
    candidates.push(PathBuf::from("scripts/provision-runtimes.sh"));
    candidates.into_iter().find(|p| p.is_file())
}

fn managed_python_present() -> bool {
    runtimes_root()
        .map(|r| r.join("python/bin/python3").is_file())
        .unwrap_or(false)
}

fn managed_node_present() -> bool {
    runtimes_root()
        .map(|r| r.join("node/bin/node").is_file())
        .unwrap_or(false)
}

/// Best-effort background provisioning: runs `provision-runtimes.sh` when a
/// managed interpreter is missing and the script is available. Never blocks
/// or fails runtime startup — system interpreters remain the fallback.
pub fn spawn_runtime_provision() {
    if managed_python_present() && managed_node_present() {
        return;
    }
    let Some(script) = resolve_provision_script() else {
        info!(
            target: "anycode_bootstrap",
            "managed runtimes incomplete and provision script not found; \
             using system python3/node if present (install via scripts/provision-runtimes.sh)"
        );
        return;
    };
    tokio::spawn(async move {
        let status = tokio::process::Command::new("bash")
            .arg(&script)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        match status {
            Ok(s) if s.success() => {
                prepend_runtime_paths();
                info!(target: "anycode_bootstrap", "managed runtimes provisioning finished");
            }
            Ok(s) => tracing::warn!(
                target: "anycode_bootstrap",
                code = ?s.code(),
                "runtime provisioning script exited non-zero; falling back to system interpreters"
            ),
            Err(e) => tracing::warn!(
                target: "anycode_bootstrap",
                error = %e,
                "runtime provisioning script failed to start; falling back to system interpreters"
            ),
        }
    });
}

/// Status snapshot for diagnostics / Workbench API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeStatus {
    pub python: Option<String>,
    pub node: Option<String>,
    pub managed_python: bool,
    pub managed_node: bool,
}

fn which_version(cmd: &str, arg: &str) -> Option<String> {
    let out = std::process::Command::new(cmd).arg(arg).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Detect python/node currently reachable (managed PATH entries already
/// prepended win over system interpreters).
#[must_use]
pub fn detect_runtime_status() -> RuntimeStatus {
    RuntimeStatus {
        python: which_version("python3", "--version"),
        node: which_version("node", "--version"),
        managed_python: managed_python_present(),
        managed_node: managed_node_present(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bin_dirs_only_lists_existing_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("python/bin")).unwrap();
        std::env::set_var("ANYCODE_RUNTIMES_DIR", dir.path().display().to_string());
        let dirs = runtime_bin_dirs();
        assert_eq!(dirs.len(), 1);
        assert!(dirs[0].ends_with("python/bin"));
        std::env::remove_var("ANYCODE_RUNTIMES_DIR");
    }

    #[test]
    fn provision_script_resolution_prefers_env_override() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("provision-runtimes.sh");
        std::fs::write(&script, "#!/usr/bin/env bash\n").unwrap();
        std::env::set_var("ANYCODE_PROVISION_RUNTIMES", script.display().to_string());
        assert_eq!(resolve_provision_script().unwrap(), script);
        std::env::remove_var("ANYCODE_PROVISION_RUNTIMES");
    }
}
