use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalSurface {
    Cli,
    Web,
    WeChat,
    Silent,
}

/// 从工具输入中提取模型自述原因（`description` / `reason` / `explanation`）。
fn extract_reason(input: &Value) -> Option<String> {
    ["description", "reason", "explanation"]
        .iter()
        .find_map(|key| {
            input
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
}

/// 规则式中文说明：帮助用户在审批时快速理解「为什么需要这个权限」。
/// 与 Claude Code 权限解释器对齐——展示工具意图而非只给参数 JSON。
fn rule_based_explanation(tool: &str, input: &Value) -> Option<String> {
    let brief = |cmd: &str| -> String {
        const MAX: usize = 120;
        let trimmed = cmd.trim();
        if trimmed.len() > MAX {
            // 字节索引必须落在 UTF-8 字符边界上，否则 &s[..n] 会 panic。
            // 回退到 <= MAX 的最近合法边界。
            let mut end = MAX;
            while end > 0 && !trimmed.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}…", &trimmed[..end])
        } else {
            trimmed.to_string()
        }
    };
    match tool {
        "Bash" => input
            .get("command")
            .and_then(Value::as_str)
            .map(|c| format!("执行命令：{}", brief(c))),
        "FileWrite" | "Write" => input
            .get("file_path")
            .and_then(Value::as_str)
            .map(|p| format!("写入文件：{p}")),
        "Edit" | "FileEdit" => input
            .get("file_path")
            .and_then(Value::as_str)
            .map(|p| format!("修改文件：{p}")),
        "Read" | "FileRead" => input
            .get("file_path")
            .and_then(Value::as_str)
            .map(|p| format!("读取文件：{p}")),
        "Grep" => input
            .get("pattern")
            .and_then(Value::as_str)
            .map(|p| format!("搜索模式：{p}")),
        "WebFetch" => input
            .get("url")
            .and_then(Value::as_str)
            .map(|u| format!("抓取网页：{u}")),
        "WebSearch" => input
            .get("query")
            .and_then(Value::as_str)
            .map(|q| format!("联网搜索：{}", brief(q))),
        "CronCreate" => input
            .get("schedule")
            .or_else(|| input.get("cron"))
            .and_then(Value::as_str)
            .map(|s| format!("注册定时任务：{s}")),
        "Task" | "Agent" => input
            .get("prompt")
            .and_then(Value::as_str)
            .map(|p| format!("派出子代理任务：{}", brief(p))),
        _ => None,
    }
}

/// 渲染审批请求（权限解释器版）：优先展示模型原因，其次规则式意图说明，最后回退参数 JSON。
pub fn render_approval_request(surface: ApprovalSurface, tool: &str, input: &Value) -> String {
    let reason = extract_reason(input)
        .or_else(|| rule_based_explanation(tool, input))
        .map(|r| format!("原因：{r}"));
    let payload = serde_json::to_string_pretty(input).unwrap_or_else(|_| "{}".to_string());
    match surface {
        ApprovalSurface::Cli => {
            let head = reason
                .as_deref()
                .map(|r| format!("Approve tool `{tool}`?\n{r}"))
                .unwrap_or_else(|| format!("Approve tool `{tool}`?"));
            format!("{head}\n{payload}")
        }
        ApprovalSurface::Web => {
            let head = reason
                .as_deref()
                .map(|r| format!("Web approval requested for `{tool}`\n{r}"))
                .unwrap_or_else(|| format!("Web approval requested for `{tool}`"));
            format!("{head}\n{payload}")
        }
        ApprovalSurface::WeChat => {
            let head = reason
                .as_deref()
                .map(|r| format!("待审批工具：{tool}\n{r}"))
                .unwrap_or_else(|| format!("待审批工具：{tool}"));
            format!("{head}\n请在运行 anycode 的终端（或 TUI）里按提示批准，或忽略以拒绝。")
        }
        ApprovalSurface::Silent => {
            let head = reason
                .as_deref()
                .map(|r| format!("approval required: {tool} ({r})"))
                .unwrap_or_else(|| format!("approval required: {tool}"));
            head
        }
    }
}
