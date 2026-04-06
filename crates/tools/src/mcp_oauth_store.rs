//! 将 `rmcp` 的 [`StoredCredentials`] 持久化为 JSON 文件；刷新 access token 时由 `AuthorizationManager` 写回同一文件。

use async_trait::async_trait;
use rmcp::transport::auth::{AuthError, CredentialStore, StoredCredentials};
use std::path::PathBuf;

/// 单文件 JSON 凭证存储（权限在 Unix 上尽量设为 `0600`）。
#[derive(Debug, Clone)]
pub struct JsonFileCredentialStore {
    path: PathBuf,
}

impl JsonFileCredentialStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl CredentialStore for JsonFileCredentialStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, AuthError> {
        match tokio::fs::read_to_string(&self.path).await {
            Ok(s) => {
                let s = s.trim();
                if s.is_empty() {
                    return Ok(None);
                }
                let v = serde_json::from_str::<StoredCredentials>(s).map_err(|e| {
                    AuthError::InternalError(format!("oauth credentials JSON: {e}"))
                })?;
                Ok(Some(v))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AuthError::InternalError(format!(
                "read {}: {e}",
                self.path.display()
            ))),
        }
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), AuthError> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                AuthError::InternalError(format!("create_dir_all {}: {e}", parent.display()))
            })?;
        }
        let json = serde_json::to_string_pretty(&credentials)
            .map_err(|e| AuthError::InternalError(format!("serialize StoredCredentials: {e}")))?;
        let tmp = self.path.with_extension("json.tmp");
        tokio::fs::write(&tmp, json.as_bytes())
            .await
            .map_err(|e| AuthError::InternalError(format!("write {}: {e}", tmp.display())))?;
        tokio::fs::rename(&tmp, &self.path).await.map_err(|e| {
            AuthError::InternalError(format!("rename to {}: {e}", self.path.display()))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            tokio::fs::set_permissions(&self.path, perms)
                .await
                .map_err(|e| {
                    AuthError::InternalError(format!("chmod {}: {e}", self.path.display()))
                })?;
        }

        Ok(())
    }

    async fn clear(&self) -> Result<(), AuthError> {
        match tokio::fs::remove_file(&self.path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(AuthError::InternalError(format!(
                "remove {}: {e}",
                self.path.display()
            ))),
        }
    }
}
