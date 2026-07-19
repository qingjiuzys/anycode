//! Built-in native CDP browser + legacy Playwright MCP bundle detection.

use anycode_browser::{chromium_doctor_message, resolve_chromium_executable};
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

pub fn native_chromium_ready() -> bool {
    resolve_chromium_executable().is_some()
}

/// True when the user can turn on built-in browser automation (bundled Chromium or system Chrome).
pub fn browser_automation_can_enable() -> bool {
    resolve_browser_mcp_bundle_root()
        .as_ref()
        .is_some_and(|p| is_browser_bundle(p) && browser_chromium_present(p))
        || native_chromium_ready()
}

/// Actionable message when Chromium is missing (workbench panel + agent tools).
pub fn browser_unavailable_message() -> String {
    if native_chromium_ready() {
        return chromium_doctor_message();
    }
    if let Some(root) = resolve_browser_mcp_bundle_root().filter(|p| is_browser_bundle(p)) {
        if browser_chromium_present(&root) {
            return format!(
                "Chromium bundle at {} but executable not resolved. Set ANYCODE_CHROMIUM_PATH or run scripts/prepare-chromium.sh.",
                root.display()
            );
        }
    }
    format!(
        "{} Run scripts/prepare-browser-mcp.sh or install Google Chrome.",
        chromium_doctor_message()
    )
}

pub fn read_browser_enabled(cfg: &Value) -> bool {
    if let Some(v) = cfg
        .get("browser")
        .and_then(|b| b.get("enabled"))
        .and_then(|v| v.as_bool())
    {
        return v;
    }
    cfg.get(CONFIG_KEY)
        .and_then(|m| m.get("browser"))
        .and_then(|b| b.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn set_browser_enabled(cfg: &mut Value, enabled: bool) {
    let root = cfg.as_object_mut().expect("config root must be object");
    root.insert(
        "browser".into(),
        json!({ "enabled": enabled, "native": true }),
    );
    let mcp = root
        .entry(CONFIG_KEY)
        .or_insert_with(|| json!({ "browser": { "enabled": false } }));
    if let Some(obj) = mcp.as_object_mut() {
        obj.insert("browser".into(), json!({ "enabled": enabled }));
    }
}

pub fn browser_connector_doctor_check(enabled: bool) -> crate::schema::DoctorCheck {
    if !enabled {
        return crate::schema::DoctorCheck {
            id: "browser_connector".into(),
            status: "ok".into(),
            message: "Built-in browser disabled".into(),
        };
    }
    if native_chromium_ready() {
        return crate::schema::DoctorCheck {
            id: "browser_connector".into(),
            status: "ok".into(),
            message: format!("Native CDP browser ready. {}", chromium_doctor_message()),
        };
    }
    let bundle = resolve_browser_mcp_bundle_root();
    if let Some(root) = bundle.filter(|p| is_browser_bundle(p)) {
        if browser_chromium_present(&root) {
            return crate::schema::DoctorCheck {
                id: "browser_connector".into(),
                status: "warn".into(),
                message: format!(
                    "Chromium bundle at {} but ANYCODE_CHROMIUM_PATH not resolved. Run scripts/prepare-chromium.sh.",
                    root.display()
                ),
            };
        }
    }
    crate::schema::DoctorCheck {
        id: "browser_connector".into(),
        status: "error".into(),
        message: format!(
            "Built-in browser enabled but Chromium not found. {}",
            chromium_doctor_message()
        ),
    }
}

pub fn browser_connector_status() -> Value {
    let bundle = resolve_browser_mcp_bundle_root();
    let bundled = bundle.as_ref().is_some_and(|p| is_browser_bundle(p)) || native_chromium_ready();
    let chromium_ready =
        native_chromium_ready() || bundle.as_ref().is_some_and(|p| browser_chromium_present(p));
    let chromium_path = resolve_chromium_executable().map(|p| p.display().to_string());
    json!({
        "bundled": bundled,
        "chromium_ready": chromium_ready,
        "native": true,
        "bundle_path": bundle.as_ref().map(|p| p.display().to_string()),
        "chromium_path": chromium_path,
        "mcp_browser_deprecated": true,
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

    #[test]
    fn browser_automation_can_enable_with_native_chrome_only() {
        if native_chromium_ready() {
            assert!(browser_automation_can_enable());
        }
    }
}
