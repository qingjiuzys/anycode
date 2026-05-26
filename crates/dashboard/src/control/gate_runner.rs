//! On-demand verification gate execution for project workspaces.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatePreset {
    pub id: String,
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateExecuteResult {
    pub name: String,
    pub command: String,
    pub status: String,
    pub output_excerpt: String,
    pub elapsed_ms: u64,
}

fn is_rust_project(root: &Path) -> bool {
    root.join("Cargo.toml").is_file()
        || root.join("Cargo.lock").is_file()
        || root.join("rust-toolchain.toml").is_file()
}

#[must_use]
pub fn list_presets(project_root: &Path) -> Vec<GatePreset> {
    let mut presets = Vec::new();

    if is_rust_project(project_root) {
        presets.extend([
            GatePreset {
                id: "cargo_fmt".into(),
                name: "cargo fmt check".into(),
                command: "cargo fmt --all -- --check".into(),
            },
            GatePreset {
                id: "cargo_clippy".into(),
                name: "cargo clippy".into(),
                command: "cargo clippy --workspace --all-targets -- -D warnings".into(),
            },
            GatePreset {
                id: "cargo_test".into(),
                name: "cargo test".into(),
                command: "cargo test --workspace --quiet".into(),
            },
        ]);
    }

    if project_root.join("package.json").is_file() {
        presets.push(GatePreset {
            id: "npm_test".into(),
            name: "npm test".into(),
            command: "npm test --if-present".into(),
        });
        if project_root.join("playwright.config.ts").is_file()
            || project_root.join("playwright.config.js").is_file()
        {
            presets.push(GatePreset {
                id: "playwright".into(),
                name: "playwright test".into(),
                command: "npx playwright test --reporter=line".into(),
            });
        }
    }

    if project_root.join("pubspec.yaml").is_file() {
        presets.extend([
            GatePreset {
                id: "flutter_analyze".into(),
                name: "flutter analyze".into(),
                command: "flutter analyze".into(),
            },
            GatePreset {
                id: "flutter_test".into(),
                name: "flutter test".into(),
                command: "flutter test".into(),
            },
        ]);
    }

    if project_root.join("go.mod").is_file() {
        presets.push(GatePreset {
            id: "go_test".into(),
            name: "go test".into(),
            command: "go test ./...".into(),
        });
    }

    if project_root.join("pyproject.toml").is_file() || project_root.join("pytest.ini").is_file() {
        presets.push(GatePreset {
            id: "pytest".into(),
            name: "pytest".into(),
            command: "pytest -q".into(),
        });
    }

    presets
}

use anyhow::{Context, Result};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

pub async fn execute_gate(
    project_root: &Path,
    name: &str,
    command: &str,
) -> Result<GateExecuteResult> {
    let t0 = Instant::now();
    let output = tokio::process::Command::new("sh")
        .arg("-lc")
        .arg(command)
        .current_dir(project_root)
        .output()
        .await
        .with_context(|| format!("spawn gate {name}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let excerpt = truncate_gate_output(&combined, 4000);
    let status = if output.status.success() {
        "passed"
    } else {
        "failed"
    };
    Ok(GateExecuteResult {
        name: name.to_string(),
        command: command.to_string(),
        status: status.into(),
        output_excerpt: excerpt,
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

/// Run gate with line-by-line output sent to `line_tx` (for SSE streaming).
pub async fn execute_gate_streaming(
    project_root: &Path,
    name: &str,
    command: &str,
    line_tx: mpsc::Sender<String>,
) -> Result<GateExecuteResult> {
    let t0 = Instant::now();
    let combined = Arc::new(Mutex::new(String::new()));
    let mut child = Command::new("sh")
        .arg("-lc")
        .arg(command)
        .current_dir(project_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn gate {name}"))?;

    let mut handles = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let tx = line_tx.clone();
        let buf = combined.clone();
        handles.push(tokio::spawn(async move {
            pump_lines(stdout, tx, buf).await;
        }));
    }
    if let Some(stderr) = child.stderr.take() {
        let tx = line_tx.clone();
        let buf = combined.clone();
        handles.push(tokio::spawn(async move {
            pump_lines(stderr, tx, buf).await;
        }));
    }

    let status = child
        .wait()
        .await
        .with_context(|| format!("wait gate {name}"))?;
    for h in handles {
        let _ = h.await;
    }

    let combined = combined.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let excerpt = truncate_gate_output(&combined, 4000);
    let gate_status = if status.success() { "passed" } else { "failed" };
    Ok(GateExecuteResult {
        name: name.to_string(),
        command: command.to_string(),
        status: gate_status.into(),
        output_excerpt: excerpt,
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

async fn pump_lines<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    tx: mpsc::Sender<String>,
    combined: Arc<Mutex<String>>,
) {
    let mut lines = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match lines.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                if !trimmed.is_empty() {
                    if let Ok(mut c) = combined.lock() {
                        c.push_str(&trimmed);
                        c.push('\n');
                    }
                    let _ = tx.send(trimmed).await;
                }
            }
            Err(_) => break,
        }
    }
}

fn truncate_gate_output(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn executes_echo_gate() {
        let dir = tempdir().unwrap();
        let res = execute_gate(dir.path(), "echo", "echo GATE_OK")
            .await
            .unwrap();
        assert_eq!(res.status, "passed");
        assert!(res.output_excerpt.contains("GATE_OK"));
    }

    #[tokio::test]
    async fn streaming_echo_emits_lines() {
        let dir = tempdir().unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let res = execute_gate_streaming(dir.path(), "echo", "printf 'a\\nb\\n'", tx)
            .await
            .unwrap();
        assert_eq!(res.status, "passed");
        assert!(res.output_excerpt.contains('a'));
        assert!(res.output_excerpt.contains('b'));
    }

    #[test]
    fn presets_include_npm_when_package_json() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        let ids: Vec<_> = list_presets(dir.path()).into_iter().map(|p| p.id).collect();
        assert!(ids.contains(&"npm_test".to_string()));
        assert!(!ids.contains(&"cargo_test".to_string()));
    }

    #[test]
    fn presets_include_cargo_when_cargo_toml() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        let ids: Vec<_> = list_presets(dir.path()).into_iter().map(|p| p.id).collect();
        assert!(ids.contains(&"cargo_test".to_string()));
    }
}
