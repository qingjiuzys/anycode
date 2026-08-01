//! 运维层（对齐 Claude Code 2.1.218 多存储同步/导出语义）。
//!
//! 二进制提取语义：
//! - `tengu_memory_stream_list` / `team_memory_multistore_stream_list`：流式列表遥测。
//! - `streamListSuccessStreak` / `streamListFailureStreak` / `streamListRetryAfterMs`：
//!   流式列表连续成功/失败计数与退避重试间隔（毫秒）。
//! - `tengu_memory_bulk_inflate` / `team_memory_multistore_bulk_inflate`：批量解包遥测，
//!   失败原因含 `not-attempted` / `not_found` / `http_error` / `bulk inflate unavailable` / `fell-back`。
//! - `tengu_memory_store_resync_interval_minutes`：store 周期性重同步间隔（分钟），
//!   由 `CLAUDE_CODE_DISABLE_MEMORY_PERIODIC_RESYNC` 关闭。
//! - `tengu_memory_threshold_crossed`：RSS/堆内存阈值事件（`rss_mb` / `heap_used_mb`，级别 normal|high|critical）。
//! - NDJSON 导出行类型：`store` / `memory` / `memory_error` / `complete`（complete 含 `memory_count` / `error_count`），
//!   memory 行含 `id` / `path` / `content` / `content_sha256` / `updated_at`；
//!   校验失败原因：`parse_failed` / `oversized_line` / `too_many_entries` / `stream_error` /
//!   `write_failed` / `stream_truncated` / `count_mismatch` / `decrypt_errors`。

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 周期性重同步开关（对齐 Claude `CLAUDE_CODE_DISABLE_MEMORY_PERIODIC_RESYNC`）。
pub const ENV_DISABLE_MEMORY_PERIODIC_RESYNC: &str = "CLAUDE_CODE_DISABLE_MEMORY_PERIODIC_RESYNC";
/// 重同步间隔环境变量（分钟，对齐 `tengu_memory_store_resync_interval_minutes`）。
pub const ENV_RESYNC_INTERVAL_MINUTES: &str = "tengu_memory_store_resync_interval_minutes";
/// 默认重同步间隔（分钟）。
pub const DEFAULT_RESYNC_INTERVAL_MINUTES: u64 = 60;
/// 单行导出最大长度（对齐 `oversized_line` 校验）。
pub const MAX_EXPORT_LINE_BYTES: usize = 4 * 1024 * 1024;

/// NDJSON 导出行（对齐 Claude 多存储导出行格式）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportLine {
    /// 起始 store 元数据行。
    Store {
        #[serde(default)]
        store_id: String,
        #[serde(default)]
        name: String,
    },
    /// 单条记忆行。
    Memory {
        id: String,
        path: String,
        content: String,
        content_sha256: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    },
    /// 解密/读取失败行（统计 `error_count`）。
    MemoryError {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        error: String,
    },
    /// 流结束汇总行。
    Complete {
        memory_count: u64,
        #[serde(default)]
        error_count: u64,
    },
}

/// 导出行解析/流校验失败原因（对齐 Claude 字符串 `parse_failed` 等）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportStreamFailure {
    ParseFailed,
    OversizedLine,
    TooManyEntries,
    StreamError,
    WriteFailed,
    StreamTruncated,
    CountMismatch,
    DecryptErrors,
}

impl ExportStreamFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseFailed => "parse_failed",
            Self::OversizedLine => "oversized_line",
            Self::TooManyEntries => "too_many_entries",
            Self::StreamError => "stream_error",
            Self::WriteFailed => "write_failed",
            Self::StreamTruncated => "stream_truncated",
            Self::CountMismatch => "count_mismatch",
            Self::DecryptErrors => "decrypt_errors",
        }
    }
}

/// 解析单行 NDJSON 导出；行过长返回 `OversizedLine`。
pub fn parse_export_line(line: &str) -> Result<ExportLine, ExportStreamFailure> {
    if line.len() > MAX_EXPORT_LINE_BYTES {
        return Err(ExportStreamFailure::OversizedLine);
    }
    serde_json::from_str(line).map_err(|_| ExportStreamFailure::ParseFailed)
}

/// 批量解包失败原因（对齐 Claude `bulk inflate unavailable` 分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkInflateFailure {
    /// 未尝试（功能关闭）。
    NotAttempted,
    /// 目标记忆不存在。
    NotFound,
    /// 远端 HTTP 错误。
    HttpError,
    /// 流不可用。
    Unavailable,
    /// 回退也失败（`fallback_failed`）。
    FallbackFailed,
    /// 写入本地失败。
    WriteFailed,
    /// 内容哈希与 `content_sha256` 不符。
    ShaMismatch,
}

impl BulkInflateFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotAttempted => "not-attempted",
            Self::NotFound => "not_found",
            Self::HttpError => "http_error",
            Self::Unavailable => "unavailable",
            Self::FallbackFailed => "fallback_failed",
            Self::WriteFailed => "write_failed",
            Self::ShaMismatch => "sha_mismatch",
        }
    }
}

/// 批量解包结果（对齐 `bulk inflated N file(s) from M exported memory line(s)`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkInflateOutcome {
    pub entries_listed: u64,
    pub files_written: u64,
    pub files_deleted: u64,
    pub files_skipped_concurrent: u64,
    pub error_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<BulkInflateFailure>,
}

impl BulkInflateOutcome {
    pub fn ok() -> Self {
        Self::default()
    }

    pub fn failed(failure: BulkInflateFailure) -> Self {
        Self {
            failure: Some(failure),
            ..Self::default()
        }
    }

    pub fn succeeded(&self) -> bool {
        self.failure.is_none()
    }
}

/// 流式列表健康状态（对齐 `streamListSuccessStreak` / `streamListFailureStreak` / `streamListRetryAfterMs`）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StreamListState {
    pub success_streak: u64,
    pub failure_streak: u64,
    /// 下次允许重试的时间点（基于 `retry_after_ms`）。
    pub retry_after: Option<std::time::Instant>,
}

impl StreamListState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次成功：失败连击清零，成功连击 +1。
    pub fn record_success(&mut self) {
        self.failure_streak = 0;
        self.success_streak += 1;
        self.retry_after = None;
    }

    /// 记录一次失败：成功连击清零，失败连击 +1，并按指数退避设置重试窗口。
    pub fn record_failure(&mut self, retry_after_ms: u64) {
        self.success_streak = 0;
        self.failure_streak += 1;
        let backoff = retry_after_ms.min(60_000) << self.failure_streak.min(5);
        self.retry_after = Some(std::time::Instant::now() + Duration::from_millis(backoff));
    }

    /// 当前是否处于退避窗口内（应暂缓下一次流式列表）。
    pub fn backing_off(&self) -> bool {
        match self.retry_after {
            Some(deadline) => std::time::Instant::now() < deadline,
            None => false,
        }
    }
}

/// 周期性重同步策略（对齐 `tengu_memory_store_resync_interval_minutes`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResyncPolicy {
    pub interval_minutes: u64,
    pub disabled: bool,
}

impl Default for ResyncPolicy {
    fn default() -> Self {
        Self {
            interval_minutes: DEFAULT_RESYNC_INTERVAL_MINUTES,
            disabled: false,
        }
    }
}

impl ResyncPolicy {
    /// 从环境变量构造：`CLAUDE_CODE_DISABLE_MEMORY_PERIODIC_RESYNC` 关闭；
    /// `tengu_memory_store_resync_interval_minutes` 覆盖间隔（分钟）。
    pub fn from_env() -> Self {
        let disabled = std::env::var(ENV_DISABLE_MEMORY_PERIODIC_RESYNC)
            .map(|v| !v.is_empty() && v != "0")
            .unwrap_or(false);
        let interval_minutes = std::env::var(ENV_RESYNC_INTERVAL_MINUTES)
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_RESYNC_INTERVAL_MINUTES);
        Self {
            interval_minutes,
            disabled,
        }
    }

    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_minutes * 60)
    }

    /// 是否应触发一次重同步（距离上次 >= 间隔且未禁用）。
    pub fn due(&self, last_resync: Option<chrono::DateTime<chrono::Utc>>) -> bool {
        if self.disabled {
            return false;
        }
        match last_resync {
            None => true,
            Some(last) => {
                let elapsed = chrono::Utc::now().signed_duration_since(last);
                elapsed.num_minutes() >= self.interval_minutes as i64
            }
        }
    }
}

/// 内存阈值级别（对齐 `tengu_memory_threshold_crossed`：normal|high|critical）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryThresholdLevel {
    Normal,
    High,
    Critical,
}

impl MemoryThresholdLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// 阈值越线事件（对齐 `tengu_memory_threshold_crossed`，字段 `rss_mb` / `heap_used_mb`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdCrossedEvent {
    pub level: MemoryThresholdLevel,
    pub rss_mb: u64,
    pub heap_used_mb: u64,
}

impl ThresholdCrossedEvent {
    /// 构造阈值事件；rss/heap 均低于阈值时归为 normal，任一超过 critical 阈值则 critical。
    pub fn from_usage(rss_mb: u64, heap_used_mb: u64, high_mb: u64, critical_mb: u64) -> Self {
        let level = if rss_mb >= critical_mb || heap_used_mb >= critical_mb {
            MemoryThresholdLevel::Critical
        } else if rss_mb >= high_mb || heap_used_mb >= high_mb {
            MemoryThresholdLevel::High
        } else {
            MemoryThresholdLevel::Normal
        };
        Self {
            level,
            rss_mb,
            heap_used_mb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ndjson_export_lines() {
        let store =
            parse_export_line(r#"{"type":"store","store_id":"team-a","name":"Team A"}"#).unwrap();
        assert!(matches!(store, ExportLine::Store { store_id, .. } if store_id == "team-a"));

        let mem = parse_export_line(
            r#"{"type":"memory","id":"m1","path":"user/m1.md","content":"hi","content_sha256":"abc"}"#,
        )
        .unwrap();
        assert!(matches!(mem, ExportLine::Memory { id, .. } if id == "m1"));

        let complete =
            parse_export_line(r#"{"type":"complete","memory_count":2,"error_count":1}"#).unwrap();
        assert!(
            matches!(complete, ExportLine::Complete { memory_count, error_count } if memory_count == 2 && error_count == 1)
        );
    }

    #[test]
    fn rejects_malformed_or_oversized_lines() {
        assert_eq!(
            parse_export_line("not json"),
            Err(ExportStreamFailure::ParseFailed)
        );
        let huge = format!(
            "{{\"type\":\"memory\",\"content\":\"{}\"}}",
            "x".repeat(5 * 1024 * 1024)
        );
        assert_eq!(
            parse_export_line(&huge),
            Err(ExportStreamFailure::OversizedLine)
        );
    }

    #[test]
    fn stream_list_backoff_tracks_streaks() {
        let mut s = StreamListState::new();
        assert!(!s.backing_off());
        s.record_success();
        s.record_success();
        assert_eq!(s.success_streak, 2);
        s.record_failure(1_000);
        assert_eq!(s.success_streak, 0);
        assert_eq!(s.failure_streak, 1);
        assert!(s.backing_off());
        s.record_success();
        assert!(!s.backing_off());
        assert_eq!(s.failure_streak, 0);
    }

    #[test]
    fn resync_policy_due_math_and_env_override() {
        // 未配置：从未重同步过 → 立即触发。
        let p = ResyncPolicy::default();
        assert!(p.due(None));
        // 上次 2 分钟前、间隔 60 → 未到期。
        let last = chrono::Utc::now() - chrono::Duration::minutes(2);
        assert!(!p.due(Some(last)));
        // 上次 61 分钟前 → 到期。
        let old = chrono::Utc::now() - chrono::Duration::minutes(61);
        assert!(p.due(Some(old)));

        // 环境变量覆盖间隔。
        unsafe {
            std::env::set_var(ENV_RESYNC_INTERVAL_MINUTES, "5");
            std::env::set_var(ENV_DISABLE_MEMORY_PERIODIC_RESYNC, "0");
        }
        let p = ResyncPolicy::from_env();
        assert_eq!(p.interval_minutes, 5);
        assert!(!p.disabled);
        // 间隔 5 分钟：3 分钟前重同步 → 未到期；6 分钟前 → 到期。
        let recent = chrono::Utc::now() - chrono::Duration::minutes(3);
        assert!(!p.due(Some(recent)));
        let overdue = chrono::Utc::now() - chrono::Duration::minutes(6);
        assert!(p.due(Some(overdue)));

        // 禁用后永不触发。
        unsafe {
            std::env::set_var(ENV_DISABLE_MEMORY_PERIODIC_RESYNC, "1");
        }
        let p = ResyncPolicy::from_env();
        assert!(p.disabled);
        assert!(!p.due(None));
        unsafe {
            std::env::remove_var(ENV_RESYNC_INTERVAL_MINUTES);
            std::env::remove_var(ENV_DISABLE_MEMORY_PERIODIC_RESYNC);
        }
    }

    #[test]
    fn threshold_level_selection() {
        let e = ThresholdCrossedEvent::from_usage(100, 80, 512, 1024);
        assert_eq!(e.level, MemoryThresholdLevel::Normal);
        let e = ThresholdCrossedEvent::from_usage(600, 80, 512, 1024);
        assert_eq!(e.level, MemoryThresholdLevel::High);
        let e = ThresholdCrossedEvent::from_usage(100, 1500, 512, 1024);
        assert_eq!(e.level, MemoryThresholdLevel::Critical);
        assert_eq!(e.rss_mb, 100);
        assert_eq!(e.heap_used_mb, 1500);
    }

    #[test]
    fn bulk_inflate_outcome_ok_and_failed() {
        let ok = BulkInflateOutcome::ok();
        assert!(ok.succeeded());
        let failed = BulkInflateOutcome::failed(BulkInflateFailure::HttpError);
        assert!(!failed.succeeded());
        assert_eq!(failed.failure, Some(BulkInflateFailure::HttpError));
    }
}
