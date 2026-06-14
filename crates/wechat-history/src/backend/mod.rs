mod attachment_resolver;
mod chatlog_http;
mod sqlcipher_key_map;
mod sqlite_cipher;
mod sqlite_plain;
mod sqlite_shared;
mod sqlite_snapshot;

pub(crate) use sqlite_cipher::{cipher_pragmas_for_key, run_sqlcipher_json};
pub(crate) use sqlite_shared::discover_sqlite_files;
pub(crate) use sqlite_snapshot::SnapshotDb;

pub use chatlog_http::ChatlogHttpBackend;
pub use sqlcipher_key_map::SqlcipherKeyMapBackend;
pub use sqlite_plain::SqlitePlainBackend;

use crate::config::{WechatHistoryBackendKind, WechatHistoryConfig};
use crate::model::{WechatHistoryQuery, WechatHistoryResult};
use crate::{Result, WechatHistoryError};

pub trait WechatHistoryBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn query(
        &self,
        query: &WechatHistoryQuery,
        config: &WechatHistoryConfig,
    ) -> Result<WechatHistoryResult>;
}

pub fn build_backend(config: &WechatHistoryConfig) -> Result<Box<dyn WechatHistoryBackend>> {
    if !config.enabled {
        return Err(WechatHistoryError::Disabled);
    }
    let backend = match config.backend {
        WechatHistoryBackendKind::Auto => WechatHistoryBackendKind::resolve_auto(config),
        other => other,
    };
    match backend {
        WechatHistoryBackendKind::ChatlogHttp => Ok(Box::new(ChatlogHttpBackend::new(
            config.http_endpoint.clone(),
        ))),
        WechatHistoryBackendKind::SqlitePlain => {
            let data_dir = config.data_dir.clone().ok_or_else(|| {
                WechatHistoryError::BackendNotConfigured(
                    "sqlite_plain requires wechatHistory.dataDir".into(),
                )
            })?;
            Ok(Box::new(SqlitePlainBackend::new(data_dir)))
        }
        WechatHistoryBackendKind::SqlcipherKeyMap => {
            let data_dir = config.data_dir.clone().ok_or_else(|| {
                WechatHistoryError::BackendNotConfigured(
                    "sqlcipher_key_map requires wechatHistory.dataDir".into(),
                )
            })?;
            let key_map_path = config.key_map_path.clone().ok_or_else(|| {
                WechatHistoryError::BackendNotConfigured(
                    "sqlcipher_key_map requires wechatHistory.keyMapPath; run: anycode wechat history setup"
                        .into(),
                )
            })?;
            Ok(Box::new(SqlcipherKeyMapBackend::new(
                data_dir,
                key_map_path,
            )))
        }
        WechatHistoryBackendKind::Auto => {
            unreachable!("backend::Auto must be resolved before build_backend match")
        }
    }
}
