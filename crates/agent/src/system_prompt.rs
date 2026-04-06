//! 系统提示多段合成（override / append / 默认段与记忆的优先级）。

use crate::prompt_assembler::PromptAssembler;
use anycode_core::{Agent, Memory};
use anycode_core::RuntimeMode;

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
            "# Agent loop\n\nYou run in an agentic loop: the host executes tools and appends results as separate messages. When the user needs shell, file I/O, or repo search, emit the tool call **in this turn**. Do not ask the user to run commands you can run via Bash. On OpenAI-style tool gateways (e.g. GLM), when the task clearly needs tools, **the first assistant turn should contain tool_calls**—avoid long text-only preambles that defer execution.\n\n## Tools exposed to this agent\n\n{}",
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
    memories: &[Memory],
    cwd: &str,
    task_append: Option<&str>,
    mode: RuntimeMode,
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
        memories,
        cwd,
        task_append,
        mode,
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
        let out = compose_effective_system_prompt(
            &cfg,
            &agent,
            &[],
            "/tmp",
            Some("TASK"),
            RuntimeMode::General,
        );
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
        let out = compose_effective_system_prompt(
            &cfg,
            &agent,
            &[],
            "/w",
            Some("FROM_TASK"),
            RuntimeMode::General,
        );
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
        let out = compose_effective_system_prompt(
            &cfg,
            &agent,
            &[],
            "/my/cwd",
            None,
            RuntimeMode::General,
        );
        assert!(out.contains("/my/cwd"));
    }

    #[test]
    fn skills_section_injected_after_tool_list() {
        let cfg = RuntimePromptConfig {
            skills_section: Some("## Available skills\n\n- **demo**: test".into()),
            ..Default::default()
        };
        let agent = stub(vec!["Skill".into()]);
        let out = compose_effective_system_prompt(
            &cfg,
            &agent,
            &[],
            "/w",
            None,
            RuntimeMode::General,
        );
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
        let out = compose_effective_system_prompt(
            &cfg,
            &agent,
            &[],
            "/w",
            None,
            RuntimeMode::General,
        );
        assert!(out.starts_with("CUSTOM_BODY"));
        assert!(!out.contains("# Tone"));
        assert!(out.contains("TAIL"));
    }
}
