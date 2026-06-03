//! Security layer, tool policies, and MCP defer gate.

use crate::app_config::Config;
use anycode_core::prelude::*;
use anycode_security::{ApprovalCallback, SecurityLayer, SecurityPolicy};
use anycode_tools::catalog;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub(crate) struct SecuritySetup {
    pub security: Arc<SecurityLayer>,
    pub fw_policy: SecurityPolicy,
    pub mcp_defer_gate: Option<Arc<Mutex<HashSet<String>>>>,
}

pub(crate) async fn build_security_setup(
    config: &Config,
    approval_override: Option<Box<dyn ApprovalCallback>>,
) -> SecuritySetup {
    let permission_mode = match config.security.permission_mode.as_str() {
        "auto" => PermissionMode::Auto,
        "plan" => PermissionMode::Plan,
        "accept_edits" | "acceptEdits" => PermissionMode::AcceptEdits,
        "bypass" => PermissionMode::BypassPermissions,
        _ => PermissionMode::Default,
    };
    let approval_callback: Option<Box<dyn ApprovalCallback>> = if let Some(cb) = approval_override {
        Some(cb)
    } else if !crate::app_config::security_wants_interactive_approval_callback(config) {
        None
    } else {
        Some(Box::new(
            crate::workbench_approval::WorkbenchApprovalCallback::web_and_cli(),
        ))
    };
    let security = Arc::new(SecurityLayer::new_with_optional_callback(
        permission_mode,
        approval_callback,
    ));
    let mut bash_policy = SecurityPolicy::interactive_shell();
    bash_policy.sandbox_mode = config.security.sandbox_mode;
    let mut fw_policy = SecurityPolicy::sensitive_mutation();
    fw_policy.sandbox_mode = config.security.sandbox_mode;
    if !config.security.require_approval {
        bash_policy.require_approval = false;
        fw_policy.require_approval = false;
    }
    security
        .set_tool_policy(catalog::TOOL_BASH, bash_policy)
        .await;
    security
        .set_tool_policy(catalog::TOOL_FILE_WRITE, fw_policy.clone())
        .await;

    for t in catalog::SECURITY_SENSITIVE_TOOL_IDS {
        security.set_tool_policy(*t, fw_policy.clone()).await;
    }

    let mcp_defer_gate = if config.security.defer_mcp_tools {
        Some(Arc::new(Mutex::new(HashSet::new())))
    } else {
        None
    };

    SecuritySetup {
        security,
        fw_policy,
        mcp_defer_gate,
    }
}
