//! Built-in Playwright browser MCP bundle detection and config helpers.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const CONFIG_KEY: &str = "mcp";

pub fn resolve_browser_mcp_bundle_root() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("ANYCODE_BROWSER_MCP_ROOT") {
        let p = PathBuf::from(raw.trim());
        if is_browser_bundle(&p) {
            return Some(p);
        }
    }
    None
}

pub fn is_browser_bundle(root: &Path) -> bool {
    root.join("run.sh").is_file() && root.join("node_modules/@playwright/mcp/cli.js").is_file()
}

pub fn browser_chromium_present(root: &Path) -> bool {
    let browsers = root.join("browsers");
    browsers.is_dir()
        && std::fs::read_dir(&browsers)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
}

pub fn read_browser_enabled(cfg: &Value) -> bool {
    cfg.get(CONFIG_KEY)
        .and_then(|m| m.get("browser"))
        .and_then(|b| b.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn set_browser_enabled(cfg: &mut Value, enabled: bool) {
    let root = cfg.as_object_mut().expect("config root must be object");
    let mcp = root
        .entry(CONFIG_KEY)
        .or_insert_with(|| json!({ "browser": { "enabled": false } }));
    if let Some(obj) = mcp.as_object_mut() {
        obj.insert("browser".into(), json!({ "enabled": enabled }));
    }
}

pub fn browser_connector_doctor_check(enabled: bool) -> crate::schema::DoctorCheck {
    let bundle = resolve_browser_mcp_bundle_root();
    if !enabled {
        return crate::schema::DoctorCheck {
            id: "browser_connector".into(),
            status: "ok".into(),
            message: "Built-in browser connector disabled".into(),
        };
    }
    let Some(root) = bundle.filter(|p| is_browser_bundle(p)) else {
        return crate::schema::DoctorCheck {
            id: "browser_connector".into(),
            status: "error".into(),
            message: "Browser connector enabled but bundle missing (reinstall desktop app)".into(),
        };
    };
    if !browser_chromium_present(&root) {
        return crate::schema::DoctorCheck {
            id: "browser_connector".into(),
            status: "warn".into(),
            message: "Browser MCP bundle found but Chromium binaries missing".into(),
        };
    }
    crate::schema::DoctorCheck {
        id: "browser_connector".into(),
        status: "ok".into(),
        message: format!(
            "Browser connector ready (Playwright MCP at {})",
            root.display()
        ),
    }
}

pub fn browser_connector_status() -> Value {
    let bundle = resolve_browser_mcp_bundle_root();
    let bundled = bundle.as_ref().is_some_and(|p| is_browser_bundle(p));
    let chromium_ready = bundle.as_ref().is_some_and(|p| browser_chromium_present(p));
    json!({
        "bundled": bundled,
        "chromium_ready": chromium_ready,
        "bundle_path": bundle.as_ref().map(|p| p.display().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_read_browser_enabled() {
        let mut cfg = json!({ "provider": "z.ai" });
        assert!(!read_browser_enabled(&cfg));
        set_browser_enabled(&mut cfg, true);
        assert!(read_browser_enabled(&cfg));
        set_browser_enabled(&mut cfg, false);
        assert!(!read_browser_enabled(&cfg));
    }
}
