//! Centralized `Task` / `TaskContext` construction for the headless daemon.
//!
//! NOTE(2026-07): currently unwired — the only caller was the removed terminal CLI.
//! Kept per ADR 014 §6 (workflow DAG + checkpoints); rewire into the scheduler
//! cron path or delete — see docs/planning/audit-questions-2026-07-24.md Q6.
#![allow(dead_code)]

use crate::app_config::{resolve_agent_loop_limits, Config};
use crate::tasks::RunTaskOptions;
use crate::tool_policy::{
    headless_task_surface, resolve_headless_task_tool_filters, resolve_task_tool_filters,
};
use anycode_core::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

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
            live_trace_tx: None,
            tool_deny_names,
            tool_deny_prefixes,
            budget: options.budget,
            user_vision_images: vec![],
            loop_limits,
            chat_turn: None,
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
            live_trace_tx: None,
            tool_deny_names: vec![],
            tool_deny_prefixes: vec![],
            budget: TaskBudget::default(),
            user_vision_images: vec![],
            loop_limits: anycode_core::resolve_agent_loop_limits(None, None),
            chat_turn: None,
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
