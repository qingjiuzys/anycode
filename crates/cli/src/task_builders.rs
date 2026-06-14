//! Centralized `Task` / `TaskContext` construction for CLI entrypoints.

use crate::app_config::{resolve_agent_loop_limits, Config};
use crate::channel_task::{im_channel_cron_scheduling_hint, ChannelTaskInput};
use crate::tasks::RunTaskOptions;
use crate::tool_policy::{
    channel_task_tool_filters, headless_task_surface, resolve_headless_task_tool_filters,
    resolve_task_tool_filters,
};
use anycode_core::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) struct WechatTaskParams {
    pub agent: String,
    pub prompt: String,
    pub working_directory: PathBuf,
    pub system_prompt_append: Option<String>,
    pub tool_deny_names: Vec<String>,
    pub tool_deny_prefixes: Vec<String>,
    pub nested_cancel: Option<Arc<AtomicBool>>,
    pub user_vision_images: Vec<VisionImage>,
}

pub(crate) fn build_headless_task(
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    options: &RunTaskOptions,
    config: Option<&Config>,
) -> Task {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    let surface = headless_task_surface();
    let (tool_deny_names, tool_deny_prefixes) = match config {
        Some(cfg) => resolve_task_tool_filters(cfg, surface, options),
        None => resolve_headless_task_tool_filters(options),
    };
    let loop_limits = config
        .map(|c| resolve_agent_loop_limits(&c.runtime))
        .unwrap_or_else(|| anycode_core::resolve_agent_loop_limits(None, None));
    let session_id = options.session_id.unwrap_or_else(Uuid::new_v4);
    Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(agent_type),
        prompt,
        context: TaskContext {
            session_id,
            working_directory: working_dir.to_string_lossy().to_string(),
            environment: HashMap::new(),
            user_id: None,
            system_prompt_append: None,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
            tool_deny_names,
            tool_deny_prefixes,
            budget: options.budget,
            user_vision_images: vec![],
            loop_limits,
        },
        created_at: chrono::Utc::now(),
    }
}

pub(crate) fn build_channel_task(input: ChannelTaskInput, config: &Config) -> Task {
    let (tool_deny_names, tool_deny_prefixes) = channel_task_tool_filters(config);
    let loop_limits = resolve_agent_loop_limits(&config.runtime);
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
                "## Channel Runtime\nchannel={}\nchannel_id={}\nuser_id={}\nFor channel requests, prefer concise, directly actionable answers and avoid UI-only instructions.{}\n\n{}",
                input.channel_name,
                input.channel_id,
                input.user_id,
                crate::channel_task::channel_ask_user_question_hint(input.channel_name),
                im_channel_cron_scheduling_hint(),
            )),
            context_injections: vec![format!(
                "## Channel Session\nplatform={}\nchat_or_channel={}\nuser={}",
                input.channel_name, input.channel_id, input.user_id
            )],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
            tool_deny_names,
            tool_deny_prefixes,
            budget: TaskBudget::default(),
            user_vision_images: input.user_vision_images,
            loop_limits,
        },
        created_at: chrono::Utc::now(),
    }
}

pub(crate) fn build_wechat_task(params: WechatTaskParams) -> Task {
    let working_dir =
        std::fs::canonicalize(&params.working_directory).unwrap_or(params.working_directory);
    Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(params.agent),
        prompt: params.prompt,
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: working_dir.to_string_lossy().to_string(),
            environment: HashMap::new(),
            user_id: None,
            system_prompt_append: params.system_prompt_append,
            context_injections: vec![format!("## Channel Session\nplatform=wechat\n")],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: params.nested_cancel,
            channel_progress_tx: None,
            tool_deny_names: params.tool_deny_names,
            tool_deny_prefixes: params.tool_deny_prefixes,
            budget: TaskBudget::default(),
            user_vision_images: params.user_vision_images,
            loop_limits: anycode_core::resolve_agent_loop_limits(None, None),
        },
        created_at: chrono::Utc::now(),
    }
}

/// Goal / workflow paths without tool-policy profiles (empty deny lists).
pub(crate) fn build_minimal_task(
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    system_prompt_append: Option<String>,
) -> Task {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(agent_type),
        prompt,
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: working_dir.to_string_lossy().to_string(),
            environment: HashMap::new(),
            user_id: None,
            system_prompt_append,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
            tool_deny_names: vec![],
            tool_deny_prefixes: vec![],
            budget: TaskBudget::default(),
            user_vision_images: vec![],
            loop_limits: anycode_core::resolve_agent_loop_limits(None, None),
        },
        created_at: chrono::Utc::now(),
    }
}

pub(crate) fn build_cron_task(
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    options: &RunTaskOptions,
    config: &Config,
) -> Task {
    build_headless_task(agent_type, prompt, working_dir, options, Some(config))
}
