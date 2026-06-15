//! Clap command-line definitions (global flags and subcommands).

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

fn env_ignore_approval() -> bool {
    match std::env::var("ANYCODE_IGNORE_APPROVAL") {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

fn is_falsey_ignore_value(s: &str) -> bool {
    matches!(s.trim(), "false" | "0" | "no" | "off" | "n")
}

/// Scan argv outside Clap so approval stays off when global flags are not wired into `Args`.
pub(crate) fn argv_requests_ignore_approval() -> bool {
    for arg in std::env::args_os().skip(1) {
        let Some(a) = arg.to_str() else {
            continue;
        };
        match a {
            "--ignore-approval" | "--ingroe" | "-I" => return true,
            "--ignore" => return true,
            _ => {
                if let Some(rest) = a.strip_prefix("--ignore-approval=") {
                    if !is_falsey_ignore_value(rest) {
                        return true;
                    }
                } else if let Some(rest) = a.strip_prefix("--ignore=") {
                    if !is_falsey_ignore_value(rest) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// True if Clap `ignore_approval`, raw argv (`-I` / `--ignore`), or `ANYCODE_IGNORE_APPROVAL` is set.
pub(crate) fn session_ignore_approval(cli_ignore_approval: bool) -> bool {
    cli_ignore_approval || argv_requests_ignore_approval() || env_ignore_approval()
}

/// anyCode - Integrated AI Agent System
///
/// - **No subcommand (default)**: **交互式终端**（TTY 上流式视口 + 底栏）；**非 TTY** 为 **stdio 逐行**会话。
#[derive(Parser, Debug)]
#[command(name = "anycode")]
#[command(
    author = "anyCode Contributors",
    version = env!("CARGO_PKG_VERSION"),
    about = "anyCode - terminal AI agent for developers (Rust)",
    long_about = "anyCode is a Rust CLI for local tool-using agents.\n\
                 • Default (`anycode` with no subcommand): stream terminal UI on an interactive TTY; non-TTY uses line-at-a-time stdio.\n\
                 • Use global `--agent`, `-C` / `--directory`, `--model`, `--resume` to tune the default session."
)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    pub(crate) debug: bool,

    /// Path to config JSON
    #[arg(short, long, global = true)]
    pub(crate) config: Option<PathBuf>,

    /// Skip tool y/n approval for this process (temporary; same as security.require_approval=false for this run; deny rules still apply).
    /// Env `ANYCODE_IGNORE_APPROVAL=1|true|yes` is equivalent (for IDEs/wrappers that omit flags).
    #[arg(
        short = 'I',
        long = "ignore-approval",
        visible_alias = "ignore",
        alias = "ingroe",
        global = true
    )]
    pub(crate) ignore_approval: bool,

    /// Agent id (default: from config runtime mode).
    #[arg(short, long, global = true)]
    pub(crate) agent: Option<String>,

    /// Working directory for the default interactive session.
    #[arg(short = 'C', long = "directory", global = true)]
    pub(crate) directory: Option<PathBuf>,

    /// Override default model for this process only (long `--model` only).
    #[arg(long = "model", global = true)]
    pub(crate) model: Option<String>,

    /// Resume a saved session snapshot (`~/.anycode/sessions/<uuid>.json`).
    #[arg(long = "resume", global = true)]
    pub(crate) resume: Option<Uuid>,

    /// Log crossterm `Event` kind to stderr (paste payloads redacted); for diagnosing IME/PTY issues.
    #[arg(long = "repl-debug-events", global = true, hide = true)]
    pub(crate) repl_debug_events: bool,
}

mod audit;
mod auth;
mod channel;
mod cron;
mod dashboard;
mod doctor;
mod eval;
mod mcp;
mod memory;
mod model;
mod project;
mod skills;
mod statusline;
mod wechat;
mod workflow;
mod workspace;

pub(crate) use audit::*;
pub(crate) use auth::*;
pub(crate) use channel::*;
pub(crate) use cron::*;
pub(crate) use dashboard::*;
pub(crate) use doctor::*;
pub(crate) use eval::*;
pub(crate) use mcp::*;
pub(crate) use memory::*;
pub(crate) use model::*;
pub(crate) use project::*;
pub(crate) use skills::*;
pub(crate) use statusline::*;
pub(crate) use wechat::*;
pub(crate) use workflow::*;
pub(crate) use workspace::*;

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// ⏱️  Built-in scheduler: run persisted CronCreate jobs (`~/.anycode/tasks/orchestration.json`)
    Scheduler {
        /// Working directory for each triggered agent task
        #[arg(short = 'C', long)]
        directory: Option<PathBuf>,
        /// Re-read orchestration.json and cap sleep between ticks (seconds)
        #[arg(long, default_value_t = 30)]
        reload_secs: u64,
    },

    /// ▶️  Run a single task
    Run {
        /// Agent type
        #[arg(short, long)]
        agent: Option<String>,

        /// Runtime mode: general / explore / plan / code / channel / goal
        #[arg(long)]
        mode: Option<String>,

        /// Load and execute a workflow file (.yml/.yaml). If omitted, `workflow.yml` discovery may still be used by slash commands.
        #[arg(long)]
        workflow: Option<PathBuf>,

        /// Goal objective for retrying until done
        #[arg(long)]
        goal: Option<String>,

        /// Goal completion marker: retry until assistant output contains this substring
        #[arg(long)]
        done_when: Option<String>,

        /// Cap goal retry attempts (default: unlimited until done_when / objective is met)
        #[arg(long)]
        max_goal_attempts: Option<usize>,

        /// Stop after this many total input/output/cache tokens for the task.
        #[arg(long)]
        token_budget: Option<u32>,

        /// Reserved cost budget in USD for dashboard/runtime policy reporting.
        #[arg(long)]
        cost_budget_usd: Option<f64>,

        /// Stop before the next model turn after this many elapsed seconds.
        #[arg(long)]
        max_duration_secs: Option<u64>,

        /// Task description
        prompt: String,

        /// Working directory
        #[arg(short = 'C', long)]
        directory: Option<PathBuf>,
    },

    /// 🧩  Agent Skills (SKILL.md discovery under configured roots)
    Skills {
        #[command(subcommand)]
        sub: SkillsCommands,
    },

    /// ⚙️  Interactive configuration wizard
    Config,

    /// 🧭  Show or set the default runtime mode
    Mode {
        /// Runtime mode: general / explore / plan / code / channel / goal
        mode: Option<String>,
    },

    /// ✅  Enable a runtime feature
    Enable { feature: String },

    /// 🚫  Disable a runtime feature
    Disable { feature: String },

    /// 📊  Show current model / mode / feature status
    Status {
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Sample JSON for `statusLine.command` stdin (HUD)
    Statusline {
        #[command(subcommand)]
        sub: StatuslineCommands,
    },

    /// 🗂️  Manage workspace registry and defaults
    Workspace {
        #[command(subcommand)]
        sub: WorkspaceCommands,
    },

    /// 📦  Create projects from built-in templates (Flutter app pack, …)
    Project {
        #[command(subcommand)]
        sub: ProjectCommands,
    },

    /// 📋 Validate declarative workflow plans
    Workflow {
        #[command(subcommand)]
        sub: WorkflowCommands,
    },

    /// 🧠 Memory pipeline utilities (import legacy Markdown into hot store)
    Memory {
        #[command(subcommand)]
        sub: MemoryCommands,
    },

    /// 🧪  Production eval harness (mock scenarios; no real API key)
    Eval {
        #[command(subcommand)]
        sub: EvalCommands,
    },

    /// 🩺  Diagnose local anyCode runtime state
    Doctor {
        #[command(subcommand)]
        sub: DoctorCommands,
    },

    /// ⏱️  Inspect persisted cron jobs and run ledger
    Cron {
        #[command(subcommand)]
        sub: CronCommands,
    },

    /// 🔍  Query tool governance audit log
    Audit {
        #[command(subcommand)]
        sub: AuditCommands,
    },

    /// 🚀  First-time setup: model → memory / embeddings (TTY) → channel (wechat / telegram / discord) or skip
    Setup {
        /// Channel: wechat, telegram, discord, or skip/none to skip (optional)
        #[arg(long)]
        channel: Option<String>,
        /// WeChat data directory (default ~/.anycode/wechat; used when channel=wechat)
        #[arg(long, env = "WCC_DATA_DIR")]
        data_dir: Option<PathBuf>,
    },

    /// 📡 Channel bridge commands (wechat / telegram / discord)
    Channel {
        #[command(subcommand)]
        sub: ChannelCommands,
    },

    /// 💬  Local WeChat chat history (encrypted DB; not iLink bot)
    Wechat {
        #[command(subcommand)]
        sub: WechatCommands,
    },

    /// 🤖  Models and credentials (interactive when no nested subcommand; aligned with openclaw-style flow)
    Model {
        #[command(subcommand)]
        command: Option<ModelCommands>,
    },

    /// 🔒  Exercise security / approvals
    TestSecurity {
        /// Tool name
        #[arg(short, long)]
        tool: String,

        /// Tool input (JSON)
        #[arg(short, long)]
        input: String,
    },

    /// 🔌  MCP helpers (status always; OAuth login needs `--features mcp-oauth`)
    Mcp {
        #[command(subcommand)]
        sub: McpCommands,
    },

    /// ☁️  Cloud account login and device link
    Auth {
        #[command(subcommand)]
        sub: AuthCommands,
    },

    /// 📊  Local Digital Workbench (project dashboard API + Web UI)
    Dashboard {
        #[command(subcommand)]
        sub: Option<DashboardSubcommands>,
        #[command(flatten)]
        run: DashboardRunArgs,
    },
}

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct DashboardRunArgs {
    /// Bind host (default 127.0.0.1, or saved preferences)
    #[arg(long)]
    pub host: Option<String>,
    /// HTTP port (default 43180, or saved preferences)
    #[arg(long)]
    pub port: Option<u16>,
    /// SQLite database path (default ~/.anycode/projects.db, or saved preferences)
    #[arg(long)]
    pub db: Option<PathBuf>,
    /// Serve built UI static files from this directory
    #[arg(long)]
    pub static_dir: Option<PathBuf>,
    /// Open default browser to the dashboard URL
    #[arg(long, default_value_t = false)]
    pub open: bool,
}

/// Parse argv with locale-aware Clap help/about (see [`crate::i18n::localize_cli_command`]).
pub fn parse_args() -> Args {
    let mut cmd = Args::command();
    crate::i18n::localize_cli_command(&mut cmd);
    let m = cmd.get_matches();
    Args::from_arg_matches(&m).unwrap_or_else(|e| e.exit())
}

#[cfg(test)]
mod clap_tests {
    use super::{session_ignore_approval, Args, ChannelCommands, Commands, StatuslineCommands};
    use clap::Parser;
    use std::sync::{Mutex, OnceLock};
    use uuid::Uuid;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn session_ignore_false_without_env() {
        let _g = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        std::env::remove_var("ANYCODE_IGNORE_APPROVAL");
        assert!(!session_ignore_approval(false));
        assert!(session_ignore_approval(true));
    }

    #[test]
    fn session_ignore_true_from_env() {
        let _g = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        std::env::set_var("ANYCODE_IGNORE_APPROVAL", "1");
        assert!(session_ignore_approval(false));
        std::env::remove_var("ANYCODE_IGNORE_APPROVAL");
        assert!(!session_ignore_approval(false));
    }

    #[test]
    fn ignore_approval_no_subcommand() {
        let a = Args::try_parse_from(["anycode", "--ignore"]).unwrap();
        assert!(a.ignore_approval);
        assert!(a.command.is_none());
    }

    #[test]
    fn ignore_approval_short_capital_i() {
        let a = Args::try_parse_from(["anycode", "-I", "run", "--agent", "g", "p"]).unwrap();
        assert!(a.ignore_approval);
    }

    #[test]
    fn ignore_approval_global_before_subcommand() {
        let a = Args::try_parse_from([
            "anycode",
            "--ignore",
            "run",
            "--agent",
            "general-purpose",
            "hi",
        ])
        .unwrap();
        assert!(a.ignore_approval);
        assert!(matches!(a.command, Some(Commands::Run { .. })));
    }

    #[test]
    fn ignore_approval_global_after_run_options() {
        let a = Args::try_parse_from([
            "anycode",
            "run",
            "--agent",
            "general-purpose",
            "--ignore",
            "hi",
        ])
        .unwrap();
        assert!(a.ignore_approval);
    }

    /// If `--ignore` is placed after the positional PROMPT, Clap may not parse it — document that users should place global flags earlier.
    #[test]
    fn ignore_approval_after_positional_prompt_is_still_parsed() {
        let a = Args::try_parse_from([
            "anycode",
            "run",
            "--agent",
            "general-purpose",
            "task text",
            "--ignore",
        ])
        .unwrap();
        assert!(
            a.ignore_approval,
            "if this fails, --ignore after PROMPT is not supported; document placement"
        );
    }

    #[test]
    fn default_entry_parses_global_agent_and_directory() {
        let a = Args::try_parse_from([
            "anycode", "--agent", "explore", "-C", "/tmp", "--model", "glm-5",
        ])
        .unwrap();
        assert!(a.command.is_none());
        assert_eq!(a.agent.as_deref(), Some("explore"));
        assert_eq!(a.directory, Some(std::path::PathBuf::from("/tmp")));
        assert_eq!(a.model.as_deref(), Some("glm-5"));
    }

    #[test]
    fn model_without_subcommand_parses() {
        let a = Args::try_parse_from(["anycode", "model"]).unwrap();
        match a.command {
            Some(Commands::Model { command }) => assert!(command.is_none()),
            _ => unreachable!("clap test: expected model command"),
        }
    }

    #[test]
    fn model_status_hidden_subcommand_parses() {
        let a = Args::try_parse_from(["anycode", "model", "status"]).unwrap();
        match a.command {
            Some(Commands::Model { command }) => {
                assert!(matches!(
                    command,
                    Some(super::ModelCommands::Status { json: false })
                ));
            }
            _ => unreachable!("clap test: expected model status"),
        }
    }

    #[test]
    fn statusline_print_schema_parses() {
        let a = Args::try_parse_from(["anycode", "statusline", "print-schema"]).unwrap();
        match a.command {
            Some(Commands::Statusline { sub }) => {
                assert!(matches!(sub, StatuslineCommands::PrintSchema));
            }
            _ => panic!("expected statusline print-schema"),
        }
    }

    #[test]
    fn resume_uuid_parses() {
        let a = Args::try_parse_from([
            "anycode",
            "--resume",
            "d5e55f53-f0ef-42d9-a0fb-359005d5b8aa",
        ])
        .unwrap();
        assert_eq!(
            a.resume,
            Some(Uuid::parse_str("d5e55f53-f0ef-42d9-a0fb-359005d5b8aa").unwrap())
        );
    }

    #[test]
    fn setup_subcommand_parses() {
        let a = Args::try_parse_from(["anycode", "setup", "--channel", "telegram"]).unwrap();
        match a.command {
            Some(Commands::Setup { channel, data_dir }) => {
                assert_eq!(channel.as_deref(), Some("telegram"));
                assert!(data_dir.is_none());
            }
            _ => panic!("expected setup"),
        }
        let a = Args::try_parse_from(["anycode", "setup", "--channel", "skip"]).unwrap();
        match a.command {
            Some(Commands::Setup { channel, .. }) => {
                assert_eq!(channel.as_deref(), Some("skip"));
            }
            _ => panic!("expected setup"),
        }
    }

    #[test]
    fn onboard_command_removed() {
        let err = Args::try_parse_from(["anycode", "onboard", "--skip-wechat"]).unwrap_err();
        assert!(
            err.to_string().contains("unrecognized subcommand"),
            "expected onboard to be removed, got: {err}"
        );
    }

    #[test]
    fn removed_commands_are_rejected() {
        for sub in ["chat", "daemon", "list-agents", "list-tools", "repl", "tui"] {
            let err = Args::try_parse_from(["anycode", sub]).unwrap_err();
            assert!(
                err.to_string().contains("unrecognized subcommand"),
                "expected removed subcommand `{sub}` to be rejected, got: {err}"
            );
        }
    }

    #[test]
    fn scheduler_subcommand_parses() {
        let a = Args::try_parse_from([
            "anycode",
            "scheduler",
            "-C",
            "/tmp/wd",
            "--reload-secs",
            "60",
        ])
        .unwrap();
        match a.command {
            Some(Commands::Scheduler {
                directory,
                reload_secs,
            }) => {
                assert_eq!(directory, Some(std::path::PathBuf::from("/tmp/wd")));
                assert_eq!(reload_secs, 60);
            }
            _ => panic!("expected scheduler"),
        }
    }

    #[test]
    fn workspace_list_parses() {
        let a = Args::try_parse_from(["anycode", "workspace", "list", "--json"]).unwrap();
        match a.command {
            Some(Commands::Workspace { sub }) => {
                assert!(matches!(sub, super::WorkspaceCommands::List { json: true }));
            }
            _ => panic!("expected workspace list"),
        }
    }

    #[test]
    fn memory_import_parses() {
        let a = Args::try_parse_from(["anycode", "memory", "import", "--dry-run", "--limit", "10"])
            .unwrap();
        match a.command {
            Some(Commands::Memory { sub }) => {
                assert!(matches!(
                    sub,
                    super::MemoryCommands::Import {
                        dry_run: true,
                        limit: Some(10)
                    }
                ));
            }
            _ => panic!("expected memory import"),
        }
    }

    #[test]
    fn telegram_subcommand_parses() {
        let a = Args::try_parse_from([
            "anycode",
            "channel",
            "telegram",
            "--chat-id",
            "123",
            "--agent",
            "workspace-assistant",
        ])
        .unwrap();
        match a.command {
            Some(Commands::Channel { sub }) => match sub {
                ChannelCommands::Telegram { chat_id, agent, .. } => {
                    assert_eq!(chat_id.as_deref(), Some("123"));
                    assert_eq!(agent, "workspace-assistant");
                }
                _ => panic!("expected telegram subcommand"),
            },
            _ => panic!("expected channel command"),
        }
    }

    #[test]
    fn discord_subcommand_parses() {
        let a =
            Args::try_parse_from(["anycode", "channel", "discord", "--channel-id", "999"]).unwrap();
        match a.command {
            Some(Commands::Channel { sub }) => match sub {
                ChannelCommands::Discord { channel_id, .. } => {
                    assert_eq!(channel_id.as_deref(), Some("999"));
                }
                _ => panic!("expected discord subcommand"),
            },
            _ => panic!("expected channel command"),
        }
    }
}
