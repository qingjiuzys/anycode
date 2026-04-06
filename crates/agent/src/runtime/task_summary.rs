//! 任务收尾：直接 assistant 正文 vs LLM 总结回执。

use anycode_core::prelude::*;
use std::sync::Arc;

/// 若末条为带正文的 assistant，返回该正文（trim 后非空）。
pub(crate) fn last_assistant_plain_text(messages: &[Message]) -> Option<String> {
    if let Some(last) = messages.last() {
        if last.role == MessageRole::Assistant {
            if let MessageContent::Text(t) = &last.content {
                let fast = t.trim();
                if !fast.is_empty() {
                    return Some(fast.to_string());
                }
            }
        }
    }
    None
}

/// 基于运行轨迹生成中文 summary（失败时返回可展示的错误占位字符串，不向上 Err）。
pub(crate) async fn llm_summary_receipt(
    llm_client: &Arc<dyn LLMClient>,
    summary_model: &ModelConfig,
    task: &Task,
    total_tool_calls: usize,
    max_turns: usize,
    max_tool_calls: usize,
    artifacts_brief: &str,
    output_tail: &str,
) -> String {
    let summary_messages = vec![
        Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::System,
            content: MessageContent::Text(
                "你是 anyCode 的运行总结器。请基于给定的信息，为用户输出 5-10 行中文“完成回执”总结：做了什么、关键步骤（含工具调用概况）、结果/下一步。不要输出冗长正文，不要复述原始日志。".to_string()
            ),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        },
        Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(format!(
                "agent_type: {}\nprompt: {}\nturns_max: {}\ntool_calls_max: {}\ntotal_tool_calls: {}\n\n== artifacts ==\n{}\n\n== run_log_tail ==\n{}",
                task.agent_type.as_str(),
                task.prompt,
                max_turns,
                max_tool_calls,
                total_tool_calls,
                artifacts_brief,
                output_tail
            )),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        },
    ];

    match llm_client
        .chat(summary_messages, vec![], summary_model)
        .await
    {
        Ok(summary_resp) => match summary_resp.message.content {
            MessageContent::Text(t) => t,
            _ => "（summary 生成失败：unexpected content）".to_string(),
        },
        Err(e) => format!("（summary 生成失败：{}）", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_assistant_detects_trailing_text() {
        let messages = vec![Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text("  done  ".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }];
        assert_eq!(
            last_assistant_plain_text(&messages).as_deref(),
            Some("done")
        );
    }

    #[test]
    fn last_assistant_ignores_empty_tail() {
        let messages = vec![Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text("   \n  ".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }];
        assert!(last_assistant_plain_text(&messages).is_none());
    }
}
