//! SecretRef - 敏感信息引用系统
//!
//! 支持多种敏感信息来源：环境变量、文件、提供商凭证等
//! 与 OpenClaw 的 SecretRef 系统对齐

use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 敏感信息引用类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecretRef {
    /// 直接字符串（不推荐，仅用于向后兼容）
    #[serde(rename = "direct")]
    Direct(String),

    /// 环境变量引用
    #[serde(rename = "env")]
    EnvVar(String),

    /// 文件路径引用
    #[serde(rename = "file")]
    File(PathBuf),

    /// 提供商凭证引用
    #[serde(rename = "provider_credential")]
    ProviderCredential { provider: String, key: String },
}

impl SecretRef {
    /// 从字符串自动检测类型
    pub fn from_string(s: &str) -> Self {
        let trimmed = s.trim();

        // 检查环境变量引用 ${VAR_NAME}
        if trimmed.starts_with("${") && trimmed.ends_with('}') {
            let var_name = trimmed[2..trimmed.len() - 1].to_string();
            return SecretRef::EnvVar(var_name);
        }

        // 检查文件引用 @path
        if let Some(rest) = trimmed.strip_prefix('@') {
            let path = rest.to_string();
            return SecretRef::File(PathBuf::from(path));
        }

        // 默认为直接字符串
        SecretRef::Direct(trimmed.to_string())
    }

    /// 获取引用的提示文本
    pub fn hint(&self) -> String {
        match self {
            SecretRef::Direct(_) => "直接输入的敏感信息".to_string(),
            SecretRef::EnvVar(var) => format!("环境变量: {}", var),
            SecretRef::File(path) => format!("文件: {}", path.display()),
            SecretRef::ProviderCredential { provider, key } => {
                format!("提供商凭证: {} / {}", provider, key)
            }
        }
    }

    /// 判断是否为空引用
    pub fn is_empty(&self) -> bool {
        match self {
            SecretRef::Direct(s) => s.trim().is_empty(),
            SecretRef::EnvVar(s) => s.trim().is_empty(),
            SecretRef::File(_) => false, // 文件路径即使存在也算非空
            SecretRef::ProviderCredential { provider, key } => {
                provider.trim().is_empty() || key.trim().is_empty()
            }
        }
    }
}

impl Default for SecretRef {
    fn default() -> Self {
        SecretRef::Direct(String::new())
    }
}

/// 敏感信息解析器
pub struct SecretResolver {
    config_dir: PathBuf,
    runtime_secrets: HashMap<String, String>,
}

impl SecretResolver {
    /// 创建新的解析器
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            runtime_secrets: HashMap::new(),
        }
    }

    /// 设置运行时密钥
    pub fn set_runtime_secret(&mut self, key: String, value: String) {
        self.runtime_secrets.insert(key, value);
    }

    /// 解析 SecretRef 为实际值
    pub fn resolve(&self, secret_ref: &SecretRef) -> Result<String, CoreError> {
        match secret_ref {
            SecretRef::Direct(value) => Ok(value.clone()),
            SecretRef::EnvVar(var_name) => {
                // 优先检查运行时密钥
                if let Some(value) = self.runtime_secrets.get(var_name) {
                    return Ok(value.clone());
                }

                // 然后检查环境变量
                std::env::var(var_name)
                    .map_err(|_| CoreError::ConfigError(format!("环境变量 '{}' 未设置", var_name)))
            }
            SecretRef::File(path) => {
                let full_path = if path.is_absolute() {
                    path.clone()
                } else {
                    self.config_dir.join(path)
                };

                std::fs::read_to_string(&full_path)
                    .map(|s| s.trim().to_string())
                    .map_err(|e| {
                        CoreError::ConfigError(format!(
                            "无法读取密钥文件 '{}': {}",
                            full_path.display(),
                            e
                        ))
                    })
            }
            SecretRef::ProviderCredential { provider, key } => {
                let env_key = format!("{}_{}", provider.to_uppercase(), key.to_uppercase());
                std::env::var(&env_key)
                    .or_else(|_| {
                        std::env::var(format!("{}{}", provider.to_uppercase(), key.to_uppercase()))
                    })
                    .map_err(|_| {
                        CoreError::ConfigError(format!(
                            "提供商凭证未找到: 尝试环境变量 '{}'",
                            env_key
                        ))
                    })
            }
        }
    }

    /// 从配置中解析 API 密钥
    pub fn resolve_api_key(
        &self,
        provider: &str,
        provider_credentials: &HashMap<String, String>,
        default_api_key: &str,
    ) -> Result<String, CoreError> {
        // 1. 优先使用提供商专用凭证
        if let Some(key) = provider_credentials.get(provider) {
            if !key.trim().is_empty() {
                let secret_ref = SecretRef::from_string(key);
                return self.resolve(&secret_ref);
            }
        }

        // 2. 使用默认 API 密钥
        let default_ref = SecretRef::from_string(default_api_key);
        let resolved = self.resolve(&default_ref)?;

        if resolved.trim().is_empty() {
            return Err(CoreError::ConfigError(format!(
                "提供商 '{}' 的 API 密钥为空",
                provider
            )));
        }

        Ok(resolved)
    }

    /// 批量解析多个 SecretRef
    pub fn resolve_batch(&self, secret_refs: &[SecretRef]) -> Result<Vec<String>, CoreError> {
        secret_refs.iter().map(|ref_| self.resolve(ref_)).collect()
    }
}

impl Default for SecretResolver {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_ref_from_string_env_var() {
        let ref_ = SecretRef::from_string("${API_KEY}");
        assert!(matches!(ref_, SecretRef::EnvVar(_)));
        assert_eq!(ref_.hint(), "环境变量: API_KEY");
    }

    #[test]
    fn test_secret_ref_from_string_file() {
        let ref_ = SecretRef::from_string("@./secrets/key.txt");
        assert!(matches!(ref_, SecretRef::File(_)));
        assert!(ref_.hint().contains("文件"));
    }

    #[test]
    fn test_secret_ref_from_string_direct() {
        let ref_ = SecretRef::from_string("sk-test-123");
        assert!(matches!(ref_, SecretRef::Direct(_)));
        assert_eq!(ref_.hint(), "直接输入的敏感信息");
    }

    #[test]
    fn test_secret_resolver_env_var() {
        std::env::set_var("TEST_ANYCODE_VAR", "test_value");

        let resolver = SecretResolver::new(PathBuf::from("."));
        let ref_ = SecretRef::EnvVar("TEST_ANYCODE_VAR".to_string());
        assert_eq!(resolver.resolve(&ref_).unwrap(), "test_value");

        std::env::remove_var("TEST_ANYCODE_VAR");
    }

    #[test]
    fn test_secret_resolver_runtime_secret() {
        let mut resolver = SecretResolver::new(PathBuf::new());
        resolver.set_runtime_secret("runtime_key".to_string(), "runtime_value".to_string());

        let ref_ = SecretRef::EnvVar("runtime_key".to_string());
        assert_eq!(resolver.resolve(&ref_).unwrap(), "runtime_value");
    }

    #[test]
    fn test_secret_ref_provider_credential() {
        let ref_ = SecretRef::ProviderCredential {
            provider: "anthropic".to_string(),
            key: "api_key".to_string(),
        };

        assert_eq!(ref_.hint(), "提供商凭证: anthropic / api_key");
    }

    #[test]
    fn test_secret_ref_is_empty() {
        assert!(SecretRef::Direct(String::new()).is_empty());
        assert!(SecretRef::Direct("  ".to_string()).is_empty());
        assert!(!SecretRef::Direct("key".to_string()).is_empty());
        assert!(SecretRef::EnvVar("".to_string()).is_empty());
        assert!(!SecretRef::File(PathBuf::from("test")).is_empty());
    }
}
