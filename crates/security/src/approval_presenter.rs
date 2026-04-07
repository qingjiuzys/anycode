use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalSurface {
    Cli,
    Web,
    WeChat,
    Silent,
}

pub fn render_approval_request(surface: ApprovalSurface, tool: &str, input: &Value) -> String {
    let payload = serde_json::to_string_pretty(input).unwrap_or_else(|_| "{}".to_string());
    match surface {
        ApprovalSurface::Cli => format!("Approve tool `{tool}`?\n{payload}"),
        ApprovalSurface::Web => format!("Web approval requested for `{tool}`"),
        ApprovalSurface::WeChat => {
            format!(
                "待审批工具：{tool}\n请在运行 anycode 的终端（或 TUI）里按提示批准，或忽略以拒绝。"
            )
        }
        ApprovalSurface::Silent => format!("approval required: {tool}"),
    }
}
