use crate::app_config::Config;
use anycode_core::prelude::*;

/// Cron + scheduler semantics shared by WeChat, Telegram, Discord channel agents.
pub(crate) fn channel_ask_user_question_hint(channel_name: &str) -> &'static str {
    match channel_name {
        "telegram" => "\n\n## Telegram AskUserQuestion\nWhen you call AskUserQuestion, the user chooses via inline buttons. Prefer that tool over asking for free-form replies; if the UI fails, the user may still reply with a digit 1–N matching the listed options.",
        "discord" => "\n\n## Discord AskUserQuestion\nWhen you call AskUserQuestion, the user receives numbered options and replies with a digit 1–N.",
        "wechat" => "\n\n## WeChat AskUserQuestion\nWhen you call AskUserQuestion, the user receives numbered options and replies with a digit 1–N in chat.",
        _ => "",
    }
}

pub(crate) fn im_channel_cron_scheduling_hint() -> &'static str {
    "## Cron / scheduled tasks\n\
     - Tools: `CronCreate`, `CronDelete`, `CronList`. Jobs persist in `~/.anycode/tasks/orchestration.json`.\n\
     - Jobs only fire when a scheduler holds `~/.anycode/tasks/scheduler.lock`: run **`anycode scheduler`**, or rely on this long-running bridge (WeChat/Telegram/Discord each tries to embed the same built-in scheduler; **only one** ticks per machine).\n\
     - **`CronCreate` default `schedule_timezone`: `local`** — `schedule` uses **this machine's local wall clock** (e.g. 12:15 in China = hour 12 in the expression); it is converted to UTC for storage. Do **not** subtract 8 hours yourself. Use **`utc`** only if the expression is already UTC. For a fixed region use **IANA** (e.g. `Asia/Shanghai`) when the machine timezone differs from the user's.\n\
     - Scheduler append-only log: `~/.anycode/logs/cron-runs.jsonl` (`started` / `ok` / `error` per fire).\n\
     - `CronCreate` returns `next_fire_utc` / `next_fire_local` when the schedule parses—use them to confirm the first run time.\n\
     - After a cron fires from the WeChat bridge, the result is sent to the **last chat** on this bridge (not only stdout).\n\
     - `CronCreate` registers the schedule; do not imply a job ran if the user only saved it.\n\
     - 中文：登记定时任务用 `CronCreate`（默认按**本地时间**理解 `schedule`，无需自己减 8 小时）；到点需调度器在跑（微信桥内嵌即可）。触发后会向**最近一次对话的微信**推送结果。"
}

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

pub(crate) fn build_channel_task(input: ChannelTaskInput, config: &Config) -> Task {
    crate::task_builders::build_channel_task(input, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::{Config, LLMConfig, MemoryConfig, RuntimeSettings, SecurityConfig};
    use anycode_core::{FeatureRegistry, ModelRouteProfile, RuntimeMode};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_test_config() -> Config {
        Config {
            llm: LLMConfig {
                provider: "mock".into(),
                plan: "coding".into(),
                model: "mock".into(),
                api_key: "k".into(),
                base_url: None,
                temperature: 0.7,
                max_tokens: 4096,
                provider_credentials: HashMap::new(),
                zai_tool_choice_first_turn: false,
            },
            memory: MemoryConfig {
                path: PathBuf::from("/tmp"),
                auto_save: false,
                backend: "noop".into(),
                pipeline: anycode_core::MemoryPipelineSettings::default(),
                embedding_model: None,
                embedding_base_url: None,
                embedding_provider: "http".into(),
                embedding_local_cache_dir: None,
                embedding_local_model: None,
                embedding_hf_endpoint: None,
            },
            security: SecurityConfig {
                permission_mode: "default".into(),
                require_approval: true,
                sandbox_mode: false,
                mcp_tool_deny_patterns: vec![],
                mcp_tool_deny_rules: vec![],
                always_allow_rules: vec![],
                always_ask_rules: vec![],
                defer_mcp_tools: false,
                session_skip_interactive_approval: false,
            },
            routing: Default::default(),
            runtime: RuntimeSettings {
                default_mode: RuntimeMode::Code,
                features: FeatureRegistry::default(),
                model_routes: ModelRouteProfile::default(),
                tool_policy_profiles: Default::default(),
                tool_deny_names: vec![],
                tool_deny_prefixes: vec![],
                model_fallback: None,
                workspace_project_label: None,
                workspace_channel_profile: None,
            },
            prompt: Default::default(),
            skills: Default::default(),
            agents: Default::default(),
            session: Default::default(),
            status_line: Default::default(),
            terminal: Default::default(),
            channels: Default::default(),
            lsp: Default::default(),
            notifications: Default::default(),
        }
    }

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

    #[test]
    fn channel_task_append_includes_shared_cron_hint() {
        let config = minimal_test_config();
        let t = build_channel_task(
            ChannelTaskInput {
                agent_type: "workspace-assistant".into(),
                prompt: "hi".into(),
                working_directory: "/tmp".into(),
                channel_id: "9".into(),
                user_id: "1".into(),
                channel_name: "telegram",
            },
            &config,
        );
        let s = t.context.system_prompt_append.as_deref().expect("append");
        assert!(s.contains("CronCreate"));
        assert!(s.contains("scheduler.lock"));
    }

    #[test]
    fn channel_task_telegram_includes_ask_user_hint() {
        let config = minimal_test_config();
        let t = build_channel_task(
            ChannelTaskInput {
                agent_type: "workspace-assistant".into(),
                prompt: "hi".into(),
                working_directory: "/tmp".into(),
                channel_id: "9".into(),
                user_id: "1".into(),
                channel_name: "telegram",
            },
            &config,
        );
        let s = t.context.system_prompt_append.as_deref().expect("append");
        assert!(s.contains("Telegram AskUserQuestion"));
    }
}
