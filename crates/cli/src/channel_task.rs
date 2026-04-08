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
        },
        created_at: chrono::Utc::now(),
    }
}
