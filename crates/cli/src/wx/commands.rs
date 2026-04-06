//! 斜杠命令（Rust 子集，与 Node 行为接近）。

use crate::i18n::{tr, tr_args};
use crate::wx::store::{chat_history_text, save_session, SessionState, WcSession, WccConfig};
use anyhow::Result;
use fluent_bundle::FluentArgs;
use std::path::Path;

pub struct CmdCtx<'a> {
    pub data_root: &'a Path,
    pub account_id: &'a str,
    pub session: &'a mut WcSession,
    pub wcc: &'a mut WccConfig,
}

pub enum CmdOut {
    Reply(String),
    Nothing,
}

pub fn route_command(text: &str, ctx: &mut CmdCtx<'_>) -> Result<CmdOut> {
    let t = text.trim();
    if !t.starts_with('/') {
        return Ok(CmdOut::Nothing);
    }
    let t = t.strip_prefix('/').unwrap_or(t);
    let (cmd, args) = match t.find(' ') {
        Some(i) => (t[..i].to_lowercase(), t[i + 1..].trim()),
        None => (t.to_lowercase(), ""),
    };

    match cmd.as_str() {
        "help" | "h" => Ok(CmdOut::Reply(tr("wx-help"))),
        "version" | "v" => {
            let mut a = FluentArgs::new();
            a.set("version", env!("CARGO_PKG_VERSION"));
            Ok(CmdOut::Reply(tr_args("wx-version", &a)))
        }
        "clear" => {
            *ctx.session = WcSession {
                working_directory: ctx.session.working_directory.clone(),
                model: ctx.session.model.clone(),
                permission_mode: ctx.session.permission_mode.clone(),
                runtime_mode: ctx.session.runtime_mode.clone(),
                max_history_length: ctx.session.max_history_length,
                ..Default::default()
            };
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            Ok(CmdOut::Reply(tr("wx-cmd-clear-ok")))
        }
        "reset" => {
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".into());
            *ctx.session = WcSession {
                working_directory: cwd,
                ..Default::default()
            };
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            Ok(CmdOut::Reply(tr("wx-cmd-reset-ok")))
        }
        "status" => {
            let mode = ctx.session.permission_mode.as_deref().unwrap_or("default");
            let runtime_mode = ctx.session.runtime_mode.as_deref().unwrap_or("channel");
            let st = match ctx.session.state {
                SessionState::Idle => "idle",
                SessionState::Processing => "processing",
                SessionState::WaitingPermission => "waiting_permission",
            };
            let sid = ctx
                .session
                .sdk_session_id
                .as_deref()
                .unwrap_or("(none)");
            let mut a = FluentArgs::new();
            a.set("cwd", ctx.session.working_directory.clone());
            a.set("model", format!("{:?}", ctx.session.model));
            a.set("mode", mode);
            a.set("rt", runtime_mode);
            a.set("st", st);
            a.set("sid", sid);
            Ok(CmdOut::Reply(format!(
                "{}\nruntime_mode: {}",
                tr_args("wx-cmd-status", &a),
                runtime_mode
            )))
        }
        "cwd" => {
            if args.is_empty() {
                let mut a = FluentArgs::new();
                a.set("cwd", ctx.session.working_directory.clone());
                return Ok(CmdOut::Reply(tr_args("wx-cmd-cwd-current", &a)));
            }
            ctx.session.working_directory = args.to_string();
            ctx.wcc.working_directory = args.to_string();
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            write_config_env(ctx.data_root, ctx.wcc)?;
            let mut a = FluentArgs::new();
            a.set("path", args);
            Ok(CmdOut::Reply(tr_args("wx-cmd-cwd-ok", &a)))
        }
        "model" => {
            if args.is_empty() {
                return Ok(CmdOut::Reply(tr("wx-cmd-model-usage")));
            }
            ctx.session.model = Some(args.to_string());
            ctx.wcc.model = Some(args.to_string());
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            write_config_env(ctx.data_root, ctx.wcc)?;
            let mut a = FluentArgs::new();
            a.set("name", args);
            Ok(CmdOut::Reply(tr_args("wx-cmd-model-ok", &a)))
        }
        "permission" => {
            const MODES: &[&str] = &["default", "acceptEdits", "plan", "auto"];
            if args.is_empty() {
                let cur = ctx.session.permission_mode.as_deref().unwrap_or("default");
                let mut a = FluentArgs::new();
                a.set("cur", cur);
                a.set("modes", MODES.join(", "));
                return Ok(CmdOut::Reply(tr_args("wx-cmd-perm-current", &a)));
            }
            if !MODES.contains(&args) {
                let mut a = FluentArgs::new();
                a.set("modes", MODES.join(", "));
                return Ok(CmdOut::Reply(tr_args("wx-cmd-perm-unknown", &a)));
            }
            ctx.session.permission_mode = Some(args.to_string());
            ctx.wcc.permission_mode = Some(args.to_string());
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            write_config_env(ctx.data_root, ctx.wcc)?;
            let mut a = FluentArgs::new();
            a.set("mode", args);
            Ok(CmdOut::Reply(tr_args("wx-cmd-perm-ok", &a)))
        }
        "mode" => {
            const MODES: &[&str] = &["channel", "code", "plan", "explore", "goal"];
            if args.is_empty() {
                let cur = ctx.session.runtime_mode.as_deref().unwrap_or("channel");
                return Ok(CmdOut::Reply(format!(
                    "current runtime mode: {}\navailable: {}",
                    cur,
                    MODES.join(", ")
                )));
            }
            if !MODES.contains(&args) {
                return Ok(CmdOut::Reply(format!(
                    "unknown runtime mode: {}\navailable: {}",
                    args,
                    MODES.join(", ")
                )));
            }
            ctx.session.runtime_mode = Some(args.to_string());
            ctx.wcc.runtime_mode = Some(args.to_string());
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            write_config_env(ctx.data_root, ctx.wcc)?;
            Ok(CmdOut::Reply(format!("runtime mode set to {}", args)))
        }
        "history" => {
            let n: usize = if args.is_empty() {
                20
            } else {
                args.parse().unwrap_or(20)
            };
            let n = n.min(100);
            let text = chat_history_text(ctx.session, Some(n));
            let mut a = FluentArgs::new();
            a.set("n", n as i64);
            a.set("text", text);
            Ok(CmdOut::Reply(tr_args("wx-cmd-history", &a)))
        }
        "undo" => {
            let n: usize = if args.is_empty() {
                1
            } else {
                args.parse().unwrap_or(1)
            };
            let n = n.max(1);
            let len = ctx.session.chat_history.len();
            if len == 0 {
                return Ok(CmdOut::Reply(tr("wx-cmd-undo-empty")));
            }
            let take = n.min(len);
            ctx.session.chat_history.truncate(len - take);
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            let mut a = FluentArgs::new();
            a.set("n", take as i64);
            Ok(CmdOut::Reply(tr_args("wx-cmd-undo-ok", &a)))
        }
        "compact" => {
            if ctx.session.sdk_session_id.is_none() {
                return Ok(CmdOut::Reply(tr("wx-cmd-compact-none")));
            }
            ctx.session.previous_sdk_session_id = ctx.session.sdk_session_id.take();
            save_session(ctx.data_root, ctx.account_id, ctx.session)?;
            Ok(CmdOut::Reply(tr("wx-cmd-compact-ok")))
        }
        "prompt" => {
            if args.is_empty() {
                let cur = ctx
                    .wcc
                    .system_prompt
                    .clone()
                    .unwrap_or_else(|| tr("wx-prompt-none"));
                let mut a = FluentArgs::new();
                a.set("cur", cur);
                return Ok(CmdOut::Reply(tr_args("wx-cmd-prompt-current", &a)));
            }
            if args.eq_ignore_ascii_case("clear") {
                ctx.wcc.system_prompt = None;
            } else {
                ctx.wcc.system_prompt = Some(args.to_string());
            }
            write_config_env(ctx.data_root, ctx.wcc)?;
            Ok(CmdOut::Reply(tr("wx-cmd-prompt-updated")))
        }
        _ => Ok(CmdOut::Nothing),
    }
}

fn write_config_env(data_root: &Path, wcc: &WccConfig) -> Result<()> {
    let mut s = format!("workingDirectory={}\n", wcc.working_directory);
    if let Some(m) = &wcc.model {
        s.push_str(&format!("model={}\n", m));
    }
    if let Some(p) = &wcc.permission_mode {
        s.push_str(&format!("permissionMode={}\n", p));
    }
    if let Some(m) = &wcc.runtime_mode {
        s.push_str(&format!("runtimeMode={}\n", m));
    }
    if let Some(p) = &wcc.system_prompt {
        s.push_str(&format!("systemPrompt={}\n", p));
    }
    std::fs::create_dir_all(data_root)?;
    std::fs::write(data_root.join("config.env"), s)?;
    Ok(())
}
