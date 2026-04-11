//! 统一重试策略 - 与 Claude Code 对齐
//!
//! 支持指数退避、错误分类、Retry-After 头等

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// 基础延迟（毫秒）
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,

    /// 最大延迟（毫秒）
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// 指数退避基数
    #[serde(default = "default_exponential_base")]
    pub exponential_base: u64,

    /// 是否尊重 Retry-After 头
    #[serde(default = "default_respect_retry_after")]
    pub respect_retry_after: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 8,
            base_delay_ms: 500,
            max_delay_ms: 10000,
            exponential_base: 2,
            respect_retry_after: true,
        }
    }
}

fn default_max_retries() -> u32 {
    8
}
fn default_base_delay_ms() -> u64 {
    500
}
fn default_max_delay_ms() -> u64 {
    10_000
}
fn default_exponential_base() -> u64 {
    2
}
fn default_respect_retry_after() -> bool {
    true
}

/// 错误分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// 速率限制（429, 529）
    RateLimit,
    /// 认证失败（401, 403）
    AuthenticationFailed,
    /// 服务器错误（5xx）
    ServerError,
    /// 客户端错误（4xx，排除上述）
    ClientError,
    /// 网络错误
    NetworkError,
    /// 未知错误
    Unknown,
}

/// 重试策略
pub struct RetryStrategy {
    config: RetryConfig,
}

impl RetryStrategy {
    /// 创建新的重试策略
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// 分类错误
    pub fn categorize_error(&self, status: StatusCode, error_text: &str) -> ErrorCategory {
        // 速率限制
        if status == StatusCode::TOO_MANY_REQUESTS
            || status.as_u16() == 529
            || error_text.contains("overloaded_error")
            || error_text.contains("rate_limit")
        {
            return ErrorCategory::RateLimit;
        }

        // 认证失败
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return ErrorCategory::AuthenticationFailed;
        }

        // 服务器错误
        if status.is_server_error() {
            return ErrorCategory::ServerError;
        }

        // 客户端错误
        if status.is_client_error() {
            return ErrorCategory::ClientError;
        }

        // 网络错误（通过状态码无法判断，由调用方识别）
        ErrorCategory::Unknown
    }

    /// 判断是否应该重试
    pub fn should_retry(&self, category: ErrorCategory, attempt: u32) -> bool {
        match category {
            ErrorCategory::RateLimit => attempt <= self.config.max_retries,
            ErrorCategory::ServerError => attempt <= self.config.max_retries,
            ErrorCategory::NetworkError => attempt <= self.config.max_retries,
            ErrorCategory::AuthenticationFailed => false, // 认证失败不重试
            ErrorCategory::ClientError => false,          // 客户端错误不重试
            ErrorCategory::Unknown => attempt <= self.config.max_retries,
        }
    }

    /// 计算重试延迟
    pub fn compute_delay(&self, attempt: u32, retry_after_ms: Option<u64>) -> Duration {
        // 如果启用且提供了 Retry-After 头，优先使用
        if self.config.respect_retry_after {
            if let Some(ms) = retry_after_ms {
                return Duration::from_millis(ms.min(self.config.max_delay_ms));
            }
        }

        // 指数退避计算
        let exponential_delay = self.config.base_delay_ms
            * self
                .config
                .exponential_base
                .saturating_pow(attempt.saturating_sub(1));

        // 应用最大延迟限制
        let delay_ms = exponential_delay.min(self.config.max_delay_ms);

        Duration::from_millis(delay_ms)
    }

    /// 判断 HTTP 状态码是否可重试（向后兼容接口）
    pub fn is_retryable_status(&self, status: StatusCode) -> bool {
        let category = self.categorize_error(status, "");
        matches!(
            category,
            ErrorCategory::RateLimit | ErrorCategory::ServerError
        )
    }

    /// 获取配置
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// 创建带抖动的重试策略（避免惊群效应）
    pub fn with_jitter(&self) -> JitterRetryStrategy {
        JitterRetryStrategy::new(self.config.clone())
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::new(RetryConfig::default())
    }
}

/// 带抖动的重试策略
pub struct JitterRetryStrategy {
    config: RetryConfig,
    jitter_factor: f64, // 抖动因子 0.0 - 1.0
}

impl JitterRetryStrategy {
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            jitter_factor: 0.1, // 默认10%抖动
        }
    }

    pub fn with_jitter_factor(mut self, factor: f64) -> Self {
        self.jitter_factor = factor.clamp(0.0, 1.0);
        self
    }

    /// 计算带抖动的延迟
    pub fn compute_delay(&self, attempt: u32, retry_after_ms: Option<u64>) -> Duration {
        let base_strategy = RetryStrategy::new(self.config.clone());
        let base_delay = base_strategy.compute_delay(attempt, retry_after_ms);

        // 添加随机抖动
        let jitter_ms = (base_delay.as_millis() as f64 * self.jitter_factor) as u64;
        let random_jitter = (rand::random::<f64>() * 2.0 - 1.0) * jitter_ms as f64;
        let final_ms = (base_delay.as_millis() as f64 + random_jitter).max(0.0) as u64;

        Duration::from_millis(final_ms)
    }
}

/// 提供商特定的重试配置
pub struct ProviderRetryConfig {
    /// 提供商ID
    pub provider_id: String,
    /// 基础配置
    pub base_config: RetryConfig,
    /// 提供商特定的可重试状态码
    pub retryable_statuses: Vec<StatusCode>,
}

impl ProviderRetryConfig {
    /// 创建 Anthropic 提供商配置
    pub fn anthropic() -> Self {
        Self {
            provider_id: "anthropic".to_string(),
            base_config: RetryConfig {
                max_retries: 8,
                base_delay_ms: 500,
                max_delay_ms: 10000,
                exponential_base: 2,
                respect_retry_after: true,
            },
            retryable_statuses: vec![
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::SERVICE_UNAVAILABLE,
            ],
        }
    }

    /// 创建 OpenAI 提供商配置
    pub fn openai() -> Self {
        Self {
            provider_id: "openai".to_string(),
            base_config: RetryConfig {
                max_retries: 5,
                base_delay_ms: 1000,
                max_delay_ms: 15000,
                exponential_base: 2,
                respect_retry_after: true,
            },
            retryable_statuses: vec![
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::SERVICE_UNAVAILABLE,
            ],
        }
    }

    /// 创建 ZAI 提供商配置
    pub fn zai() -> Self {
        Self {
            provider_id: "z.ai".to_string(),
            base_config: RetryConfig {
                max_retries: 10,
                base_delay_ms: 500,
                max_delay_ms: 10000,
                exponential_base: 2,
                respect_retry_after: true,
            },
            retryable_statuses: vec![
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::BAD_GATEWAY,
            ],
        }
    }

    /// 创建通用提供商配置
    pub fn generic() -> Self {
        Self {
            provider_id: "generic".to_string(),
            base_config: RetryConfig::default(),
            retryable_statuses: vec![
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::SERVICE_UNAVAILABLE,
            ],
        }
    }

    /// 判断状态码是否可重试
    pub fn is_status_retryable(&self, status: StatusCode) -> bool {
        self.retryable_statuses.contains(&status) || status.is_server_error()
    }
}

/// 向后兼容函数：判断状态码是否可重试
pub fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::INTERNAL_SERVER_ERROR
    ) || status.is_server_error()
}

/// 向后兼容函数：计算重试延迟
pub fn retry_delay_ms(attempt: u32) -> u64 {
    const BASE: u64 = 500;
    const CAP: u64 = 10_000;
    let exp = 2u64.saturating_pow(attempt.saturating_sub(1));
    std::cmp::min(CAP, BASE.saturating_mul(exp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        let strategy = RetryStrategy::default();

        assert_eq!(
            strategy.categorize_error(StatusCode::TOO_MANY_REQUESTS, ""),
            ErrorCategory::RateLimit
        );
        assert_eq!(
            strategy.categorize_error(StatusCode::UNAUTHORIZED, ""),
            ErrorCategory::AuthenticationFailed
        );
        assert_eq!(
            strategy.categorize_error(StatusCode::INTERNAL_SERVER_ERROR, ""),
            ErrorCategory::ServerError
        );
        assert_eq!(
            strategy.categorize_error(StatusCode::BAD_REQUEST, ""),
            ErrorCategory::ClientError
        );
    }

    #[test]
    fn test_should_retry_logic() {
        let strategy = RetryStrategy::default();

        // 速率限制应该重试
        assert!(strategy.should_retry(ErrorCategory::RateLimit, 1));
        assert!(strategy.should_retry(ErrorCategory::RateLimit, 8));
        assert!(!strategy.should_retry(ErrorCategory::RateLimit, 9));

        // 认证失败不应该重试
        assert!(!strategy.should_retry(ErrorCategory::AuthenticationFailed, 1));
        assert!(!strategy.should_retry(ErrorCategory::AuthenticationFailed, 8));

        // 服务器错误应该重试
        assert!(strategy.should_retry(ErrorCategory::ServerError, 1));
        assert!(!strategy.should_retry(ErrorCategory::ServerError, 9));
    }

    #[test]
    fn test_exponential_backoff() {
        let strategy = RetryStrategy::default();

        let delays: Vec<_> = (1..=5).map(|i| strategy.compute_delay(i, None)).collect();

        // 验证指数增长
        assert!(delays[1].as_millis() > delays[0].as_millis());
        assert!(delays[2].as_millis() > delays[1].as_millis());
        assert!(delays[3].as_millis() > delays[2].as_millis());

        // 验证最大延迟限制
        for delay in delays {
            assert!(delay.as_millis() <= 10000);
        }
    }

    #[test]
    fn test_retry_after_header() {
        let config = RetryConfig {
            respect_retry_after: true,
            ..Default::default()
        };
        let strategy = RetryStrategy::new(config);

        let delay = strategy.compute_delay(1, Some(2000));
        assert_eq!(delay.as_millis(), 2000);

        // 验证最大延迟限制仍然适用
        let delay = strategy.compute_delay(1, Some(20000));
        assert_eq!(delay.as_millis(), 10000);
    }

    #[test]
    fn test_provider_specific_configs() {
        let anthropic = ProviderRetryConfig::anthropic();
        assert_eq!(anthropic.provider_id, "anthropic");
        assert_eq!(anthropic.base_config.max_retries, 8);

        let openai = ProviderRetryConfig::openai();
        assert_eq!(openai.provider_id, "openai");
        assert_eq!(openai.base_config.max_retries, 5);

        let zai = ProviderRetryConfig::zai();
        assert_eq!(zai.provider_id, "z.ai");
        assert_eq!(zai.base_config.max_retries, 10);
    }

    #[test]
    fn test_jitter_retry_strategy() {
        let config = RetryConfig::default();
        let jitter_strategy = JitterRetryStrategy::new(config).with_jitter_factor(0.2);

        let delay1 = jitter_strategy.compute_delay(2, None);
        let delay2 = jitter_strategy.compute_delay(2, None);

        // 由于抖动，两次计算结果应该不同
        assert_ne!(delay1.as_millis(), delay2.as_millis());

        // 但差异不应该太大
        let diff = (delay1.as_millis() as i64 - delay2.as_millis() as i64).abs();
        assert!(diff < 1000); // 差异小于1秒
    }

    #[test]
    fn test_backward_compatibility() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));

        assert_eq!(retry_delay_ms(1), 500);
        assert_eq!(retry_delay_ms(2), 1000);
        assert_eq!(retry_delay_ms(3), 2000);
        assert_eq!(retry_delay_ms(4), 4000);
        assert_eq!(retry_delay_ms(10), 10000); // 达到上限
    }
}
