//! 工具/Shell 安全策略（单一事实来源：`anycode-security` 与 `Tool` 元数据共用）。

use serde::{Deserialize, Serialize};

/// 安全策略 (来自 OpenClaw System Run Approvals；与运行时审批层共用)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityPolicy {
    /// 允许的命令模式（正则表达式字符串）
    pub allow_commands: Vec<String>,
    /// 拒绝的命令模式（正则表达式字符串）
    pub deny_commands: Vec<String>,
    /// 是否需要用户批准（无回调时由 `ApprovalSystem` 解释为自动通过）
    pub require_approval: bool,
    pub sandbox_mode: bool,
    /// 可选超时（毫秒），供工具执行参考
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_commands: vec![
                r"^git status".to_string(),
                r"^git diff".to_string(),
                r"^git log".to_string(),
                r"^ls\s".to_string(),
                r"^cat\s".to_string(),
                r"^find\s".to_string(),
                r"^grep\s".to_string(),
                r"^rg\s".to_string(),
            ],
            deny_commands: vec![
                r"rm\s+-rf\s".to_string(),
                r"dd\s".to_string(),
                r":()>".to_string(),
                r"mkfs".to_string(),
                r"fdisk".to_string(),
            ],
            require_approval: true,
            sandbox_mode: false,
            timeout_ms: Some(120_000),
        }
    }
}

impl SecurityPolicy {
    /// 交互式 CLI：不启用命令白名单（避免误杀）；仍匹配 deny 正则；最后走人工确认。
    pub fn interactive_shell() -> Self {
        Self {
            allow_commands: vec![],
            deny_commands: vec![
                r"rm\s+-rf".to_string(),
                r"\bdd\b".to_string(),
                r":\(\)\s*>".to_string(),
                r"mkfs".to_string(),
                r"\bfdisk\b".to_string(),
            ],
            require_approval: true,
            sandbox_mode: false,
            timeout_ms: Some(120_000),
        }
    }

    /// 写文件等敏感操作：无命令行模式匹配，仅依赖人工确认（或上层 Bypass / 无回调自动通过）。
    pub fn sensitive_mutation() -> Self {
        Self {
            allow_commands: vec![],
            deny_commands: vec![],
            require_approval: true,
            sandbox_mode: false,
            timeout_ms: None,
        }
    }
}
