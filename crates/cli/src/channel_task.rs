use anycode_core::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) struct ChannelTaskInput {
    pub agent_type: String,
    pub prompt: String,
    pub working_directory: String,
    pub channel_id: String,
    pub user_id: String,
    pub channel_name: &'static str,
}

/// Truncate runtime/provider error text for IM (UTF-8 safe, character-wise).
pub(crate) fn im_task_failure_detail_excerpt(
    details: Option<&str>,
    max_chars: usize,
) -> Option<String> {
    let d = details?.trim();
    if d.is_empty() {
        return None;
    }
    let n = d.chars().count();
    Some(if n > max_chars {
        format!("{}…", d.chars().take(max_chars).collect::<String>())
    } else {
        d.to_string()
    })
}

pub(crate) fn build_channel_task(input: ChannelTaskInput) -> Task {
    Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(input.agent_type),
        prompt: input.prompt,
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: input.working_directory,
            environment: HashMap::new(),
            user_id: Some(input.user_id.clone()),
            system_prompt_append: Some(format!(
                "## Channel Runtime\nchannel={}\nchannel_id={}\nuser_id={}\nFor channel requests, prefer concise, directly actionable answers and avoid UI-only instructions.",
                input.channel_name, input.channel_id, input.user_id
            )),
            context_injections: vec![format!(
                "## Channel Session\nplatform={}\nchat_or_channel={}\nuser={}",
                input.channel_name, input.channel_id, input.user_id
            )],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
        },
        created_at: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn im_detail_excerpt_skips_blank() {
        assert!(im_task_failure_detail_excerpt(None, 10).is_none());
        assert!(im_task_failure_detail_excerpt(Some("  \n"), 10).is_none());
    }

    #[test]
    fn im_detail_excerpt_truncates_by_char() {
        let s = "α".repeat(50);
        let ex = im_task_failure_detail_excerpt(Some(&s), 12).unwrap();
        assert!(ex.ends_with('…'));
        assert_eq!(ex.chars().count(), 13);
    }
}
