use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_timezone() -> String {
    "Asia/Shanghai".to_string()
}

fn default_max_rows() -> usize {
    500
}

fn default_http_endpoint() -> String {
    "http://127.0.0.1:5030".to_string()
}

/// Backend kind for local WeChat chat history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WechatHistoryBackendKind {
    /// Direct in-process read via SQLCipher key map (default; no HTTP port).
    #[default]
    SqlcipherKeyMap,
    ChatlogHttp,
    SqlitePlain,
    /// Resolve at runtime: key map → sqlcipher, else data dir → sqlite_plain, else chatlog HTTP.
    Auto,
}

impl WechatHistoryBackendKind {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "chatlog_http" | "chatlog-http" | "http" => Some(Self::ChatlogHttp),
            "sqlite_plain" | "sqlite-plain" | "sqlite" => Some(Self::SqlitePlain),
            "sqlcipher_key_map" | "sqlcipher-key-map" | "sqlcipher" => Some(Self::SqlcipherKeyMap),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }

    pub fn resolve_auto(config: &WechatHistoryConfig) -> Self {
        if config.key_map_path.is_some() && config.data_dir.is_some() {
            Self::SqlcipherKeyMap
        } else if config.data_dir.is_some() {
            Self::SqlitePlain
        } else {
            Self::ChatlogHttp
        }
    }
}

/// Runtime configuration for querying local WeChat chat history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatHistoryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub backend: WechatHistoryBackendKind,
    /// Root directory for SQLite / SQLCipher databases (WeChat data dir or export dir).
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
    /// Per-database SQLCipher key map JSON (path → hex key).
    #[serde(default)]
    pub key_map_path: Option<PathBuf>,
    /// chatlog-compatible HTTP base URL (default `http://127.0.0.1:5030`).
    #[serde(default = "default_http_endpoint")]
    pub http_endpoint: String,
    #[serde(default = "default_timezone")]
    pub default_timezone: String,
    #[serde(default = "default_max_rows")]
    pub max_rows_per_query: usize,
}

impl Default for WechatHistoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: WechatHistoryBackendKind::default(),
            data_dir: None,
            key_map_path: None,
            http_endpoint: default_http_endpoint(),
            default_timezone: default_timezone(),
            max_rows_per_query: default_max_rows(),
        }
    }
}

impl WechatHistoryConfig {
    pub fn effective_limit(&self, requested: Option<usize>) -> usize {
        requested
            .unwrap_or(self.max_rows_per_query)
            .min(self.max_rows_per_query.max(1))
    }
}
