//! 交互式 REPL（stdio / 全屏 TUI 行编辑）与斜杠分发。

use std::io::{IsTerminal, Write};

use crate::app_config::{apply_optional_repl_model, Config};
use crate::bootstrap::initialize_runtime;
use crate::builtin_agents::parse_agent_slash_command;
use crate::i18n::{tr, tr_args};
use crate::repl_banner;
use crate::slash_commands::{self, ParsedSlashCommand};
use crate::workspace;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use anycode_tools::{iter_cli_tool_help, workflows};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::tasks_run::run_single_task_with_tail;
use super::tasks_sink::ReplSink;
use super::workflow_exec::{run_workflow_definition, run_workflow_path};

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
            ParsedSlashCommand::Compact => {
                sink.line(
                    "compact is available in TUI/session runtime; use session auto-compact or the compact flow.",
                );
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
