//! Clap command-line definitions (global flags and subcommands).

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;

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

/// 🦀 anyCode - Integrated AI Agent System
///
/// - **No subcommand**: fullscreen TUI (ratatui).
/// - **`repl`**: line REPL; output in the main terminal buffer (no fullscreen UI).
#[derive(Parser, Debug)]
#[command(name = "anycode")]
#[command(
    author = "anyCode Contributors",
    version = "0.1.0",
    about = "anyCode - terminal AI agent for developers (Rust)",
    long_about = "anyCode is a Rust CLI for local tool-using agents: TUI, REPL, and automation.\n\
                 • Default (no subcommand): fullscreen TUI.\n\
                 • `repl`: line-based REPL with native terminal scrollback."
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

    /// Override default model for this process only (TUI with no subcommand; same validation as `repl --model`; long `--model` only).
    #[arg(long = "model", global = true)]
    pub(crate) model: Option<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// 💬  Unified chat entry (line REPL for now)
    Chat {
        #[arg(short, long, default_value = "general-purpose")]
        agent: String,
        #[arg(short = 'C', long)]
        directory: Option<PathBuf>,
        #[arg(short, long)]
        model: Option<String>,
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

        /// Task description
        prompt: String,

        /// Working directory
        #[arg(short = 'C', long)]
        directory: Option<PathBuf>,
    },

    /// 📜  Line REPL: one line per task, **no fullscreen TUI**; scrollback in the main buffer
    Repl {
        #[arg(short, long, default_value = "general-purpose")]
        agent: String,
        #[arg(short = 'C', long)]
        directory: Option<PathBuf>,
        #[arg(short, long)]
        model: Option<String>,
    },

    /// 📋  List available agents
    ListAgents,

    /// 🔧  List available tools
    ListTools,

    /// 🧩  Agent Skills (SKILL.md discovery under configured roots)
    Skills {
        #[command(subcommand)]
        sub: SkillsCommands,
    },

    /// 🌐  HTTP daemon (long-running)
    Daemon {
        /// Bind address
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: String,
    },

    /// ⚙️  Interactive configuration wizard
    Config,

    /// 🧭  Show or set the default runtime mode
    Mode {
        /// Runtime mode: general / explore / plan / code / channel / goal
        mode: Option<String>,
    },

    /// ✅  Enable a runtime feature
    Enable {
        feature: String,
    },

    /// 🚫  Disable a runtime feature
    Disable {
        feature: String,
    },

    /// 📊  Show current model / mode / feature status
    Status {
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// 🗂️  Manage workspace registry and defaults
    Workspace {
        #[command(subcommand)]
        sub: WorkspaceCommands,
    },

    /// 🚀  First-time setup: workspace → API wizard if needed → optional WeChat scan and login autostart (`--skip-wechat` to skip WeChat)
    Onboard {
        /// Skip WeChat binding (wizard + workspace only)
        #[arg(long, default_value_t = false)]
        skip_wechat: bool,
        /// WeChat data directory (default ~/.anycode/wechat; same as `wechat` subcommand)
        #[arg(long, env = "WCC_DATA_DIR")]
        data_dir: Option<PathBuf>,
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

    /// 🔌  MCP helpers (e.g. OAuth login; build with `--features mcp-oauth`)
    #[cfg(feature = "mcp-oauth")]
    Mcp {
        #[command(subcommand)]
        sub: McpCommands,
    },

    /// 💬  WeChat: scan to bind; installs login autostart bridge on success
    Wechat {
        /// Data directory (default ~/.anycode/wechat; `WCC_DATA_DIR` for legacy wechat-claude-code paths)
        #[arg(long, env = "WCC_DATA_DIR")]
        data_dir: Option<PathBuf>,
        /// Invoked by LaunchAgent/systemd to run the message bridge (do not use manually)
        #[arg(long, hide = true)]
        run_as_bridge: bool,
        /// Same as `anycode run --agent` (only with `--run-as-bridge`)
        #[arg(long, default_value = "workspace-assistant", hide = true)]
        agent: String,
    },
}

#[cfg(feature = "mcp-oauth")]
#[derive(Subcommand, Debug)]
pub(crate) enum McpCommands {
    /// Browser OAuth for remote MCP; prints access token (for ANYCODE_MCP_SERVERS `bearer_token`)
    OauthLogin {
        /// MCP endpoint URL, e.g. https://example.com/mcp
        #[arg(long)]
        url: String,
        /// Local callback bind address (127.0.0.1 recommended)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Local callback port
        #[arg(long, default_value_t = 9876)]
        port: u16,
        /// Callback path (must match redirect_uri; default /callback)
        #[arg(long, default_value = "/callback")]
        callback_path: String,
        /// OAuth client metadata URL (SEP-991; optional for most servers)
        #[arg(long)]
        client_metadata_url: Option<String>,
        /// Requested OAuth scopes (repeat flag)
        #[arg(long = "scope", action = clap::ArgAction::Append)]
        scopes: Vec<String>,
        /// Do not open browser; only print the authorization URL
        #[arg(long, default_value_t = false)]
        no_browser: bool,
        /// Write access_token as one line to this file (mind file permissions)
        #[arg(long)]
        write_token: Option<PathBuf>,
        /// Write full OAuth JSON (refresh_token) for ANYCODE_MCP_SERVERS `oauth_credentials_path` auto-refresh
        #[arg(long)]
        credentials_store: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum SkillsCommands {
    /// List discovered skills (id, description, has `run`, root path)
    List,
    /// Print effective skill search roots (extra_dirs then ~/.anycode/skills)
    Path,
    /// Create ~/.anycode/skills/<name>/ with minimal SKILL.md and `run` template
    Init {
        /// Skill id (directory name): letters, digits, `.`, `_`, `-` only
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum WorkspaceCommands {
    /// List recent workspaces
    List {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Show current workspace status
    Status,
    /// Register a workspace path or refresh last_seen
    Touch {
        path: Option<PathBuf>,
    },
    /// Set default mode for a workspace
    SetMode {
        mode: String,
        path: Option<PathBuf>,
    },
    /// Set default channel profile for a workspace
    SetChannel {
        channel: String,
        path: Option<PathBuf>,
    },
    /// Set a human-readable label for a workspace
    Label {
        label: String,
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ModelCommands {
    /// List models (z.ai static catalog for now)
    #[command(hide = true)]
    List {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Plain output (one model id per line)
        #[arg(long, default_value_t = false)]
        plain: bool,
    },

    /// Show configured model status
    #[command(hide = true)]
    Status {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Set default model
    #[command(hide = true)]
    Set {
        /// Model id (e.g. glm-5)
        model: String,
    },

    /// OAuth / token helpers for model providers
    Auth {
        #[command(subcommand)]
        sub: ModelAuthCommands,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ModelAuthCommands {
    /// GitHub device flow; writes ~/.anycode/credentials/github-oauth.json for GitHub Copilot
    Copilot,
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
    use super::{session_ignore_approval, Args, Commands};
    use clap::Parser;
    use std::sync::{Mutex, OnceLock};

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
    fn ignore_approval_default_tui_no_subcommand() {
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
    fn repl_subcommand_parses() {
        let a = Args::try_parse_from([
            "anycode",
            "repl",
            "--agent",
            "explore",
            "-C",
            "/tmp",
        ])
        .unwrap();
        match a.command {
            Some(Commands::Repl {
                agent,
                directory,
                model,
            }) => {
                assert_eq!(agent, "explore");
                assert_eq!(directory, Some(std::path::PathBuf::from("/tmp")));
                assert!(model.is_none());
            }
            _ => panic!("expected repl"),
        }
    }

    #[test]
    fn repl_subcommand_parses_model_flag() {
        let a = Args::try_parse_from([
            "anycode",
            "repl",
            "--model",
            "glm-5",
            "--agent",
            "general-purpose",
        ])
        .unwrap();
        match a.command {
            Some(Commands::Repl { model, .. }) => assert_eq!(model.as_deref(), Some("glm-5")),
            _ => panic!("expected repl"),
        }
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
    fn onboard_subcommand_parses() {
        let a = Args::try_parse_from(["anycode", "onboard", "--skip-wechat"]).unwrap();
        match a.command {
            Some(Commands::Onboard {
                skip_wechat,
                data_dir,
            }) => {
                assert!(skip_wechat);
                assert!(data_dir.is_none());
            }
            _ => panic!("expected onboard"),
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
}
