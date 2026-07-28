//! Persistent LAN instance identity (`~/.anycode/lan/instance.json`).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanInstance {
    pub instance_id: String,
    #[serde(default = "default_device_name")]
    pub device_name: String,
    #[serde(default = "chrono_now")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanSettings {
    #[serde(default = "default_true")]
    pub discovery_enabled: bool,
    #[serde(default = "default_device_name")]
    pub display_name: String,
    #[serde(default = "default_lan_port")]
    pub lan_port: u16,
    #[serde(default = "default_max_bundle_mb")]
    pub max_bundle_mb: u64,
}

impl Default for LanSettings {
    fn default() -> Self {
        Self {
            discovery_enabled: true,
            display_name: default_device_name(),
            lan_port: default_lan_port(),
            max_bundle_mb: default_max_bundle_mb(),
        }
    }
}

impl LanSettings {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("settings.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(data_dir).context("create lan data dir")?;
        let path = data_dir.join("settings.json");
        let body = serde_json::to_string_pretty(self).context("serialize lan settings")?;
        std::fs::write(path, body).context("write lan settings")?;
        Ok(())
    }
}

fn instance_path(data_dir: &Path) -> PathBuf {
    data_dir.join("instance.json")
}

pub fn load_or_create_instance(data_dir: &Path) -> LanInstance {
    let path = instance_path(data_dir);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(inst) = serde_json::from_str(&text) {
            return inst;
        }
    }
    let inst = LanInstance {
        instance_id: format!("lan_{}", Uuid::new_v4().simple()),
        device_name: default_device_name(),
        created_at: chrono_now(),
    };
    let _ = save_instance(data_dir, &inst);
    inst
}

pub fn save_instance(data_dir: &Path, inst: &LanInstance) -> Result<()> {
    std::fs::create_dir_all(data_dir).context("create lan data dir")?;
    let body = serde_json::to_string_pretty(inst).context("serialize lan instance")?;
    std::fs::write(instance_path(data_dir), body).context("write lan instance")?;
    Ok(())
}

fn default_device_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "AnyCode".into())
}

fn default_lan_port() -> u16 {
    43181
}

fn default_max_bundle_mb() -> u64 {
    500
}

fn default_true() -> bool {
    true
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}
