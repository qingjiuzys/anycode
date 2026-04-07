//! anyCode Security Layer
//!
//! 工具调用审批、deny/allow 规则与沙箱相关策略

pub mod approval_presenter;

pub use anycode_core::SecurityPolicy;

use crate::approval_presenter::{render_approval_request, ApprovalSurface};
use anycode_core::prelude::*;
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tokio::sync::RwLock;

// ============================================================================
// 预编译策略（避免每条工具调用重复 Regex::new）
// ============================================================================

struct CompiledPolicy {
    raw: SecurityPolicy,
    /// (原始模式串, 编译结果) — 仅包含编译成功的项，与旧行为一致（非法正则被跳过）
    deny: Vec<(String, Regex)>,
    allow: Vec<Regex>,
}

impl CompiledPolicy {
    fn compile(raw: SecurityPolicy) -> Self {
        let deny = raw
            .deny_commands
            .iter()
            .filter_map(|p| Regex::new(p).ok().map(|re| (p.clone(), re)))
            .collect();
        let allow = raw
            .allow_commands
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();
        Self { raw, deny, allow }
    }

    fn raw(&self) -> &SecurityPolicy {
        &self.raw
    }
}

fn default_compiled_policy() -> &'static CompiledPolicy {
    static C: OnceLock<CompiledPolicy> = OnceLock::new();
    C.get_or_init(|| CompiledPolicy::compile(SecurityPolicy::default()))
}

// ============================================================================
// Approval System (来自 OpenClaw)
// ============================================================================

pub struct ApprovalSystem {
    policies: Arc<RwLock<HashMap<ToolName, CompiledPolicy>>>,
    approval_callback: Option<Box<dyn ApprovalCallback>>,
}

#[async_trait]
pub trait ApprovalCallback: Send + Sync {
    async fn request_approval(
        &self,
        tool: &str,
        input: &serde_json::Value,
        policy: &SecurityPolicy,
    ) -> anyhow::Result<bool>;
}

impl Default for ApprovalSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalSystem {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            approval_callback: None,
        }
    }

    pub fn with_callback(mut self, callback: Box<dyn ApprovalCallback>) -> Self {
        self.approval_callback = Some(callback);
        self
    }

    pub async fn set_policy(&self, tool: ToolName, policy: SecurityPolicy) {
        let compiled = CompiledPolicy::compile(policy);
        let mut policies = self.policies.write().await;
        policies.insert(tool, compiled);
    }

    pub fn has_approval_callback(&self) -> bool {
        self.approval_callback.is_some()
    }

    /// 与 `check_tool_call` 相同的策略解析：已注册工具用其策略，否则默认策略。
    pub async fn tool_policy_require_approval(&self, tool: &str) -> bool {
        let policies = self.policies.read().await;
        let compiled = policies
            .get(tool)
            .map(|c| c as &CompiledPolicy)
            .unwrap_or_else(|| default_compiled_policy());
        compiled.raw().require_approval
    }

    /// 策略判定 + 可选交互审批。
    ///
    /// **约定**：`policy.require_approval == true` 且未注册 `approval_callback` 时视为自动通过
    ///（供 `require_approval: false` 的 CLI / 守护进程路径使用）；有回调则必须经回调确认。
    pub async fn check_tool_call(&self, tool: &str, input: &serde_json::Value) -> ApprovalResult {
        let policies = self.policies.read().await;
        let compiled = policies
            .get(tool)
            .map(|c| c as &CompiledPolicy)
            .unwrap_or_else(|| default_compiled_policy());
        let policy = compiled.raw();

        if let Some(command) = Self::extract_command(tool, input) {
            for (pat, re) in &compiled.deny {
                if re.is_match(&command) {
                    return ApprovalResult::Denied {
                        reason: format!("Command matches deny pattern: {}", pat),
                    };
                }
            }

            let mut allowed = false;
            for re in &compiled.allow {
                if re.is_match(&command) {
                    allowed = true;
                    break;
                }
            }

            if !allowed && !policy.allow_commands.is_empty() {
                return ApprovalResult::Denied {
                    reason: "Command not in allow list".to_string(),
                };
            }
        }

        if policy.require_approval {
            if let Some(callback) = &self.approval_callback {
                match callback.request_approval(tool, input, policy).await {
                    Ok(approved) => {
                        if approved {
                            ApprovalResult::Approved
                        } else {
                            ApprovalResult::Denied {
                                reason: "User denied".to_string(),
                            }
                        }
                    }
                    Err(e) => ApprovalResult::Denied {
                        reason: format!("Approval error: {}", e),
                    },
                }
            } else {
                ApprovalResult::Approved
            }
        } else {
            ApprovalResult::Approved
        }
    }

    /// Claude `alwaysAsk`：必须经用户确认；**无审批回调时拒绝**（与 `require_approval` 无回调自动通过不同）。
    pub async fn confirm_claude_ask_or_deny(
        &self,
        tool: &str,
        input: &serde_json::Value,
    ) -> Result<(), CoreError> {
        let Some(callback) = &self.approval_callback else {
            return Err(CoreError::PermissionDenied(
                "Tool matches alwaysAsk (security.always_ask_rules); register an interactive approval callback (security.require_approval and/or non-empty always_ask_rules, and not -I / ANYCODE_IGNORE_APPROVAL)."
                    .to_string(),
            ));
        };
        match callback
            .request_approval(tool, input, default_compiled_policy().raw())
            .await
        {
            Ok(true) => Ok(()),
            Ok(false) => Err(CoreError::PermissionDenied("User denied".to_string())),
            Err(e) => Err(CoreError::PermissionDenied(format!(
                "Approval error: {}",
                e
            ))),
        }
    }

    fn extract_command(tool: &str, input: &serde_json::Value) -> Option<String> {
        match tool {
            "Bash" | "PowerShell" => input
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        }
    }
}

/// 批准结果
#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalResult {
    Approved,
    Denied { reason: String },
}

// ============================================================================
// Security Layer (集成层)
// ============================================================================

pub struct SecurityLayer {
    permission_mode: Arc<RwLock<PermissionMode>>,
    approval_system: ApprovalSystem,
    audit_log: Arc<RwLock<Vec<SecurityEvent>>>,
}

#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool: String,
    pub input: serde_json::Value,
    pub result: ApprovalResult,
    pub user_id: Option<String>,
}

impl SecurityLayer {
    pub fn new(permission_mode: PermissionMode) -> Self {
        Self::new_with_optional_callback(permission_mode, None)
    }

    pub fn new_with_optional_callback(
        permission_mode: PermissionMode,
        approval_callback: Option<Box<dyn ApprovalCallback>>,
    ) -> Self {
        let approval_system = match approval_callback {
            Some(cb) => ApprovalSystem::new().with_callback(cb),
            None => ApprovalSystem::new(),
        };
        Self {
            permission_mode: Arc::new(RwLock::new(permission_mode)),
            approval_system,
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn set_tool_policy(&self, tool: impl Into<ToolName>, policy: SecurityPolicy) {
        self.approval_system.set_policy(tool.into(), policy).await;
    }

    pub async fn check_tool_call(
        &self,
        tool: &str,
        input: &serde_json::Value,
    ) -> Result<bool, CoreError> {
        let permission_mode = self.permission_mode.read().await;
        match *permission_mode {
            PermissionMode::BypassPermissions => {
                return Ok(true);
            }
            PermissionMode::AcceptEdits if is_readonly_tool(tool) => {
                return Ok(true);
            }
            PermissionMode::Auto if is_readonly_tool(tool) => {
                return Ok(true);
            }
            PermissionMode::AcceptEdits => {}
            PermissionMode::Auto => {}
            _ => {}
        }
        drop(permission_mode);

        let result = self.approval_system.check_tool_call(tool, input).await;

        let event = SecurityEvent {
            timestamp: chrono::Utc::now(),
            tool: tool.to_string(),
            input: input.clone(),
            result: result.clone(),
            user_id: None,
        };
        self.audit_log.write().await.push(event);

        match result {
            ApprovalResult::Approved => Ok(true),
            ApprovalResult::Denied { reason } => Err(CoreError::PermissionDenied(reason)),
        }
    }

    /// 见 `ApprovalSystem::confirm_claude_ask_or_deny`（Claude `alwaysAsk`）。
    pub async fn confirm_claude_ask_or_deny(
        &self,
        tool: &str,
        input: &serde_json::Value,
    ) -> Result<(), CoreError> {
        self.approval_system
            .confirm_claude_ask_or_deny(tool, input)
            .await
    }

    /// `check_tool_call` 已因该工具 `require_approval` + 注册回调完成交互确认时，跳过重复的 `confirm_claude_ask_or_deny`。
    pub async fn skip_redundant_claude_ask_after_tool_check(&self, tool: &str) -> bool {
        if !self.approval_system.has_approval_callback() {
            return false;
        }
        let permission_mode = self.permission_mode.read().await;
        if matches!(*permission_mode, PermissionMode::Auto) && is_readonly_tool(tool) {
            return false;
        }
        drop(permission_mode);
        self.approval_system
            .tool_policy_require_approval(tool)
            .await
    }

    pub async fn set_permission_mode(&self, mode: PermissionMode) {
        let mut permission_mode = self.permission_mode.write().await;
        *permission_mode = mode;
    }

    pub async fn get_audit_log(&self) -> Vec<SecurityEvent> {
        self.audit_log.read().await.clone()
    }

    /// `BypassPermissions` 时跳过基于规则的拦截（与 `check_tool_call` 一致）。
    pub async fn is_bypass_permissions(&self) -> bool {
        matches!(
            *self.permission_mode.read().await,
            PermissionMode::BypassPermissions
        )
    }
}

fn is_readonly_tool(tool: &str) -> bool {
    matches!(
        tool,
        "FileRead" | "Glob" | "Grep" | "LSP" | "WebSearch" | "WebFetch"
    )
}

// ============================================================================
// Interactive Approval Callback
// ============================================================================

pub struct InteractiveApprovalCallback {
    prompt_format: PromptFormat,
    session_read_allow_dirs: Arc<StdMutex<HashSet<String>>>,
}

impl InteractiveApprovalCallback {
    pub fn new(prompt_format: PromptFormat) -> Self {
        Self {
            prompt_format,
            session_read_allow_dirs: Arc::new(StdMutex::new(HashSet::new())),
        }
    }

    fn extract_read_path(input: &serde_json::Value) -> Option<String> {
        for k in ["path", "file_path", "target_directory", "directory"] {
            if let Some(v) = input.get(k).and_then(|v| v.as_str()) {
                let t = v.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
        }
        None
    }

    fn read_scope_dir(path: &str) -> Option<String> {
        let p = Path::new(path);
        if p.is_dir() {
            return Some(path.trim_end_matches('/').to_string());
        }
        p.parent()
            .map(|d| d.to_string_lossy().trim_end_matches('/').to_string())
    }

    fn path_allowed_for_session(&self, path: &str) -> bool {
        let Ok(guard) = self.session_read_allow_dirs.lock() else {
            return false;
        };
        let p = path.trim_end_matches('/');
        guard
            .iter()
            .any(|dir| p == dir || p.starts_with(&format!("{dir}/")))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PromptFormat {
    CLI,
    Silent,
}

#[async_trait]
impl ApprovalCallback for InteractiveApprovalCallback {
    async fn request_approval(
        &self,
        tool: &str,
        input: &serde_json::Value,
        _policy: &SecurityPolicy,
    ) -> anyhow::Result<bool> {
        match self.prompt_format {
            PromptFormat::CLI => {
                let read_like = is_readonly_tool(tool);
                if read_like {
                    if let Some(path) = Self::extract_read_path(input) {
                        if self.path_allowed_for_session(&path) {
                            return Ok(true);
                        }
                        let dir_name = Self::read_scope_dir(&path).unwrap_or(path.clone());
                        println!("\n⚠️  Tool Execution Request");
                        println!(
                            "{}",
                            render_approval_request(ApprovalSurface::Cli, tool, input)
                        );
                        println!("\nDo you want to proceed?");
                        println!("❯ 1. Yes");
                        println!("  2. Yes, allow reading from {dir_name}/ during this session");
                        println!("  3. No");
                        print!("Select [1-3] (default: 3): ");
                        use std::io::Write;
                        let _ = std::io::stdout().flush();

                        let mut line = String::new();
                        std::io::stdin().read_line(&mut line)?;
                        let choice = line.trim();
                        match choice {
                            "1" | "y" | "Y" => return Ok(true),
                            "2" => {
                                if let Ok(mut g) = self.session_read_allow_dirs.lock() {
                                    g.insert(dir_name);
                                }
                                return Ok(true);
                            }
                            _ => return Ok(false),
                        }
                    }
                }

                println!("\n⚠️  Tool Execution Request");
                println!(
                    "{}",
                    render_approval_request(ApprovalSurface::Cli, tool, input)
                );

                print!("Approve? [y/N] ");
                use std::io::Write;
                let _ = std::io::stdout().flush();

                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;

                Ok(line.trim().to_lowercase() == "y")
            }
            PromptFormat::Silent => {
                tracing::warn!(
                    target: "anycode_security",
                    "{}",
                    render_approval_request(ApprovalSurface::Silent, tool, input)
                );
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestApproveAll;

    #[async_trait]
    impl ApprovalCallback for TestApproveAll {
        async fn request_approval(
            &self,
            _tool: &str,
            _input: &serde_json::Value,
            _policy: &SecurityPolicy,
        ) -> anyhow::Result<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn skip_redundant_claude_ask_when_tool_policy_requires_approval() {
        let layer = SecurityLayer::new_with_optional_callback(
            PermissionMode::Default,
            Some(Box::new(TestApproveAll)),
        );
        layer
            .set_tool_policy(
                "Bash",
                SecurityPolicy {
                    require_approval: true,
                    allow_commands: vec![],
                    deny_commands: vec![],
                    sandbox_mode: false,
                    timeout_ms: None,
                },
            )
            .await;
        assert!(
            layer
                .skip_redundant_claude_ask_after_tool_check("Bash")
                .await
        );
    }

    #[tokio::test]
    async fn no_skip_redundant_claude_ask_when_tool_policy_skips_approval() {
        let layer = SecurityLayer::new_with_optional_callback(
            PermissionMode::Default,
            Some(Box::new(TestApproveAll)),
        );
        layer
            .set_tool_policy(
                "Bash",
                SecurityPolicy {
                    require_approval: false,
                    allow_commands: vec![],
                    deny_commands: vec![],
                    sandbox_mode: false,
                    timeout_ms: None,
                },
            )
            .await;
        assert!(
            !layer
                .skip_redundant_claude_ask_after_tool_check("Bash")
                .await
        );
    }

    #[tokio::test]
    async fn no_skip_redundant_claude_ask_for_readonly_under_auto_mode() {
        let layer = SecurityLayer::new_with_optional_callback(
            PermissionMode::Auto,
            Some(Box::new(TestApproveAll)),
        );
        assert!(
            !layer
                .skip_redundant_claude_ask_after_tool_check("FileRead")
                .await
        );
    }

    #[tokio::test]
    async fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert!(!policy.allow_commands.is_empty());
        assert!(!policy.deny_commands.is_empty());
        assert!(policy.require_approval);
    }

    #[tokio::test]
    async fn test_approval_system() {
        let system = ApprovalSystem::new();

        let result = system
            .check_tool_call("Bash", &serde_json::json!({"command": "rm -rf /"}))
            .await;
        assert!(matches!(result, ApprovalResult::Denied { .. }));

        let result = system
            .check_tool_call("Bash", &serde_json::json!({"command": "git status"}))
            .await;
        assert!(matches!(result, ApprovalResult::Approved));
    }

    #[tokio::test]
    async fn test_security_layer() {
        let layer = SecurityLayer::new(PermissionMode::Default);

        assert!(layer
            .check_tool_call("FileRead", &serde_json::json!({"file_path": "/tmp/test"}))
            .await
            .is_ok());

        let result = layer
            .check_tool_call("Bash", &serde_json::json!({"command": "echo test"}))
            .await;
        assert!(result.is_ok() || result.is_err());
    }
}
