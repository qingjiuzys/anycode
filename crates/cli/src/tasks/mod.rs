//! Task entrypoints: `run`, REPL, listings, `test-security`, etc.

mod tasks_repl;
mod tasks_run;
mod tasks_sink;
#[path = "../tasks_workflow.rs"]
mod workflow_exec;

pub(crate) use tasks_repl::run_interactive;
pub(crate) use tasks_run::run_task;

use crate::app_config::Config;
use crate::cli_args::SkillsCommands;
use crate::i18n::{tr, tr_args};
use anycode_core::PermissionMode;
use anycode_security::SecurityLayer;
use anycode_tools::{default_skill_roots, SkillCatalog};
use fluent_bundle::FluentArgs;

fn build_skill_catalog_for_cli(config: &Config) -> SkillCatalog {
    if config.skills.enabled {
        let roots = default_skill_roots(&config.skills.extra_dirs, dirs::home_dir().as_deref());
        SkillCatalog::scan(
            &roots,
            config.skills.allowlist.as_deref(),
            config.skills.run_timeout_ms,
            config.skills.minimal_env,
        )
    } else {
        SkillCatalog::scan(
            &[],
            None,
            config.skills.run_timeout_ms,
            config.skills.minimal_env,
        )
    }
}

pub(crate) fn run_skills_command(config: &Config, sub: SkillsCommands) -> anyhow::Result<()> {
    match sub {
        SkillsCommands::List => {
            let cat = build_skill_catalog_for_cli(config);
            if cat.is_empty() {
                if !config.skills.enabled {
                    println!("(no skills: skills.enabled is false — only project `<cwd>/skills` / `.anycode/skills` resolve at run time; enable scanning in config to list them here)");
                } else {
                    println!("(no skills found under configured roots)");
                }
                return Ok(());
            }
            println!("id\thas_run\tdescription\troot");
            for m in cat.metas() {
                let run = if m.has_run { "yes" } else { "no" };
                println!(
                    "{}\t{}\t{}\t{}",
                    m.id,
                    run,
                    m.description.replace('\t', " "),
                    m.root_dir.display()
                );
            }
            Ok(())
        }
        SkillsCommands::Path => {
            let roots = default_skill_roots(&config.skills.extra_dirs, dirs::home_dir().as_deref());
            println!("skills.enabled: {}", config.skills.enabled);
            for r in roots {
                println!("{}", r.display());
            }
            Ok(())
        }
        SkillsCommands::Init { name } => {
            let id = name.trim();
            if id.is_empty() {
                anyhow::bail!("skill name must not be empty");
            }
            if !SkillCatalog::is_valid_skill_id(id) {
                anyhow::bail!(
                    "invalid skill id {:?}: use only ASCII letters, digits, `.`, `_`, `-`",
                    id
                );
            }
            let Some(home) = dirs::home_dir() else {
                anyhow::bail!("could not resolve home directory");
            };
            let root = home.join(".anycode/skills");
            let dir = root.join(id);
            if dir.exists() {
                anyhow::bail!("already exists: {}", dir.display());
            }
            std::fs::create_dir_all(&root)?;
            std::fs::create_dir_all(&dir)?;
            let skill_md = format!(
                "---\nname: {id}\ndescription: TODO describe this skill\n---\n\n# {id}\n\n"
            );
            std::fs::write(dir.join("SKILL.md"), skill_md)?;
            let run_script = format!(
                "#!/usr/bin/env bash\nset -euo pipefail\necho \"skill {id}: implement me\"\n"
            );
            let run_path = dir.join("run");
            std::fs::write(&run_path, run_script)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&run_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&run_path, perms)?;
            }
            println!("{}", dir.display());
            Ok(())
        }
    }
}

pub(crate) async fn test_security_system(tool: String, input: String) -> anyhow::Result<()> {
    use anycode_security::{InteractiveApprovalCallback, PromptFormat};

    let security_layer = SecurityLayer::new(PermissionMode::Default);

    // Approval callback (CLI prompts) — kept for parity with interactive flows.
    let _callback = Box::new(InteractiveApprovalCallback::new(PromptFormat::CLI));

    let input_value: serde_json::Value = serde_json::from_str(&input)?;
    let mut tc = FluentArgs::new();
    tc.set("tool", tool.clone());
    println!("{}", tr_args("repl-test-checking", &tc));
    let mut ti = FluentArgs::new();
    ti.set("input", input.clone());
    println!("{}", tr_args("repl-test-input", &ti));

    match security_layer.check_tool_call(&tool, &input_value).await {
        Ok(approved) => {
            if approved {
                println!("{}", tr("repl-test-approved"));
            } else {
                println!("{}", tr("repl-test-denied"));
            }
        }
        Err(e) => {
            let mut te = FluentArgs::new();
            te.set("err", e.to_string());
            println!("{}", tr_args("repl-test-error", &te));
        }
    }

    Ok(())
}

#[cfg(test)]
mod workflow_runtime_tests {
    use super::workflow_exec::{render_workflow_prompt, should_run_workflow_step};
    use anycode_core::prelude::*;
    use anycode_core::WorkflowStep;
    use std::collections::HashMap;

    fn step(id: &str, when: Option<&str>, prompt: &str) -> WorkflowStep {
        WorkflowStep {
            id: id.to_string(),
            prompt: prompt.to_string(),
            when: when.map(|s| s.to_string()),
            mode: None,
            model: None,
            done_when: None,
            vars: HashMap::new(),
        }
    }

    #[test]
    fn workflow_step_when_contains_matches_context() {
        let step = step("s1", Some("contains:alpha"), "do it");
        let result = TaskResult::Success {
            output: "ok".to_string(),
            artifacts: vec![],
        };
        assert!(should_run_workflow_step(&step, "alpha beta", &result));
        assert!(!should_run_workflow_step(&step, "beta", &result));
    }

    #[test]
    fn workflow_step_when_result_failure_matches_failure() {
        let step = step("s1", Some("result_failure"), "retry it");
        let result = TaskResult::Failure {
            error: "boom".to_string(),
            details: None,
        };
        assert!(should_run_workflow_step(&step, "", &result));
    }

    #[test]
    fn render_workflow_prompt_expands_vars() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "anycode".to_string());
        let step = WorkflowStep {
            id: "s1".to_string(),
            prompt: "hello {{name}}".to_string(),
            when: None,
            mode: None,
            model: None,
            done_when: Some("done".to_string()),
            vars,
        };
        let rendered =
            render_workflow_prompt("ctx".to_string(), "wf", &step, step.done_when.as_deref());
        assert!(rendered.contains("hello anycode"));
        assert!(rendered.contains("done_when: done"));
    }
}
