//! Runtime-facing model capabilities.
//!
//! This is the single conservative resolver used by providers and the agent
//! runtime.  Product catalogs may add display metadata, but must not invent a
//! different context window or tool-loop capability for the same model.

use anycode_core::{LLMProvider, Message, MessageContent, MessageRole, ModelConfig};

pub const TOOL_RECOVERY_NUDGE: &str = "[anycode:tool-recovery] Call a tool now — do not refuse or outline a plan. For delivery/code tasks start with Glob or FileRead under fixtures/ or fixtures/e2e-complex-repo/, then Bash for cargo test. Use FileWrite/Edit only after inspecting the workspace.";

pub const TOOL_RECOVERY_NUDGE_FORCE_GLOB: &str =
    "[anycode:tool-recovery] REQUIRED: invoke Glob now with pattern `fixtures/**` (or `fixtures/e2e-complex-repo/**`). No prose, no refusal — only a tool call.";

/// Extra system guidance for 1B / Ollama weak-local models (path + tool protocol).
pub const WEAK_LOCAL_TOOL_GUIDANCE: &str = "# Weak local model constraints\n\n\
- The **Working directory** in Environment is the only writable project root. \
Use **relative paths** in FileWrite/Edit/FileRead/Glob (e.g. `notes.md`, `docs/plan.md`) — never invent `/Users/...` or `~/.anycode/workspace` paths.\n\
- For **Glob**, omit `path` or use `.` — the Environment working directory is already the search root.\n\
- When calling tools, emit **one** valid tool call with plain JSON arguments.\n\
- **Do not** output `<think>`, chain-of-thought, or long planning prose — call tools immediately.\n\
- For deliverables (md/ppt/doc), call FileWrite first, then confirm in one short sentence.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeModelCapabilities {
    pub chat: bool,
    pub native_tools: bool,
    pub context_tokens: u32,
    pub tool_loop_verified: bool,
    pub weak_local_model: bool,
}

fn provider_id(provider: &LLMProvider) -> &str {
    match provider {
        LLMProvider::Anthropic => "anthropic",
        LLMProvider::OpenAI => "openai",
        LLMProvider::Local => "local",
        LLMProvider::Custom(id) => id.as_str(),
    }
}

fn is_loopback_url(url: Option<&str>) -> bool {
    url.is_some_and(|url| {
        let lower = url.to_ascii_lowercase();
        lower.contains("127.0.0.1") || lower.contains("localhost") || lower.contains("[::1]")
    })
}

#[must_use]
pub fn resolve_runtime_model_capabilities(
    provider: &str,
    model: &str,
    base_url: Option<&str>,
) -> RuntimeModelCapabilities {
    let provider = crate::normalize_provider_id(provider);
    let model = model.trim().to_ascii_lowercase();
    let local = provider == "local"
        || provider == "ollama"
        || provider == "sglang"
        || is_loopback_url(base_url)
        || model.starts_with("managed-");
    let minicpm_1b = model.contains("minicpm5-1b") || model.contains("minicpm-5-1b");
    let weak_local_model = local && (minicpm_1b || model.contains("1b"));
    let sglang_native_minicpm_tools = provider == "sglang" && minicpm_1b;

    let context_tokens = if minicpm_1b {
        if model.contains("-e2e") || (local && (provider == "ollama" || provider == "sglang")) {
            // Ollama/SGLang harness align compaction threshold with deployment (32k).
            32_768
        } else {
            4_096
        }
    } else if local {
        // Unknown local runtimes must be conservative. Their configured value
        // can still override this through session.context_window_tokens.
        4_096
    } else {
        crate::model_context::resolve_known_context_window_tokens(&provider, &model)
    };

    RuntimeModelCapabilities {
        chat: true,
        native_tools: true,
        context_tokens,
        tool_loop_verified: sglang_native_minicpm_tools || !weak_local_model,
        weak_local_model,
    }
}

#[must_use]
pub fn capabilities_for_model_config(config: &ModelConfig) -> RuntimeModelCapabilities {
    resolve_runtime_model_capabilities(
        provider_id(&config.provider),
        &config.model,
        config.base_url.as_deref(),
    )
}

fn message_text(message: &Message) -> Option<&str> {
    match &message.content {
        MessageContent::Text(text) => Some(text.as_str()),
        _ => None,
    }
}

#[must_use]
pub fn is_first_agent_turn(messages: &[Message]) -> bool {
    !messages
        .iter()
        .any(|message| matches!(message.role, MessageRole::Assistant | MessageRole::Tool))
}

#[must_use]
pub fn has_tool_recovery_nudge(messages: &[Message]) -> bool {
    messages
        .iter()
        .rev()
        .filter_map(message_text)
        .any(|text| text.contains("[anycode:tool-recovery]"))
}

/// Deliberately conservative: ordinary Q&A must never be forced into a tool
/// call. Agent tasks that explicitly request inspection, mutation, execution,
/// or search are eligible.
#[must_use]
pub fn explicitly_requests_tool_execution(messages: &[Message]) -> bool {
    let Some(text) = messages
        .iter()
        .rev()
        .find(|message| message.role == MessageRole::User)
        .and_then(message_text)
    else {
        return false;
    };
    let text = text.to_ascii_lowercase();
    [
        "use the tool",
        "use tools",
        "run ",
        "execute ",
        "read ",
        "write ",
        "edit ",
        "search ",
        "find ",
        "inspect ",
        "create ",
        "delete ",
        "cargo",
        "manifest",
        "artifacts/",
        "fixtures/",
        "workspace",
        "git ",
        "delivery contract",
        "修改",
        "修复",
        "执行",
        "运行",
        "读取",
        "搜索",
        "查找",
        "创建",
        "删除",
        "交付",
        "验收",
        "合并",
        "汇报",
        "做个",
        "写一个",
        "写个",
        "生成",
        "ppt",
        ".md",
        "markdown",
        "filewrite",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn text(role: MessageRole, value: &str) -> Message {
        Message {
            id: Uuid::new_v4(),
            role,
            content: MessageContent::Text(value.into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn sglang_minicpm_uses_32k_context() {
        let got = resolve_runtime_model_capabilities(
            "sglang",
            "MiniCPM5-1B",
            Some("http://127.0.0.1:30000/v1/chat/completions"),
        );
        assert_eq!(got.context_tokens, 32_768);
        assert!(got.weak_local_model);
        assert!(got.tool_loop_verified);
    }

    #[test]
    fn ollama_minicpm_e2e_uses_32k_context() {
        let got = resolve_runtime_model_capabilities(
            "ollama",
            "minicpm5-1b-e2e",
            Some("http://127.0.0.1:11434/v1/chat/completions"),
        );
        assert_eq!(got.context_tokens, 32_768);
        assert!(got.weak_local_model);
    }

    #[test]
    fn managed_minicpm_has_conservative_verified_profile() {
        let got = resolve_runtime_model_capabilities(
            "openai",
            "managed-minicpm5-1b",
            Some("http://127.0.0.1:47100/v1/chat/completions"),
        );
        assert_eq!(got.context_tokens, 4_096);
        assert!(got.native_tools);
        assert!(got.weak_local_model);
        assert!(!got.tool_loop_verified);
    }

    #[test]
    fn complex_delivery_brief_requests_tools() {
        let brief = concat!(
            "六月数据、代码仓库、董事会材料一堆事，帮我完整交付一轮。\n",
            "仓库：fixtures/e2e-complex-repo/。材料放 artifacts/。\n",
            "DELIVERY_MANIFEST.json cargo test --workspace"
        );
        assert!(explicitly_requests_tool_execution(&[text(
            MessageRole::User,
            brief,
        )]));
    }

    #[test]
    fn explicit_tool_intent_does_not_match_plain_question() {
        assert!(explicitly_requests_tool_execution(&[text(
            MessageRole::User,
            "请读取 Cargo.toml 并修改版本"
        )]));
        assert!(!explicitly_requests_tool_execution(&[text(
            MessageRole::User,
            "Rust 的所有权是什么？"
        )]));
    }

    #[test]
    fn first_turn_excludes_existing_assistant_or_tool() {
        assert!(is_first_agent_turn(&[
            text(MessageRole::System, "s"),
            text(MessageRole::User, "run tests"),
        ]));
        assert!(!is_first_agent_turn(&[
            text(MessageRole::User, "run tests"),
            text(MessageRole::Assistant, "ok"),
        ]));
    }
}
