//! 系统提示多段合成（override / append / 默认段与记忆的优先级）。

use crate::model_instructions::ModelInstructionsConfig;
use crate::prompt_assembler::PromptAssembler;
use anycode_core::Agent;
use std::path::Path;

/// 运行时系统提示配置（通常来自 `config.json` + 解析后的 `@path` 文件内容）。
#[derive(Debug, Clone, Default)]
pub struct RuntimePromptConfig {
    /// 若非空：整段 system 仅此内容（不注入默认段、记忆、append）。
    pub system_prompt_override: Option<String>,
    /// 接在合成 system 末尾（在 per-task append 之前）。
    pub system_prompt_append: Option<String>,
    /// Injected after the tool list when not using override (from discovered `SKILL.md` skills).
    pub skills_section: Option<String>,
    pub workspace_section: Option<String>,
    pub channel_section: Option<String>,
    pub workflow_section: Option<String>,
    pub goal_section: Option<String>,
    #[allow(clippy::vec_box)]
    pub prompt_fragments: Vec<String>,
    /// Configuration for model instructions file discovery (AGENTS.md).
    pub model_instructions: ModelInstructionsConfig,
    /// Path to a model instructions file (e.g., `AGENTS.md`) whose content is injected into the system prompt.
    /// Supports absolute paths or paths relative to the working directory.
    pub model_instructions_file: Option<std::path::PathBuf>,
    /// Cached content of the model instructions file (resolved at runtime).
    pub model_instructions_content: Option<String>,
}

impl RuntimePromptConfig {
    /// Resolve and load the model instructions file content.
    /// If `model_instructions_file` is set, reads the file and stores content in `model_instructions_content`.
    /// If the file path is relative, it is resolved relative to `working_dir`.
    /// Returns `Ok(())` if successful or if no file is configured.
    /// Returns `Err` only on I/O errors when the file is configured but cannot be read.
    pub fn resolve_model_instructions_file(
        &mut self,
        working_dir: &Path,
    ) -> Result<(), std::io::Error> {
        let Some(ref path) = self.model_instructions_file else {
            return Ok(());
        };

        let resolved = if path.is_absolute() {
            path.clone()
        } else {
            working_dir.join(path)
        };

        if !resolved.is_file() {
            tracing::debug!(
                target: "anycode_agent",
                path = %resolved.display(),
                "model_instructions_file not found, skipping"
            );
            return Ok(());
        }

        match std::fs::read_to_string(&resolved) {
            Ok(content) => {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    tracing::info!(
                        target: "anycode_agent",
                        path = %resolved.display(),
                        len = trimmed.len(),
                        "loaded model instructions file"
                    );
                    self.model_instructions_content = Some(content);
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    target: "anycode_agent",
                    path = %resolved.display(),
                    error = %e,
                    "failed to read model_instructions_file"
                );
                Err(e)
            }
        }
    }

    /// Create a new RuntimePromptConfig with model instructions file set.
    pub fn with_model_instructions_file(mut self, path: Option<std::path::PathBuf>) -> Self {
        self.model_instructions_file = path;
        self
    }
}

fn env_section(cwd: &str) -> String {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let os = std::env::consts::OS;
    format!(
        "# Environment\n\n- Working directory: {}\n- OS: {}\n- Local date: {}",
        cwd, os, date
    )
}

pub(crate) fn default_stack_sections(
    agent: &dyn Agent,
    cwd: &str,
    skills_section: Option<&str>,
) -> Vec<String> {
    let mut parts = vec![
        "# System\n\nYou are an AI coding agent. Tool results arrive as separate messages; ground answers in them. Do not invent tool output.".to_string(),
        "# Tone\n\nBe concise. Prefer `file:line` for code references. Avoid emoji unless asked. Finish with plain language, not a promise to call a tool later.".to_string(),
        env_section(cwd),
        format!(
            "# Agent loop\n\nYou run in an agentic loop: the host executes tools and appends results as separate messages. When the user needs shell, file I/O, or repo search, emit the tool call **in this turn**. Do not ask the user to run commands you can run via Bash. On OpenAI-style tool gateways (e.g. GLM), when the task clearly needs tools, **the first assistant turn should contain tool_calls**—avoid long text-only preambles that defer execution.\n\nLines the user types that start with **`/`** in the TUI or REPL first line are **host slash commands** (not model API). Text inside this system message or other prompt templates that looks like `/foo` is **plain text** unless the product docs say otherwise.\n\n## Tools exposed to this agent\n\n{}",
            agent.tools().join(", ")
        ),
    ];
    if let Some(sk) = skills_section {
        let t = sk.trim();
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    parts.push("<!-- SYSTEM_PROMPT_DYNAMIC_BOUNDARY -->".to_string());
    parts
}

pub(crate) fn compose_default_sections(
    agent: &dyn Agent,
    cwd: &str,
    skills_section: Option<&str>,
) -> String {
    let mut parts = default_stack_sections(agent, cwd, skills_section);
    parts.push(format!(
        "# Custom Agent Instructions\n\n{}",
        agent.description()
    ));
    parts.join("\n\n")
}

/// 合成最终一条 system 文本（段之间双换行拼接）。
pub fn compose_effective_system_prompt(
    config: &RuntimePromptConfig,
    agent: &dyn Agent,
    cwd: &str,
    task_append: Option<&str>,
) -> String {
    if let Some(rep) = agent.system_prompt_replaces_default_sections() {
        let t = rep.trim();
        if !t.is_empty() {
            let mut out = vec![t.to_string()];
            if let Some(a) = config.system_prompt_append.as_deref() {
                if !a.trim().is_empty() {
                    out.push(a.trim().to_string());
                }
            }
            if let Some(a) = task_append {
                if !a.trim().is_empty() {
                    out.push(a.trim().to_string());
                }
            }
            return out.join("\n\n");
        }
    }
    PromptAssembler {
        config,
        agent,
        cwd,
        task_append,
    }
    .compose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::{AgentType, CoreError, Task, TaskResult, ToolName};
    use async_trait::async_trait;

    struct StubAgent {
        agent_type: AgentType,
        desc: &'static str,
        replace: Option<&'static str>,
        tools: Vec<ToolName>,
    }

    #[async_trait]
    impl Agent for StubAgent {
        fn agent_type(&self) -> &AgentType {
            &self.agent_type
        }

        fn description(&self) -> &str {
            self.desc
        }

        fn tools(&self) -> Vec<ToolName> {
            self.tools.clone()
        }

        async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
            unreachable!()
        }

        fn system_prompt_replaces_default_sections(&self) -> Option<&str> {
            self.replace
        }
    }

    fn stub(tools: Vec<ToolName>) -> StubAgent {
        StubAgent {
            agent_type: AgentType::new("stub"),
            desc: "agent-desc",
            replace: None,
            tools,
        }
    }

    #[test]
    fn override_is_only_body() {
        let cfg = RuntimePromptConfig {
            system_prompt_override: Some("OVERRIDE_ONLY".to_string()),
            system_prompt_append: Some("SHOULD_NOT_APPEAR".to_string()),
            skills_section: Some("SHOULD_NOT_APPEAR_SKILLS".into()),
            ..Default::default()
        };
        let agent = stub(vec!["A".to_string()]);
        let out = compose_effective_system_prompt(&cfg, &agent, "/tmp", Some("TASK"));
        assert_eq!(out, "OVERRIDE_ONLY");
    }

    #[test]
    fn append_order_config_then_task() {
        let cfg = RuntimePromptConfig {
            system_prompt_override: None,
            system_prompt_append: Some("FROM_CONFIG".to_string()),
            skills_section: None,
            ..Default::default()
        };
        let agent = stub(vec!["T".to_string()]);
        let out = compose_effective_system_prompt(&cfg, &agent, "/w", Some("FROM_TASK"));
        assert!(out.contains("# Custom Agent Instructions"));
        assert!(out.contains("FROM_CONFIG"));
        assert!(out.contains("FROM_TASK"));
        let pos_c = out.find("FROM_CONFIG").unwrap();
        let pos_t = out.find("FROM_TASK").unwrap();
        assert!(pos_c < pos_t, "config append before task append");
    }

    #[test]
    fn cwd_appears_in_default_stack() {
        let cfg = RuntimePromptConfig::default();
        let agent = stub(vec!["X".into()]);
        let out = compose_effective_system_prompt(&cfg, &agent, "/my/cwd", None);
        assert!(out.contains("/my/cwd"));
    }

    #[test]
    fn skills_section_injected_after_tool_list() {
        let cfg = RuntimePromptConfig {
            skills_section: Some("## Available skills\n\n- **demo**: test".into()),
            ..Default::default()
        };
        let agent = stub(vec!["Skill".into()]);
        let out = compose_effective_system_prompt(&cfg, &agent, "/w", None);
        let pos_tools = out.find("Skill").unwrap();
        let pos_sk = out.find("Available skills").unwrap();
        assert!(pos_sk > pos_tools);
    }

    #[test]
    fn agent_replace_skips_default_stack_but_keeps_append() {
        let cfg = RuntimePromptConfig {
            system_prompt_override: None,
            system_prompt_append: Some("TAIL".into()),
            skills_section: None,
            ..Default::default()
        };
        let mut agent = stub(vec!["Z".into()]);
        agent.replace = Some("CUSTOM_BODY");
        let out = compose_effective_system_prompt(&cfg, &agent, "/w", None);
        assert!(out.starts_with("CUSTOM_BODY"));
        assert!(!out.contains("# Tone"));
        assert!(out.contains("TAIL"));
    }

    #[test]
    fn model_instructions_content_injected_into_prompt() {
        let cfg = RuntimePromptConfig {
            model_instructions_content: Some("You are a helpful assistant.".into()),
            ..Default::default()
        };
        let agent = stub(vec!["T".into()]);
        let out = compose_effective_system_prompt(&cfg, &agent, "/w", None);
        assert!(out.contains("# Model Instructions"));
        assert!(out.contains("You are a helpful assistant."));
    }

    #[test]
    fn model_instructions_not_shown_when_override() {
        let cfg = RuntimePromptConfig {
            system_prompt_override: Some("OVERRIDE_ONLY".to_string()),
            model_instructions_content: Some("Should not appear".into()),
            ..Default::default()
        };
        let agent = stub(vec!["T".into()]);
        let out = compose_effective_system_prompt(&cfg, &agent, "/w", None);
        assert_eq!(out, "OVERRIDE_ONLY");
        assert!(!out.contains("Should not appear"));
    }

    #[test]
    fn resolve_model_instructions_file_absolute() {
        use std::io::Write;

        let tmpdir = tempfile::tempdir().unwrap();
        let instructions_path = tmpdir.path().join("AGENTS.md");
        let mut f = std::fs::File::create(&instructions_path).unwrap();
        writeln!(f, "# Custom Instructions\nBe helpful.").unwrap();

        let mut cfg = RuntimePromptConfig {
            model_instructions_file: Some(instructions_path.clone()),
            ..Default::default()
        };

        cfg.resolve_model_instructions_file(std::path::Path::new("/some/working/dir"))
            .unwrap();

        assert!(cfg.model_instructions_content.is_some());
        assert!(cfg
            .model_instructions_content
            .as_ref()
            .unwrap()
            .contains("Be helpful."));
    }

    #[test]
    fn resolve_model_instructions_file_relative() {
        use std::io::Write;

        let tmpdir = tempfile::tempdir().unwrap();
        let instructions_path = tmpdir.path().join("AGENTS.md");
        let mut f = std::fs::File::create(&instructions_path).unwrap();
        writeln!(f, "# Relative Instructions\nFollow these rules.").unwrap();

        let mut cfg = RuntimePromptConfig {
            model_instructions_file: Some(std::path::PathBuf::from("AGENTS.md")),
            ..Default::default()
        };

        cfg.resolve_model_instructions_file(tmpdir.path()).unwrap();

        assert!(cfg.model_instructions_content.is_some());
        assert!(cfg
            .model_instructions_content
            .as_ref()
            .unwrap()
            .contains("Follow these rules."));
    }

    #[test]
    fn resolve_model_instructions_file_missing_is_ok() {
        let mut cfg = RuntimePromptConfig {
            model_instructions_file: Some(std::path::PathBuf::from("/nonexistent/AGENTS.md")),
            ..Default::default()
        };

        // Should not error, just leaves content as None
        cfg.resolve_model_instructions_file(std::path::Path::new("/tmp"))
            .unwrap();

        assert!(cfg.model_instructions_content.is_none());
    }
}
