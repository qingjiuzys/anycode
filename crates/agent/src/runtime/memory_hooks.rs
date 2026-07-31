//! Memory pipeline hooks, session notifications, and project autosave.

use super::artifacts::truncate_text;
use super::session_notify::{build_notification_value, spawn_dispatch};
use super::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::{looks_like_secret, EpisodeEvent, EpisodeRecord};
use tracing::warn;

const MEMORY_AUTOSAVE_TITLE_MAX_CHARS: usize = 200;
const MEMORY_AUTOSAVE_CONTENT_MAX_BYTES: usize = 64 * 1024;
const STRUCTURED_OUTCOME_MAX: usize = 480;

pub(super) fn last_user_plain_text_for_autosave(msgs: &[Message]) -> String {
    msgs.iter()
        .rev()
        .find_map(|m| {
            if m.role != MessageRole::User {
                return None;
            }
            // Skip compiler-injected context (Task Spec / Gate Plan / repair
            // diagnostics) — autosave must use the real user turn as its title.
            let injected = m
                .metadata
                .get(ANYCODE_CONTEXT_USER_METADATA_KEY)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if injected {
                return None;
            }
            match &m.content {
                MessageContent::Text(t) if !t.trim().is_empty() => Some(t.clone()),
                _ => None,
            }
        })
        .unwrap_or_default()
}

fn memory_root() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/memory"))
}

fn record_episode_event(session_label: &str, task_id: TaskId, event: EpisodeEvent) {
    if looks_like_secret(&event.to_structured_text()) {
        return;
    }
    let Some(root) = memory_root() else {
        return;
    };
    let mut record = EpisodeRecord {
        id: format!("ep_{}", uuid::Uuid::new_v4()),
        session_id: session_label.to_string(),
        task_id: task_id.to_string(),
        events: vec![event],
        created_at: chrono::Utc::now(),
        evidence_hash: String::new(),
    };
    record.recompute_hash();
    if let Err(e) = anycode_memory::append_episode(&root, record) {
        warn!(target: "anycode_agent", "episode append failed: {}", e);
    }
}

impl AgentRuntime {
    pub(super) async fn pipeline_memory_hook_tool_result(
        &self,
        session_label: &str,
        task_id: TaskId,
        tool_name: &str,
        tool_text: &str,
    ) {
        let Some(ref pipe) = self.memory_pipeline else {
            return;
        };
        let Some(ref s) = self.memory_pipeline_settings else {
            return;
        };
        if !s.hook_after_tool_result {
            return;
        }
        if s.hook_tool_deny_prefixes
            .iter()
            .any(|p| tool_name.starts_with(p.as_str()))
        {
            return;
        }
        if looks_like_secret(tool_text) {
            return;
        }
        let (body, _) = truncate_text(
            tool_text.to_string(),
            STRUCTURED_OUTCOME_MAX.min(s.hook_max_bytes),
        );
        let ok = !body.to_ascii_lowercase().contains("error");
        let event = EpisodeEvent::ToolTrace {
            tool: tool_name.to_string(),
            outcome: body.clone(),
            ok,
        };
        record_episode_event(session_label, task_id, event.clone());
        let text = event.to_structured_text();
        let sess = format!("{}:{}", session_label, task_id);
        if let Err(e) = pipe
            .ingest_fragment(&sess, &text, MemoryType::Project)
            .await
        {
            warn!(target: "anycode_agent", "memory pipeline hook (tool): {}", e);
        }
    }

    pub(super) async fn pipeline_memory_hook_agent_turn(
        &self,
        session_label: &str,
        task_id: TaskId,
        turn: usize,
        assistant_excerpt: &str,
    ) {
        let Some(ref pipe) = self.memory_pipeline else {
            return;
        };
        let Some(ref s) = self.memory_pipeline_settings else {
            return;
        };
        if !s.hook_after_agent_turn {
            return;
        }
        let t = assistant_excerpt.trim();
        if t.is_empty() || looks_like_secret(t) {
            return;
        }
        let (body, _) = truncate_text(t.to_string(), STRUCTURED_OUTCOME_MAX.min(s.hook_max_bytes));
        let event = EpisodeEvent::KeyDecision {
            decision: format!("turn {turn} summary"),
            rationale: body,
        };
        record_episode_event(session_label, task_id, event.clone());
        let text = event.to_structured_text();
        let sess = format!("{}:{}", session_label, task_id);
        if let Err(e) = pipe
            .ingest_fragment(&sess, &text, MemoryType::Project)
            .await
        {
            warn!(target: "anycode_agent", "memory pipeline hook (turn): {}", e);
        }
    }

    pub(super) fn maybe_session_notify_tool_result(
        &self,
        session_label: &str,
        task_id: TaskId,
        turn: usize,
        tool_name: &str,
        tool_text: &str,
        cwd: Option<&str>,
    ) {
        let Some(ref cfg) = self.session_notifications else {
            return;
        };
        if !cfg.after_tool_result || !cfg.is_configured() {
            return;
        }
        if cfg
            .tool_deny_prefixes
            .iter()
            .any(|p| tool_name.starts_with(p.as_str()))
        {
            return;
        }
        let payload = build_notification_value(
            "tool_result",
            session_label,
            task_id,
            turn,
            Some(tool_name),
            tool_text,
            cwd,
            cfg.max_body_bytes,
        );
        spawn_dispatch(cfg.clone(), payload);
    }

    pub(super) fn maybe_session_notify_agent_turn(
        &self,
        session_label: &str,
        task_id: TaskId,
        turn: usize,
        assistant_excerpt: &str,
        cwd: Option<&str>,
    ) {
        let Some(ref cfg) = self.session_notifications else {
            return;
        };
        if !cfg.after_agent_turn || !cfg.is_configured() {
            return;
        }
        let t = assistant_excerpt.trim();
        if t.is_empty() {
            return;
        }
        let payload = build_notification_value(
            "agent_turn",
            session_label,
            task_id,
            turn,
            None,
            t,
            cwd,
            cfg.max_body_bytes,
        );
        spawn_dispatch(cfg.clone(), payload);
    }

    pub(super) async fn maybe_autosave_memory(&self, task_id: TaskId, prompt: &str, output: &str) {
        if !self.memory_project_autosave_enabled {
            return;
        }
        if looks_like_secret(prompt) || looks_like_secret(output) {
            return;
        }
        let line0 = prompt.lines().next().unwrap_or("").trim();
        let title = if line0.chars().count() > MEMORY_AUTOSAVE_TITLE_MAX_CHARS {
            line0
                .chars()
                .take(MEMORY_AUTOSAVE_TITLE_MAX_CHARS)
                .collect::<String>()
        } else {
            line0.to_string()
        };
        let title = if title.is_empty() {
            "(empty prompt)".to_string()
        } else {
            title
        };
        let (content, _) = truncate_text(output.to_string(), MEMORY_AUTOSAVE_CONTENT_MAX_BYTES);
        record_episode_event(
            "autosave",
            task_id,
            EpisodeEvent::TaskIntent {
                summary: title.clone(),
                family: String::new(),
            },
        );
        record_episode_event(
            "autosave",
            task_id,
            EpisodeEvent::Deliverable {
                path_or_title: title.clone(),
                summary: content.chars().take(STRUCTURED_OUTCOME_MAX).collect(),
            },
        );
        if let Some(ref pipe) = self.memory_pipeline {
            let session = task_id.to_string();
            let text = format!("{}\n\n{}", title, content);
            if let Err(e) = pipe
                .ingest_fragment(&session, &text, MemoryType::Project)
                .await
            {
                warn!(
                    target: "anycode_agent",
                    "memory pipeline ingest (auto_save) failed: {}",
                    e
                );
            }
            return;
        }
        let now = chrono::Utc::now();
        let memory = Memory {
            id: task_id.to_string(),
            mem_type: MemoryType::Project,
            title,
            content,
            tags: vec![],
            scope: MemoryScope::Project,
            created_at: now,
            updated_at: now,
            meta: None,
        };
        if let Err(e) = self.memory_store.save(memory).await {
            warn!(target: "anycode_agent", "memory auto_save failed: {}", e);
        }
    }

    pub(super) async fn maybe_record_verify_recipe(
        &self,
        session_label: &str,
        task_id: TaskId,
        command: &str,
        tool_text: &str,
        cwd: &str,
    ) {
        let Some(recipe) =
            super::discoverable_verification::try_verify_recipe_from_bash(command, tool_text, cwd)
        else {
            return;
        };
        if looks_like_secret(&recipe) {
            return;
        }
        record_episode_event(
            session_label,
            task_id,
            EpisodeEvent::KeyDecision {
                decision: "verify_recipe".into(),
                rationale: recipe.clone(),
            },
        );
        if let Some(ref pipe) = self.memory_pipeline {
            let sess = format!("{session_label}:{task_id}");
            if let Err(e) = pipe
                .ingest_fragment(&sess, &recipe, MemoryType::Project)
                .await
            {
                warn!(
                    target: "anycode_agent",
                    "memory pipeline verify_recipe failed: {}",
                    e
                );
            }
            return;
        }
        let now = chrono::Utc::now();
        let memory = Memory {
            id: format!("verify_recipe_{task_id}"),
            mem_type: MemoryType::Project,
            title: "verify_recipe".into(),
            content: recipe,
            tags: vec!["verify_recipe".into()],
            scope: MemoryScope::Project,
            created_at: now,
            updated_at: now,
            meta: None,
        };
        if let Err(e) = self.memory_store.save(memory).await {
            warn!(target: "anycode_agent", "verify_recipe memory save failed: {}", e);
        }
    }
}
