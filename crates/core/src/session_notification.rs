//! 会话外向通知（HTTP / shell），与记忆管线 `hook_after_*` 解耦：用于 OpenClaw 类网关或自定义脚本。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

fn default_http_timeout_ms() -> u64 {
    5000
}

fn default_shell_timeout_ms() -> u64 {
    5000
}

fn default_max_body_bytes() -> usize {
    4096
}

/// `config.json` 的 `notifications` 段（与 `memory.pipeline.hook_*` 独立）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNotificationSettings {
    /// 是否在每条工具结果后触发（需配置 `http_url` 或 `shell_command` 之一）。
    #[serde(default = "default_true")]
    pub after_tool_result: bool,
    /// 是否在本轮 assistant 结束且无后续 tool_calls 时触发。
    #[serde(default = "default_true")]
    pub after_agent_turn: bool,
    #[serde(default)]
    pub http_url: Option<String>,
    #[serde(default = "default_http_timeout_ms")]
    pub http_timeout_ms: u64,
    /// 请求头；值中 `${VAR}` 由环境变量展开（未设置则替换为空串）。
    #[serde(default)]
    pub http_headers: HashMap<String, String>,
    /// 由 `/bin/sh -c`（Unix）或 `cmd /C`（Windows）执行；**JSON 载荷写入进程 stdin**（UTF-8）。
    #[serde(default)]
    pub shell_command: Option<String>,
    #[serde(default = "default_shell_timeout_ms")]
    pub shell_timeout_ms: u64,
    /// 正文 `excerpt` 上限；加载配置时校验在 **256..=524288**（见 CLI `validate_notifications`）。
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,
    /// 工具名前缀命中则跳过（与 memory pipeline 钩子语义一致，可单独留空表示不筛）。
    #[serde(default)]
    pub tool_deny_prefixes: Vec<String>,
}

impl Default for SessionNotificationSettings {
    fn default() -> Self {
        Self {
            after_tool_result: true,
            after_agent_turn: true,
            http_url: None,
            http_timeout_ms: default_http_timeout_ms(),
            http_headers: HashMap::new(),
            shell_command: None,
            shell_timeout_ms: default_shell_timeout_ms(),
            max_body_bytes: default_max_body_bytes(),
            tool_deny_prefixes: Vec::new(),
        }
    }
}

impl SessionNotificationSettings {
    /// 是否配置了至少一种投递方式且非空。
    pub fn is_configured(&self) -> bool {
        let http = self
            .http_url
            .as_deref()
            .map(str::trim)
            .is_some_and(|s| !s.is_empty());
        let shell = self
            .shell_command
            .as_deref()
            .map(str::trim)
            .is_some_and(|s| !s.is_empty());
        http || shell
    }
}
