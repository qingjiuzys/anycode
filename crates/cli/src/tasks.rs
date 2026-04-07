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
use std::io::{IsTerminal, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::info;
use uuid::Uuid;

/// REPL 输出目标：管道/工作流走真实终端；TTY 全屏写入 transcript 并在每次增量后重绘。
pub(crate) enum ReplSink {
    Stdio,
    Tui {
        transcript: Arc<Mutex<String>>,
        on_flush: Arc<dyn Fn() + Send + Sync>,
    },
}

impl ReplSink {
    pub(crate) fn line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => println!("{s}"),
            ReplSink::Tui {
                transcript,
                on_flush,
            } => {
                let mut t = transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                t.push('\n');
                drop(t);
                on_flush();
            }
        }
    }

    /// 与 `eprintln!` 对齐的 stderr 行；TTY 下仍进入 transcript（与原先一致）。
    pub(crate) fn eprint_line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => eprintln!("{s}"),
            ReplSink::Tui {
                transcript,
                on_flush,
            } => {
                let mut t = transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                t.push('\n');
                drop(t);
                on_flush();
            }
        }
    }

    pub(crate) fn push_stdout_str(&mut self, s: &str) {
        match self {
            ReplSink::Stdio => {
                print!("{s}");
                let _ = std::io::stdout().flush();
            }
            ReplSink::Tui {
                transcript,
                on_flush,
            } => {
                let mut t = transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                drop(t);
                on_flush();
            }
        }
    }
}

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

    let runtime = initialize_runtime(&config, None).await?;
    let disk = DiskTaskOutput::new_default()?;

    let mut agent = agent;

    if std::io::stdin().is_terminal() {
        run_interactive_tty(&runtime, &disk, &working_dir, &mut agent, &mut config).await?;
    } else {
        repl_banner::print_repl_welcome(&working_dir, &agent, session_skip_approval);
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
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
            let mut sink = ReplSink::Stdio;
            if repl_dispatch_one_line(
                &runtime,
                &disk,
                &working_dir,
                &mut agent,
                &mut config,
                trimmed,
                &mut sink,
            )
            .await?
            {
                break;
            }
        }
    }

    repl_banner::print_repl_goodbye();
    Ok(())
}

async fn run_interactive_tty(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    working_dir: &PathBuf,
    agent: &mut String,
    config: &mut Config,
) -> anyhow::Result<()> {
    use crate::repl_inline::{handle_event, ReplCtl, ReplLineState, ReplTerminalGuard};
    use crossterm::event;

    let guard = Arc::new(Mutex::new(ReplTerminalGuard::new()?));
    let state = Arc::new(Mutex::new(ReplLineState::default()));
    let mut head = FluentArgs::new();
    head.set("version", env!("CARGO_PKG_VERSION"));
    head.set("cwd", working_dir.display().to_string());
    head.set("agent", agent.as_str());
    {
        let st = state.lock().unwrap_or_else(|e| e.into_inner());
        let mut buf = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
        buf.push_str(&tr_args("repl-tty-title", &head));
        buf.push('\n');
        buf.push_str(&tr_args("repl-tty-sub", &head));
        buf.push_str("\n\n");
        buf.push_str(&tr("repl-hint-completion"));
        buf.push_str("\n\n");
    }

    let redraw: Arc<dyn Fn() + Send + Sync> = {
        let state = state.clone();
        let guard = guard.clone();
        Arc::new(move || {
            let Ok(st) = state.lock() else {
                return;
            };
            let Ok(mut g) = guard.lock() else {
                return;
            };
            let _ = g.draw(&*st);
        })
    };

    loop {
        {
            let st = state.lock().unwrap_or_else(|e| e.into_inner());
            let mut g = guard.lock().unwrap_or_else(|e| e.into_inner());
            g.draw(&*st)?;
        }
        let ev = tokio::task::spawn_blocking(|| event::read()).await??;
        let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
        match handle_event(ev, &mut *s)? {
            ReplCtl::Continue => {}
            ReplCtl::Submit(text) => {
                let t = crate::tui::util::trim_or_default(text.as_str());
                if t.is_empty() {
                    continue;
                }
                let workflow_esc = t.trim_start().starts_with("/workflow");
                drop(s);
                let done = if workflow_esc {
                    guard
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .suspend_for_output()?;
                    let mut sink = ReplSink::Stdio;
                    let d = repl_dispatch_one_line(
                        runtime,
                        disk,
                        working_dir,
                        agent,
                        config,
                        t,
                        &mut sink,
                    )
                    .await?;
                    guard
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .resume_after_output()?;
                    if !d {
                        let st = state.lock().unwrap_or_else(|e| e.into_inner());
                        let mut buf = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
                        buf.push_str(&tr("repl-tty-workflow-note"));
                        buf.push_str("\n\n");
                    }
                    d
                } else {
                    let tr = {
                        let st = state.lock().unwrap_or_else(|e| e.into_inner());
                        st.transcript.clone()
                    };
                    let mut sink = ReplSink::Tui {
                        transcript: tr,
                        on_flush: redraw.clone(),
                    };
                    repl_dispatch_one_line(runtime, disk, working_dir, agent, config, t, &mut sink)
                        .await?
                };
                {
                    let st = state.lock().unwrap_or_else(|e| e.into_inner());
                    let mut g = guard.lock().unwrap_or_else(|e| e.into_inner());
                    g.draw(&*st)?;
                }
                if done {
                    break;
                }
            }
            ReplCtl::Eof => break,
        }
    }

    Ok(())
}

/// 处理单行 REPL 输入；返回 `true` 表示应退出循环。
/// `ReplSink::Tui` 写入内嵌 transcript 并在增量后重绘；`Stdio` 走真实终端。
async fn repl_dispatch_one_line(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    working_dir: &PathBuf,
    agent: &mut String,
    config: &mut Config,
    trimmed: &str,
    sink: &mut ReplSink,
) -> anyhow::Result<bool> {
    if let Some(id) = parse_agent_slash_command(trimmed) {
        *agent = id.to_string();
        let mut a = FluentArgs::new();
        a.set("id", id);
        sink.line(tr_args("repl-agent-switched", &a));
        sink.line("");
        return Ok(false);
    }

    if let Some(cmd) = slash_commands::parse(trimmed) {
        match cmd {
            ParsedSlashCommand::Mode(arg) => {
                if let Some(mode) = arg {
                    if let Some(parsed) = RuntimeMode::parse(&mode) {
                        *agent = parsed.default_agent().as_str().to_string();
                        sink.line(format!("mode -> {} (agent: {})", parsed.as_str(), agent));
                    } else {
                        sink.line(format!("unknown mode: {}", mode));
                    }
                } else {
                    sink.line(format!("current agent: {}", agent));
                }
            }
            ParsedSlashCommand::Status => {
                sink.line(format!("agent: {}", agent));
                sink.line(format!("provider: {}", config.llm.provider));
                sink.line(format!("model: {}", config.llm.model));
                sink.line(format!(
                    "default_mode: {}",
                    config.runtime.default_mode.as_str()
                ));
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
                    run_workflow_path(runtime, disk, working_dir, &path, Some(trimmed.to_string()))
                        .await?;
                } else {
                    match workflows::discover_workflow(working_dir) {
                        Ok(Some((path, workflow))) => {
                            if arg.as_deref().map(|s| s.trim()) == Some("run") {
                                run_workflow_definition(
                                    runtime,
                                    disk,
                                    working_dir,
                                    &workflow,
                                    &path,
                                    Some(trimmed.to_string()),
                                )
                                .await?;
                            } else {
                                sink.line(format!(
                                    "workflow: {} ({})",
                                    workflow.name,
                                    path.display()
                                ));
                            }
                        }
                        Ok(None) => sink.line("workflow: none"),
                        Err(e) => sink.line(format!("workflow error: {}", e)),
                    }
                }
            }
            ParsedSlashCommand::Model(arg) => {
                if let Some(next) = arg {
                    sink.line(format!(
                        "model switch requires a new session right now; current={} requested={}",
                        config.llm.model, next
                    ));
                } else {
                    sink.line(format!("model: {}", config.llm.model));
                }
            }
            ParsedSlashCommand::Compact => {
                sink.line(
                    "compact is available in TUI/session runtime; use session auto-compact or the compact flow.",
                );
            }
            ParsedSlashCommand::Memory => {
                sink.line(format!("memory backend: {}", config.memory.backend));
            }
            ParsedSlashCommand::Approve => {
                sink.line("approval is handled by the active channel/runtime when required.");
            }
        }
        sink.line("");
        return Ok(false);
    }

    match trimmed {
        "exit" | "quit" | ":q" | "/exit" => return Ok(true),
        "help" | "?" | "/help" => {
            let mut h = FluentArgs::new();
            h.set("cwd", format!("{:?}", working_dir));
            h.set("agent", agent.clone());
            sink.line(tr_args("repl-help-equiv", &h));
            sink.line(tr("repl-help-cmds"));
            for line in slash_commands::help_lines() {
                sink.line(line);
            }
        }
        "agents" | "list-agents" | "/agents" => list_agents(sink),
        "tools" | "list-tools" | "/tools" => list_tools(sink),
        prompt => {
            run_single_task_with_tail(
                runtime,
                disk,
                agent.clone(),
                prompt.to_string(),
                working_dir.clone(),
                sink,
            )
            .await?;
        }
    }
    sink.line("");
    Ok(false)
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
    let mut sink = ReplSink::Stdio;
    run_single_task_with_tail(
        &runtime,
        &disk,
        resolved_agent,
        prompt,
        working_dir,
        &mut sink,
    )
    .await
}

/// Single task execution shared by `run` / `repl` (disk tail + result printing)。
/// `ReplSink::Tui` 写入 transcript 并在 tail 增量时刷新界面。
pub(crate) async fn run_single_task_with_tail(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    sink: &mut ReplSink,
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
    sink.eprint_line(tr_args("repl-task-out", &po));

    sink.eprint_line(tr("repl-task-run"));
    let exec = runtime.execute_task(task.clone());

    let mut offset: u64 = 0;
    tokio::pin!(exec);
    let result = loop {
        tokio::select! {
            res = &mut exec => break res?,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                let (delta, new_offset) = disk.read_delta(task.id, offset, 16 * 1024).unwrap_or_default();
                if !delta.is_empty() {
                    sink.push_stdout_str(&delta);
                    offset = new_offset;
                }
            }
        }
    };

    match result {
        TaskResult::Success { output, artifacts } => {
            sink.eprint_line(tr("repl-task-ok"));
            sink.line("");
            sink.line(tr("repl-output-header"));
            sink.line(output);
            let written = crate::artifact_summary::claude_turn_written_lines(&artifacts);
            if !written.is_empty() {
                sink.line("");
                sink.eprint_line(tr("repl-written-header"));
                for line in written {
                    let mut wl = FluentArgs::new();
                    wl.set("line", line);
                    sink.eprint_line(tr_args("repl-written-line", &wl));
                }
            }
        }
        TaskResult::Failure { error, details } => {
            let mut fe = FluentArgs::new();
            fe.set("err", error.to_string());
            sink.eprint_line(tr_args("repl-task-fail", &fe));
            if let Some(details) = details {
                let mut fd = FluentArgs::new();
                fd.set("details", details.to_string());
                sink.eprint_line(tr_args("repl-task-details", &fd));
            }
        }
        TaskResult::Partial { success, remaining } => {
            sink.eprint_line(tr("repl-task-partial"));
            let mut ps = FluentArgs::new();
            ps.set("done", success.to_string());
            sink.eprint_line(tr_args("repl-task-partial-done", &ps));
            let mut pr = FluentArgs::new();
            pr.set("rem", remaining.to_string());
            sink.eprint_line(tr_args("repl-task-partial-rem", &pr));
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
                let mut sink = ReplSink::Stdio;
                run_single_task_with_tail(
                    runtime,
                    disk,
                    agent.clone(),
                    prompt.clone(),
                    working_dir.to_path_buf(),
                    &mut sink,
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
                let mut sink = ReplSink::Stdio;
                run_single_task_with_tail(
                    runtime,
                    disk,
                    agent,
                    prompt,
                    working_dir.to_path_buf(),
                    &mut sink,
                )
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

pub(crate) fn list_agents(sink: &mut ReplSink) {
    use crate::builtin_agents::BUILTIN_AGENT_IDS;
    sink.line(tr("repl-list-agents-title"));
    sink.line("");
    for id in BUILTIN_AGENT_IDS {
        let desc = match id {
            "general-purpose" => tr("repl-agent-desc-gp"),
            "explore" => tr("repl-agent-desc-explore"),
            "plan" => tr("repl-agent-desc-plan"),
            "workspace-assistant" => "Workspace-first assistant for channel mode.".to_string(),
            "goal" => "Goal loop agent with retries and progress tracking.".to_string(),
            _ => String::new(),
        };
        sink.line(format!("  • {}", id));
        if !desc.is_empty() {
            sink.line(desc);
        }
        sink.line("");
    }
    sink.line(tr("repl-list-switch"));
    sink.line("");
    sink.line(tr("repl-list-usage"));
    sink.line(tr("repl-list-usage-line"));
}

pub(crate) fn list_tools(sink: &mut ReplSink) {
    sink.line(tr("repl-list-tools-title"));
    sink.line("");
    for (name, desc) in iter_cli_tool_help() {
        sink.line(format!("  • {}", name));
        sink.line(format!("    {}", desc));
        sink.line("");
    }
    sink.line(tr("repl-security-title"));
    sink.line(tr("repl-security-read"));
    sink.line(tr("repl-security-approval"));
    sink.line(tr("repl-security-sandbox"));
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
