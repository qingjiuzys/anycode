//! Structured system prompt assembly.

use crate::model_instructions::discover_model_instructions;
use crate::system_prompt::RuntimePromptConfig;
use anycode_core::{Agent, Memory, RuntimeMode, BUILTIN_SLASH_COMMANDS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptSegment {
    pub id: &'static str,
    pub text: String,
}

pub fn render_system_prompt_segments(segments: Vec<SystemPromptSegment>) -> String {
    segments
        .into_iter()
        .map(|seg| seg.text)
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn slash_commands_context_section() -> String {
    let mut lines = vec![
        "# Slash Commands".to_string(),
        "Prefer built-in slash commands when the runtime supports them:".to_string(),
    ];
    for cmd in BUILTIN_SLASH_COMMANDS {
        lines.push(format!("- /{}: {}", cmd.name, cmd.summary));
    }
    lines.join("\n")
}

pub fn runtime_mode_context_section(mode: RuntimeMode) -> String {
    format!(
        "## Runtime Mode\nCurrent mode: `{}`. Respect this mode when choosing tools, response style, and execution bias.",
        mode.as_str()
    )
}

pub fn relevant_memories_context_section(memories: &[Memory]) -> Option<String> {
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
    pub cwd: &'a str,
    pub task_append: Option<&'a str>,
}

impl<'a> PromptAssembler<'a> {
    pub fn build_segments(&self) -> Vec<SystemPromptSegment> {
        if let Some(o) = self.config.system_prompt_override.as_deref() {
            let t = o.trim();
            if !t.is_empty() {
                return vec![SystemPromptSegment {
                    id: "override",
                    text: t.to_string(),
                }];
            }
        }

        let mut segments = vec![SystemPromptSegment {
            id: "default_stack",
            text: crate::system_prompt::compose_default_sections(
                self.agent,
                self.cwd,
                self.config.skills_section.as_deref(),
            ),
        }];

        // Inject model instructions from external file (e.g., AGENTS.md)
        if let Some(content) = self.config.model_instructions_content.as_deref() {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                segments.push(SystemPromptSegment {
                    id: "model_instructions_file",
                    text: format!("# Model Instructions\n\n{}", trimmed),
                });
            }
        }

        if let Some(a) = self.config.system_prompt_append.as_deref() {
            if !a.trim().is_empty() {
                segments.push(SystemPromptSegment {
                    id: "config_append",
                    text: a.trim().to_string(),
                });
            }
        }

        // Discover and inject model instructions from AGENTS.md (or similar)
        if let Some(instructions) = discover_model_instructions(self.cwd, &self.config.model_instructions) {
            let path_display = instructions.path.display();
            segments.push(SystemPromptSegment {
                id: "model_instructions_discovered",
                text: format!(
                    "# Project Instructions\n\n<!-- Loaded from: {} -->\n\n{}",
                    path_display, instructions.content
                ),
            });
            tracing::debug!(
                target: "anycode_agent",
                "Loaded model instructions from: {}",
                path_display
            );
        }

        if let Some(a) = self.task_append {
            if !a.trim().is_empty() {
                segments.push(SystemPromptSegment {
                    id: "task_append",
                    text: a.trim().to_string(),
                });
            }
        }
        segments
    }

    pub fn compose(&self) -> String {
        render_system_prompt_segments(self.build_segments())
    }
}
