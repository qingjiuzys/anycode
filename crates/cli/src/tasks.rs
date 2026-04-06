//! Task entrypoints: `run`, REPL, `daemon`, listings, `test-security`, etc.

use crate::app_config::{apply_optional_repl_model, Config};
use crate::bootstrap::initialize_runtime;
use crate::builtin_agents::parse_agent_slash_command;
use crate::cli_args::SkillsCommands;
use crate::daemon_http;
use crate::i18n::{tr, tr_args};
use crate::repl_banner;
use crate::slash_commands::{self, ParsedSlashCommand};
use crate::workspace;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use anycode_security::SecurityLayer;
use anycode_tools::{default_skill_roots, iter_cli_tool_help, workflows, SkillCatalog};
use fluent_bundle::FluentArgs;
use std::collections::HashMap;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tracing::info;
use uuid::Uuid;

pub(crate) async fn run_interactive(
    mut config: Config,
    agent: String,
    directory: Option<PathBuf>,
    model: Option<String>,
    session_skip_approval: bool,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    apply_optional_repl_model(&mut config, model)?;

    let working_dir = directory.unwrap_or_else(|| std::env::current_dir().unwrap());
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    workspace::apply_project_overlays(&mut config, &working_dir);

    repl_banner::print_repl_welcome(&working_dir, &agent, session_skip_approval);

    let runtime = initialize_runtime(&config, None).await?;
    let disk = DiskTaskOutput::new_default()?;

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let mut agent = agent;

    loop {
        repl_banner::print_repl_prompt();
        let _ = std::io::stdout().flush();

        let line = match lines.next_line().await? {
            None => break,
            Some(l) => l,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(id) = parse_agent_slash_command(trimmed) {
            agent = id.to_string();
            let mut a = FluentArgs::new();
            a.set("id", id);
            println!("{}", tr_args("repl-agent-switched", &a));
            println!();
            continue;
        }

        if let Some(cmd) = slash_commands::parse(trimmed) {
            match cmd {
                ParsedSlashCommand::Mode(arg) => {
                    if let Some(mode) = arg {
                        if let Some(parsed) = RuntimeMode::parse(&mode) {
                            agent = parsed.default_agent().as_str().to_string();
                            println!("mode -> {} (agent: {})", parsed.as_str(), agent);
                        } else {
                            println!("unknown mode: {}", mode);
                        }
                    } else {
                        println!("current agent: {}", agent);
                    }
                }
                ParsedSlashCommand::Status => {
                    println!("agent: {}", agent);
                    println!("provider: {}", config.llm.provider);
                    println!("model: {}", config.llm.model);
                    println!("default_mode: {}", config.runtime.default_mode.as_str());
                }
                ParsedSlashCommand::Workflow(arg) => {
                    let maybe_path = arg.as_deref().and_then(|raw| {
                        let trimmed = raw.trim();
                        trimmed
                            .strip_prefix("run ")
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(PathBuf::from)
                    });
                    if let Some(path) = maybe_path {
                        run_workflow_path(
                            &runtime,
                            &disk,
                            &working_dir,
                            &path,
                            Some(trimmed.to_string()),
                        )
                        .await?;
                    } else {
                        match workflows::discover_workflow(&working_dir) {
                            Ok(Some((path, workflow))) => {
                                if arg.as_deref().map(|s| s.trim()) == Some("run") {
                                    run_workflow_definition(
                                        &runtime,
                                        &disk,
                                        &working_dir,
                                        &workflow,
                                        &path,
                                        Some(trimmed.to_string()),
                                    )
                                    .await?;
                                } else {
                                    println!("workflow: {} ({})", workflow.name, path.display());
                                }
                            }
                            Ok(None) => println!("workflow: none"),
                            Err(e) => println!("workflow error: {}", e),
                        }
                    }
                }
                ParsedSlashCommand::Model(arg) => {
                    if let Some(next) = arg {
                        println!(
                            "model switch requires a new session right now; current={} requested={}",
                            config.llm.model, next
                        );
                    } else {
                        println!("model: {}", config.llm.model);
                    }
                }
                ParsedSlashCommand::Compact => {
                    println!("compact is available in TUI/session runtime; use session auto-compact or the compact flow.");
                }
                ParsedSlashCommand::Memory => {
                    println!("memory backend: {}", config.memory.backend);
                }
                ParsedSlashCommand::Approve => {
                    println!("approval is handled by the active channel/runtime when required.");
                }
            }
            println!();
            continue;
        }

        match trimmed {
            "exit" | "quit" | ":q" | "/exit" => break,
            "help" | "?" | "/help" => {
                let mut h = FluentArgs::new();
                h.set("cwd", format!("{:?}", working_dir));
                h.set("agent", agent.clone());
                println!("{}", tr_args("repl-help-equiv", &h));
                println!("{}", tr("repl-help-cmds"));
                for line in slash_commands::help_lines() {
                    println!("{}", line);
                }
            }
            "agents" | "list-agents" | "/agents" => list_agents(),
            "tools" | "list-tools" | "/tools" => list_tools(),
            prompt => {
                run_single_task_with_tail(
                    &runtime,
                    &disk,
                    agent.clone(),
                    prompt.to_string(),
                    working_dir.clone(),
                )
                .await?;
            }
        }
        println!();
    }

    repl_banner::print_repl_goodbye();
    Ok(())
}

pub(crate) async fn run_task(
    mut config: Config,
    agent_type: Option<String>,
    mode: Option<String>,
    workflow: Option<PathBuf>,
    goal: Option<String>,
    prompt: String,
    working_dir: PathBuf,
) -> anyhow::Result<()> {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    workspace::apply_project_overlays(&mut config, &working_dir);
    let runtime = initialize_runtime(&config, None).await?;
    let disk = DiskTaskOutput::new_default()?;
    if let Some(workflow_path) = workflow {
        return run_workflow_path(&runtime, &disk, &working_dir, &workflow_path, Some(prompt))
            .await;
    }
    let resolved_mode = mode
        .as_deref()
        .and_then(RuntimeMode::parse)
        .unwrap_or(config.runtime.default_mode);
    let resolved_agent =
        agent_type.unwrap_or_else(|| resolved_mode.default_agent().as_str().to_string());
    if let Some(goal) = goal {
        return run_goal_task_with_tail(&runtime, &disk, resolved_agent, prompt, working_dir, goal)
            .await;
    }
    run_single_task_with_tail(&runtime, &disk, resolved_agent, prompt, working_dir).await
}

/// Single task execution shared by `run` / `repl` (disk tail + result printing).
pub(crate) async fn run_single_task_with_tail(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
) -> anyhow::Result<()> {
    info!("Running task with agent: {}", agent_type);
    info!("Working directory: {:?}", working_dir);
    info!("Prompt: {}", prompt);

    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);

    let task = Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(agent_type),
        prompt,
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: working_dir.to_string_lossy().to_string(),
            environment: HashMap::new(),
            user_id: None,
            system_prompt_append: None,
        },
        created_at: chrono::Utc::now(),
    };

    let output_path = disk.ensure_initialized(task.id)?;
    let mut po = FluentArgs::new();
    po.set("path", output_path.display().to_string());
    eprintln!("{}", tr_args("repl-task-out", &po));

    eprintln!("{}", tr("repl-task-run"));
    let exec = runtime.execute_task(task.clone());

    let mut offset: u64 = 0;
    tokio::pin!(exec);
    let result = loop {
        tokio::select! {
            res = &mut exec => break res?,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                let (delta, new_offset) = disk.read_delta(task.id, offset, 16 * 1024).unwrap_or_default();
                if !delta.is_empty() {
                    print!("{}", delta);
                    let _ = std::io::stdout().flush();
                    offset = new_offset;
                }
            }
        }
    };

    match result {
        TaskResult::Success { output, artifacts } => {
            eprintln!("{}", tr("repl-task-ok"));
            println!("\n{}\n{}", tr("repl-output-header"), output);
            let written = crate::artifact_summary::claude_turn_written_lines(&artifacts);
            if !written.is_empty() {
                eprintln!("\n{}", tr("repl-written-header"));
                for line in written {
                    let mut wl = FluentArgs::new();
                    wl.set("line", line);
                    eprintln!("{}", tr_args("repl-written-line", &wl));
                }
            }
        }
        TaskResult::Failure { error, details } => {
            let mut fe = FluentArgs::new();
            fe.set("err", error.to_string());
            eprintln!("{}", tr_args("repl-task-fail", &fe));
            if let Some(details) = details {
                let mut fd = FluentArgs::new();
                fd.set("details", details.to_string());
                eprintln!("{}", tr_args("repl-task-details", &fd));
            }
        }
        TaskResult::Partial { success, remaining } => {
            eprintln!("{}", tr("repl-task-partial"));
            let mut ps = FluentArgs::new();
            ps.set("done", success.to_string());
            eprintln!("{}", tr_args("repl-task-partial-done", &ps));
            let mut pr = FluentArgs::new();
            pr.set("rem", remaining.to_string());
            eprintln!("{}", tr_args("repl-task-partial-rem", &pr));
        }
    }

    Ok(())
}

async fn run_goal_task_with_tail(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    goal: String,
) -> anyhow::Result<()> {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    let task = build_task(agent_type, prompt, working_dir, None);
    let output_path = disk.ensure_initialized(task.id)?;
    eprintln!("goal output log: {}", output_path.display());
    let (result, progress) = runtime
        .execute_goal_task(
            task,
            GoalSpec {
                objective: goal,
                done_when: None,
                allow_infinite_retries: true,
            },
        )
        .await?;
    match result {
        TaskResult::Success { output, .. } => {
            println!("{}", output);
        }
        TaskResult::Failure { error, details } => {
            eprintln!("goal failed: {}", error);
            if let Some(details) = details {
                eprintln!("{}", details);
            }
        }
        TaskResult::Partial { success, remaining } => {
            println!("{}\n{}", success, remaining);
        }
    }
    eprintln!(
        "goal progress: attempts={} completed={} last_error={:?}",
        progress.attempts, progress.completed, progress.last_error
    );
    Ok(())
}

async fn run_workflow_path(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    working_dir: &Path,
    workflow_path: &Path,
    user_prompt: Option<String>,
) -> anyhow::Result<()> {
    let workflow = workflows::load_workflow_from_file(workflow_path)?;
    run_workflow_definition(
        runtime,
        disk,
        working_dir,
        &workflow,
        workflow_path,
        user_prompt,
    )
    .await
}

pub(crate) async fn execute_workflow_runtime(
    runtime: &AgentRuntime,
    working_dir: &Path,
    workflow_path: &Path,
    user_prompt: Option<String>,
) -> anyhow::Result<TaskResult> {
    let workflow = workflows::load_workflow_from_file(workflow_path)?;
    let retry_max = workflow
        .retry
        .as_ref()
        .map(|r| r.max_attempts.max(1))
        .unwrap_or(1);
    let retry_backoff_ms = workflow.retry.as_ref().map(|r| r.backoff_ms).unwrap_or(0);
    let mut current_mode = workflow
        .mode
        .as_deref()
        .and_then(RuntimeMode::parse)
        .unwrap_or(RuntimeMode::Code);
    let mut context_text = user_prompt.clone().unwrap_or_default();
    let mut last_result = TaskResult::Failure {
        error: "workflow produced no steps".to_string(),
        details: None,
    };
    for step in &workflow.steps {
        if !should_run_workflow_step(step, &context_text, &last_result) {
            continue;
        }
        let mode = step
            .mode
            .as_deref()
            .and_then(RuntimeMode::parse)
            .unwrap_or(current_mode);
        let agent = mode.default_agent().as_str().to_string();
        let prompt = render_workflow_prompt(
            user_prompt.clone().unwrap_or_default(),
            workflow.name.as_str(),
            step,
            step.done_when.as_deref().or(workflow.done_when.as_deref()),
        );
        let mut attempt = 0;
        loop {
            attempt += 1;
            let task = build_task(
                agent.clone(),
                prompt.clone(),
                working_dir.to_path_buf(),
                None,
            );
            let result = if mode == RuntimeMode::Goal {
                runtime
                    .execute_goal_task(
                        task,
                        GoalSpec {
                            objective: workflow
                                .done_when
                                .clone()
                                .unwrap_or_else(|| step.prompt.clone()),
                            done_when: workflow.done_when.clone(),
                            allow_infinite_retries: true,
                        },
                    )
                    .await
                    .map(|(r, _)| r)
            } else {
                runtime.execute_task(task).await
            };
            match result {
                Ok(r) => {
                    let failed = matches!(r, TaskResult::Failure { .. });
                    context_text = summarize_task_result(&r);
                    last_result = r;
                    if failed && attempt < retry_max {
                        if retry_backoff_ms > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                retry_backoff_ms,
                            ))
                            .await;
                        }
                        continue;
                    }
                    break;
                }
                Err(e) if attempt < retry_max => {
                    if retry_backoff_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_backoff_ms))
                            .await;
                    }
                    tracing::warn!(
                        "workflow step {} failed on attempt {}/{}: {}",
                        step.id,
                        attempt,
                        retry_max,
                        e
                    );
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }
    if let Some(handoff) = &workflow.handoff {
        if let Some(next_mode) = handoff.next_mode.as_deref().and_then(RuntimeMode::parse) {
            current_mode = next_mode;
        }
        if let Some(message) = &handoff.message {
            context_text.push('\n');
            context_text.push_str(message);
        }
        if handoff
            .message
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_some()
        {
            let agent = current_mode.default_agent().as_str().to_string();
            let handoff_prompt = format!(
                "{}\n\n## Workflow Handoff\nnext_mode={}\nmessage={}",
                user_prompt.clone().unwrap_or_default(),
                current_mode.as_str(),
                handoff.message.clone().unwrap_or_default()
            );
            let task = build_task(agent, handoff_prompt, working_dir.to_path_buf(), None);
            let result = if current_mode == RuntimeMode::Goal {
                runtime
                    .execute_goal_task(
                        task,
                        GoalSpec {
                            objective: handoff.message.clone().unwrap_or_default(),
                            done_when: workflow.done_when.clone(),
                            allow_infinite_retries: true,
                        },
                    )
                    .await
                    .map(|(r, _)| r)?
            } else {
                runtime.execute_task(task).await?
            };
            last_result = result;
        }
    }
    Ok(last_result)
}

async fn run_workflow_definition(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    working_dir: &Path,
    workflow: &WorkflowDefinition,
    workflow_path: &Path,
    user_prompt: Option<String>,
) -> anyhow::Result<()> {
    println!("workflow: {} ({})", workflow.name, workflow_path.display());
    let retry_max = workflow
        .retry
        .as_ref()
        .map(|r| r.max_attempts.max(1))
        .unwrap_or(1);
    let retry_backoff_ms = workflow.retry.as_ref().map(|r| r.backoff_ms).unwrap_or(0);
    let mut current_mode = workflow
        .mode
        .as_deref()
        .and_then(RuntimeMode::parse)
        .unwrap_or(RuntimeMode::Code);
    let mut context_text = user_prompt.clone().unwrap_or_default();
    let mut last_result = TaskResult::Failure {
        error: "workflow produced no steps".to_string(),
        details: None,
    };
    for step in &workflow.steps {
        if !should_run_workflow_step(step, &context_text, &last_result) {
            println!("workflow step {} skipped by `when`", step.id);
            continue;
        }
        let mode = step
            .mode
            .as_deref()
            .and_then(RuntimeMode::parse)
            .unwrap_or(current_mode);
        let agent = mode.default_agent().as_str().to_string();
        let prompt = render_workflow_prompt(
            user_prompt.clone().unwrap_or_default(),
            workflow.name.as_str(),
            step,
            step.done_when.as_deref().or(workflow.done_when.as_deref()),
        );
        let mut attempt = 0;
        loop {
            attempt += 1;
            let result = if mode == RuntimeMode::Goal {
                run_goal_task_with_tail(
                    runtime,
                    disk,
                    agent.clone(),
                    prompt.clone(),
                    working_dir.to_path_buf(),
                    step.done_when
                        .clone()
                        .or_else(|| workflow.done_when.clone())
                        .unwrap_or_else(|| step.prompt.clone()),
                )
                .await
            } else {
                run_single_task_with_tail(
                    runtime,
                    disk,
                    agent.clone(),
                    prompt.clone(),
                    working_dir.to_path_buf(),
                )
                .await
            };
            match result {
                Ok(()) => {
                    last_result = TaskResult::Success {
                        output: format!("workflow step {} completed", step.id),
                        artifacts: vec![],
                    };
                    context_text = format!("step {} completed", step.id);
                    break;
                }
                Err(e) if attempt < retry_max => {
                    eprintln!(
                        "workflow step {} failed (attempt {}/{}): {}",
                        step.id, attempt, retry_max, e
                    );
                    if retry_backoff_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_backoff_ms))
                            .await;
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
    if let Some(handoff) = &workflow.handoff {
        if let Some(next_mode) = handoff.next_mode.as_deref().and_then(RuntimeMode::parse) {
            current_mode = next_mode;
            println!("workflow handoff next_mode: {}", current_mode.as_str());
        }
        if let Some(message) = &handoff.message {
            println!("workflow handoff: {}", message);
            let agent = current_mode.default_agent().as_str().to_string();
            let prompt = format!(
                "{}\n\n## Workflow Handoff\nnext_mode={}\nmessage={}",
                user_prompt.clone().unwrap_or_default(),
                current_mode.as_str(),
                message
            );
            if current_mode == RuntimeMode::Goal {
                run_goal_task_with_tail(
                    runtime,
                    disk,
                    agent,
                    prompt,
                    working_dir.to_path_buf(),
                    workflow
                        .done_when
                        .clone()
                        .unwrap_or_else(|| message.clone()),
                )
                .await?;
            } else {
                run_single_task_with_tail(runtime, disk, agent, prompt, working_dir.to_path_buf())
                    .await?;
            }
        }
    }
    Ok(())
}

fn render_workflow_prompt(
    user_prompt: String,
    workflow_name: &str,
    step: &WorkflowStep,
    workflow_done_when: Option<&str>,
) -> String {
    let mut step_prompt = step.prompt.clone();
    for (key, value) in &step.vars {
        step_prompt = step_prompt.replace(&format!("{{{{{}}}}}", key), value);
    }
    let done_when = workflow_done_when.unwrap_or("step objective is complete");
    format!(
        "{}\n\n## Workflow\nname: {}\nstep_id: {}\ndone_when: {}\nstep_prompt: {}",
        user_prompt, workflow_name, step.id, done_when, step_prompt
    )
}

fn should_run_workflow_step(
    step: &WorkflowStep,
    context_text: &str,
    last_result: &TaskResult,
) -> bool {
    let Some(raw_when) = step.when.as_deref() else {
        return true;
    };
    let cond = raw_when.trim();
    if cond.is_empty() || cond.eq_ignore_ascii_case("always") {
        return true;
    }
    if let Some(needle) = cond.strip_prefix("contains:") {
        return context_text.contains(needle.trim());
    }
    if let Some(needle) = cond.strip_prefix("not_contains:") {
        return !context_text.contains(needle.trim());
    }
    if cond.eq_ignore_ascii_case("result_success") {
        return matches!(last_result, TaskResult::Success { .. });
    }
    if cond.eq_ignore_ascii_case("result_failure") {
        return matches!(last_result, TaskResult::Failure { .. });
    }
    true
}

fn summarize_task_result(result: &TaskResult) -> String {
    match result {
        TaskResult::Success { output, .. } => output.clone(),
        TaskResult::Failure { error, details } => details.clone().unwrap_or_else(|| error.clone()),
        TaskResult::Partial { success, remaining } => format!("{success}\n{remaining}"),
    }
}

fn build_task(
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    system_prompt_append: Option<String>,
) -> Task {
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
        },
        created_at: chrono::Utc::now(),
    }
}

pub(crate) fn list_agents() {
    use crate::builtin_agents::BUILTIN_AGENT_IDS;
    println!("{}", tr("repl-list-agents-title"));
    println!();
    for id in BUILTIN_AGENT_IDS {
        let desc = match id {
            "general-purpose" => tr("repl-agent-desc-gp"),
            "explore" => tr("repl-agent-desc-explore"),
            "plan" => tr("repl-agent-desc-plan"),
            "workspace-assistant" => "Workspace-first assistant for channel mode.".to_string(),
            "goal" => "Goal loop agent with retries and progress tracking.".to_string(),
            _ => String::new(),
        };
        println!("  • {}", id);
        if !desc.is_empty() {
            println!("{}", desc);
        }
        println!();
    }
    println!("{}", tr("repl-list-switch"));
    println!();
    println!("{}", tr("repl-list-usage"));
    println!("{}", tr("repl-list-usage-line"));
}

pub(crate) fn list_tools() {
    println!("{}", tr("repl-list-tools-title"));
    println!();
    for (name, desc) in iter_cli_tool_help() {
        println!("  • {}", name);
        println!("    {}", desc);
        println!();
    }
    println!("{}", tr("repl-security-title"));
    println!("{}", tr("repl-security-read"));
    println!("{}", tr("repl-security-approval"));
    println!("{}", tr("repl-security-sandbox"));
}

pub(crate) async fn run_daemon(config: Config, bind: String) -> anyhow::Result<()> {
    info!("Starting daemon mode on {}", bind);

    let runtime = initialize_runtime(&config, None).await?;
    let addr: SocketAddr = bind.parse().map_err(|e: std::net::AddrParseError| {
        let mut a = FluentArgs::new();
        a.set("bind", bind.clone());
        a.set("err", e.to_string());
        anyhow::anyhow!("{}", tr_args("repl-err-invalid-bind", &a))
    })?;

    let mut da = FluentArgs::new();
    da.set("addr", addr.to_string());
    println!("{}", tr_args("repl-daemon-http", &da));
    println!("{}", tr("repl-daemon-get"));
    println!("{}", tr("repl-daemon-post"));
    println!(
        "       {}",
        r#"{"agent":"general-purpose","prompt":"…","working_directory":null}"#
    );
    if std::env::var("ANYCODE_DAEMON_TOKEN")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        println!("{}", tr("repl-daemon-token-hint"));
    }
    println!("{}", tr("repl-daemon-stop-hint"));

    tokio::select! {
        res = daemon_http::serve(addr, runtime, std::sync::Arc::new(config)) => res,
        _ = tokio::signal::ctrl_c() => {
            println!("\n{}", tr("repl-daemon-shutdown"));
            Ok(())
        }
    }
}

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
    use super::*;

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
