//! Built-in agent plugins — system prompt overlays loaded from disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Declarative plugin manifest (`plugin.json` on disk or built-in).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_overlay: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginsState {
    #[serde(default)]
    pub enabled: HashMap<String, bool>,
}

pub fn anycode_home() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".anycode"))
        .unwrap_or_else(|| PathBuf::from(".anycode"))
}

pub fn plugins_state_path() -> PathBuf {
    anycode_home().join("plugins-state.json")
}

pub fn load_plugins_state() -> PluginsState {
    let path = plugins_state_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return PluginsState::default(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_plugins_state(state: &PluginsState) -> std::io::Result<()> {
    let path = plugins_state_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(state)?;
    std::fs::write(path, body)
}

fn apply_state(mut plugins: Vec<PluginManifest>) -> Vec<PluginManifest> {
    let state = load_plugins_state();
    for p in &mut plugins {
        if let Some(enabled) = state.enabled.get(&p.id) {
            p.enabled = *enabled;
        }
    }
    plugins.sort_by_key(|p| (p.priority, p.id.clone()));
    plugins
}

fn read_plugin_file(path: &Path) -> Option<PluginManifest> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut manifest: PluginManifest = serde_json::from_str(&text).ok()?;
    if manifest.system_prompt_overlay.is_none() {
        let overlay = path.parent()?.join("overlay.md");
        if overlay.is_file() {
            manifest.system_prompt_overlay = std::fs::read_to_string(overlay).ok();
        }
    }
    Some(manifest)
}

fn scan_plugins_dir(dir: &Path, out: &mut Vec<PluginManifest>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let manifest = path.join("plugin.json");
            if manifest.is_file() {
                if let Some(p) = read_plugin_file(&manifest) {
                    out.push(p);
                }
            }
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("plugin.json") {
            if let Some(p) = read_plugin_file(&path) {
                out.push(p);
            }
        }
    }
}

/// Load `~/.anycode/plugins` + workspace `.anycode/plugins` manifests.
///
/// No plugins ship built into the runtime anymore (channel overlays were
/// removed), so this is a plain disk scan with state applied — earlier wins
/// on duplicate ids.
#[must_use]
pub fn load_plugins(workspace: Option<&Path>) -> Vec<PluginManifest> {
    let mut disk: Vec<PluginManifest> = Vec::new();
    scan_plugins_dir(&anycode_home().join("plugins"), &mut disk);
    if let Some(ws) = workspace {
        scan_plugins_dir(&ws.join(".anycode").join("plugins"), &mut disk);
    }
    let mut seen: HashMap<String, ()> = HashMap::new();
    disk.retain(|p| seen.insert(p.id.clone(), ()).is_none());
    apply_state(disk)
}

/// Back-compat alias used by system prompt composition.
#[must_use]
pub fn load_builtin_plugins() -> Vec<PluginManifest> {
    load_plugins(None)
}

pub fn set_plugin_enabled(id: &str, enabled: bool) -> std::io::Result<()> {
    let mut state = load_plugins_state();
    state.enabled.insert(id.to_string(), enabled);
    save_plugins_state(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_dirs_yield_empty() {
        // isolate from the real ~/.anycode/plugins
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());
        let plugins = load_plugins(Some(Path::new("/nonexistent-workspace")));
        assert!(plugins.is_empty());
    }
}
