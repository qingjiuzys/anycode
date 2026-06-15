//! Playwright browser sessions for the workbench Browser panel.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct BrowserSessionInfo {
    pub session_id: String,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserScreenshot {
    pub image_base64: String,
    pub viewport: BrowserViewport,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserViewport {
    pub width: u32,
    pub height: u32,
}

struct BrowserProcess {
    child: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl BrowserProcess {
    fn request(&mut self, cmd: serde_json::Value) -> Result<serde_json::Value> {
        let line = serde_json::to_string(&cmd)?;
        writeln!(self.stdin, "{line}")?;
        self.stdin.flush()?;
        let mut response = String::new();
        self.stdout.read_line(&mut response)?;
        let value: serde_json::Value = serde_json::from_str(&response)
            .with_context(|| format!("invalid browser helper response: {response}"))?;
        if value.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            Ok(value)
        } else {
            let msg = value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("browser command failed");
            bail!("{msg}");
        }
    }
}

pub struct BrowserSessionManager {
    sessions: Mutex<HashMap<String, (String, BrowserProcess)>>,
}

impl Default for BrowserSessionManager {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl BrowserSessionManager {
    pub fn create(&self, project_id: &str) -> Result<BrowserSessionInfo> {
        let bundle = resolve_browser_bundle()?;
        let mut proc = spawn_browser_helper(&bundle)?;
        proc.request(serde_json::json!({ "cmd": "create" }))?;
        let session_id = Uuid::new_v4().to_string();
        self.sessions
            .lock()
            .map_err(|_| anyhow::anyhow!("browser session lock poisoned"))?
            .insert(session_id.clone(), (project_id.to_string(), proc));
        Ok(BrowserSessionInfo {
            session_id,
            project_id: project_id.to_string(),
        })
    }

    pub fn navigate(&self, session_id: &str, url: &str) -> Result<BrowserState> {
        let mut guard = self
            .sessions
            .lock()
            .map_err(|_| anyhow::anyhow!("browser session lock poisoned"))?;
        let (_, proc) = guard
            .get_mut(session_id)
            .context("browser session not found")?;
        let resp = proc.request(serde_json::json!({ "cmd": "navigate", "url": url }))?;
        Ok(serde_json::from_value(
            resp.get("state").cloned().unwrap_or_default(),
        )?)
    }

    pub fn state(&self, session_id: &str) -> Result<BrowserState> {
        let mut guard = self
            .sessions
            .lock()
            .map_err(|_| anyhow::anyhow!("browser session lock poisoned"))?;
        let (_, proc) = guard
            .get_mut(session_id)
            .context("browser session not found")?;
        let resp = proc.request(serde_json::json!({ "cmd": "state" }))?;
        Ok(serde_json::from_value(
            resp.get("state").cloned().unwrap_or_default(),
        )?)
    }

    pub fn screenshot(&self, session_id: &str) -> Result<BrowserScreenshot> {
        let mut guard = self
            .sessions
            .lock()
            .map_err(|_| anyhow::anyhow!("browser session lock poisoned"))?;
        let (_, proc) = guard
            .get_mut(session_id)
            .context("browser session not found")?;
        let resp = proc.request(serde_json::json!({ "cmd": "screenshot" }))?;
        Ok(BrowserScreenshot {
            image_base64: resp
                .get("image_base64")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            viewport: BrowserViewport {
                width: resp
                    .get("viewport")
                    .and_then(|v| v.get("width"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1280) as u32,
                height: resp
                    .get("viewport")
                    .and_then(|v| v.get("height"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(720) as u32,
            },
        })
    }

    pub fn close(&self, session_id: &str) -> Result<()> {
        let mut guard = self
            .sessions
            .lock()
            .map_err(|_| anyhow::anyhow!("browser session lock poisoned"))?;
        if let Some((_, mut proc)) = guard.remove(session_id) {
            let _ = proc.request(serde_json::json!({ "cmd": "close" }));
            let _ = proc.child.kill();
        }
        Ok(())
    }
}

fn resolve_browser_bundle() -> Result<PathBuf> {
    if let Ok(raw) = std::env::var("ANYCODE_BROWSER_MCP_ROOT") {
        let p = PathBuf::from(raw.trim());
        if crate::browser_connector::is_browser_bundle(&p) {
            return Ok(p);
        }
    }
    bail!("browser bundle not found; enable browser connector in settings or set ANYCODE_BROWSER_MCP_ROOT");
}

fn spawn_browser_helper(bundle: &Path) -> Result<BrowserProcess> {
    let script = browser_helper_script_path();
    let node = resolve_node_bin(bundle);
    let mut child = Command::new(&node);
    child
        .arg(&script)
        .env("PLAYWRIGHT_BROWSERS_PATH", bundle.join("browsers"))
        .env("PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS", "true")
        .env("NODE_PATH", bundle.join("node_modules"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = child.spawn().context("spawn browser helper")?;
    let stdin = child.stdin.take().context("browser helper stdin")?;
    let stdout = child.stdout.take().context("browser helper stdout")?;
    Ok(BrowserProcess {
        child,
        stdin,
        stdout: BufReader::new(stdout),
    })
}

fn resolve_node_bin(bundle: &Path) -> PathBuf {
    let bundled = bundle.join("node/bin/node");
    if bundled.is_file() {
        return bundled;
    }
    PathBuf::from("node")
}

fn browser_helper_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/browser-session.mjs")
}

pub fn shared_manager() -> Arc<BrowserSessionManager> {
    use std::sync::OnceLock;
    static MANAGER: OnceLock<Arc<BrowserSessionManager>> = OnceLock::new();
    MANAGER
        .get_or_init(|| Arc::new(BrowserSessionManager::default()))
        .clone()
}
