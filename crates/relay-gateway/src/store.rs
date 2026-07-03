use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_AGNES_CHAT_URL: &str = "https://apihub.agnes-ai.com/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayAccount {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default = "default_active")]
    pub status: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_active() -> String {
    "active".into()
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayModel {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_model")]
    pub default_model: String,
    /// `round_robin` | `weighted` | `pinned`
    #[serde(default = "default_rotation_mode")]
    pub rotation_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_account_id: Option<String>,
}

fn default_rotation_mode() -> String {
    "round_robin".into()
}

fn default_enabled() -> bool {
    true
}

fn default_model() -> String {
    "agnes-chat".into()
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            default_model: default_model(),
            rotation_mode: default_rotation_mode(),
            selected_account_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RelayStore {
    #[serde(default)]
    pub accounts: Vec<RelayAccount>,
    #[serde(default)]
    pub config: RelayConfig,
    #[serde(default)]
    pub models: Vec<RelayModel>,
}

pub fn relay_path() -> PathBuf {
    if let Ok(path) = std::env::var("ANYCODE_RELAY_PATH") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".anycode").join("relay.json"))
        .unwrap_or_else(|_| PathBuf::from(".anycode/relay.json"))
}

pub fn load_relay_store() -> RelayStore {
    let path = relay_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return RelayStore::default(),
    };
    let mut store: RelayStore = serde_json::from_str(&text).unwrap_or_default();
    normalize_relay_config(&mut store.config);
    store
}

/// Backfill rotation mode for legacy relay.json without `rotation_mode`.
pub fn normalize_relay_config(config: &mut RelayConfig) {
    if config.rotation_mode.trim().is_empty() {
        config.rotation_mode = infer_rotation_mode(config);
    }
}

fn infer_rotation_mode(config: &RelayConfig) -> String {
    match config.selected_account_id.as_deref() {
        Some(id) if !id.is_empty() && id != "auto" => "pinned".into(),
        _ => "round_robin".into(),
    }
}

pub fn save_relay_store(store: &RelayStore) -> Result<PathBuf> {
    let path = relay_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create relay config directory")?;
    }
    let mut to_save = store.clone();
    normalize_relay_config(&mut to_save.config);
    let body = serde_json::to_string_pretty(&to_save).context("serialize relay store")?;
    std::fs::write(&path, body).context("write relay store")?;
    Ok(path)
}

/// Merge incoming accounts with stored keys when client sends masked placeholders.
pub fn merge_accounts_preserving_keys(
    incoming: Vec<RelayAccount>,
    existing: &[RelayAccount],
) -> Vec<RelayAccount> {
    incoming
        .into_iter()
        .map(|mut acc| {
            if acc.api_key.as_deref() == Some("***") || acc.api_key.as_deref() == Some("") {
                if let Some(prev) = existing.iter().find(|e| e.id == acc.id) {
                    acc.api_key = prev.api_key.clone();
                } else {
                    acc.api_key = None;
                }
            }
            acc
        })
        .collect()
}

pub fn public_account(account: &RelayAccount) -> RelayAccount {
    RelayAccount {
        id: account.id.clone(),
        name: account.name.clone(),
        api_key: account
            .api_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .map(|_| "***".into()),
        base_url: account.base_url.clone(),
        status: account.status.clone(),
        weight: account.weight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_store_roundtrip_defaults() {
        let store = RelayStore::default();
        let json = serde_json::to_string(&store).unwrap();
        let parsed: RelayStore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.config.default_model, "agnes-chat");
        assert!(parsed.accounts.is_empty());
        assert!(parsed.models.is_empty());
    }

    #[test]
    fn merge_preserves_api_key_on_mask() {
        let existing = vec![RelayAccount {
            id: "a1".into(),
            name: "Pool".into(),
            api_key: Some("sk-secret".into()),
            base_url: None,
            status: "active".into(),
            weight: 1,
        }];
        let incoming = vec![RelayAccount {
            id: "a1".into(),
            name: "Pool".into(),
            api_key: Some("***".into()),
            base_url: None,
            status: "active".into(),
            weight: 1,
        }];
        let merged = merge_accounts_preserving_keys(incoming, &existing);
        assert_eq!(merged[0].api_key.as_deref(), Some("sk-secret"));
    }

    #[test]
    fn infer_pinned_rotation_from_legacy_config() {
        let mut cfg = RelayConfig {
            enabled: true,
            default_model: "agnes-chat".into(),
            rotation_mode: String::new(),
            selected_account_id: Some("acct-1".into()),
        };
        normalize_relay_config(&mut cfg);
        assert_eq!(cfg.rotation_mode, "pinned");
    }
}
