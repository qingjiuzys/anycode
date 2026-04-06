//! Structured system prompt assembly.

use crate::system_prompt::RuntimePromptConfig;
use anycode_core::{Agent, Memory, RuntimeMode, BUILTIN_SLASH_COMMANDS};

fn join_non_empty(parts: Vec<String>) -> String {
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn slash_commands_section() -> String {
    let mut lines = vec![
        "# Slash Commands".to_string(),
        "Prefer built-in slash commands when the runtime supports them:".to_string(),
    ];
    for cmd in BUILTIN_SLASH_COMMANDS {
        lines.push(format!("- /{}: {}", cmd.name, cmd.summary));
    }
    lines.join("\n")
}

fn relevant_memories_section(memories: &[Memory]) -> Option<String> {
    if memories.is_empty() {
        return None;
    }
    let mut lines = vec!["## Relevant Memories".to_string(), String::new()];
    for memory in memories {
        lines.push(format!("### {}", memory.title));
        lines.push(memory.content.clone());
        lines.push(String::new());
    }
    Some(lines.join("\n"))
}

pub struct PromptAssembler<'a> {
    pub config: &'a RuntimePromptConfig,
    pub agent: &'a dyn Agent,
    pub memories: &'a [Memory],
    pub cwd: &'a str,
    pub task_append: Option<&'a str>,
    pub mode: RuntimeMode,
}

impl<'a> PromptAssembler<'a> {
    pub fn compose(&self) -> String {
        if let Some(o) = self.config.system_prompt_override.as_deref() {
            let t = o.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }

        let mode_guidance = format!(
            "## Runtime Mode\nCurrent mode: `{}`. Respect this mode when choosing tools, response style, and execution bias.",
            self.mode.as_str()
        );
        let fragments = if self.config.prompt_fragments.is_empty() {
            None
        } else {
            Some(self.config.prompt_fragments.join("\n\n"))
        };

        let mut parts = vec![
            crate::system_prompt::compose_default_sections(
                self.agent,
                self.cwd,
                self.config.skills_section.as_deref(),
            ),
            mode_guidance,
            slash_commands_section(),
        ];
        if let Some(section) = self.config.workspace_section.as_deref() {
            parts.push(section.trim().to_string());
        }
        if let Some(section) = self.config.channel_section.as_deref() {
            parts.push(section.trim().to_string());
        }
        if let Some(section) = self.config.workflow_section.as_deref() {
            parts.push(section.trim().to_string());
        }
        if let Some(section) = self.config.goal_section.as_deref() {
            parts.push(section.trim().to_string());
        }
        if let Some(section) = relevant_memories_section(self.memories) {
            parts.push(section);
        }
        if let Some(section) = fragments {
            parts.push(section);
        }
        if let Some(a) = self.config.system_prompt_append.as_deref() {
            if !a.trim().is_empty() {
                parts.push(a.trim().to_string());
            }
        }
        if let Some(a) = self.task_append {
            if !a.trim().is_empty() {
                parts.push(a.trim().to_string());
            }
        }
        join_non_empty(parts)
    }
}
