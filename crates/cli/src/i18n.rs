//! Fluent-based UI strings (cli / repl / wizards). Locale from [`anycode_locale::resolve_locale`].
//! `FluentBundle` is not `Sync`; we use `thread_local` (CLI is effectively single-threaded for UI).

use anycode_locale::AppLocale;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::borrow::Cow;
use std::cell::RefCell;

thread_local! {
    static BUNDLE: RefCell<Option<FluentBundle<FluentResource>>> = RefCell::new(None);
}

fn build_bundle() -> FluentBundle<FluentResource> {
    let loc = anycode_locale::resolve_locale();
    let lang: unic_langid::LanguageIdentifier = match loc {
        AppLocale::ZhHans => "zh".parse().expect("langid zh"),
        AppLocale::En => "en".parse().expect("langid en"),
    };
    let (cli, repl, wizard, tui, wx) = match loc {
        AppLocale::ZhHans => (
            include_str!("../locales/zh/cli.ftl"),
            include_str!("../locales/zh/repl.ftl"),
            include_str!("../locales/zh/wizard.ftl"),
            include_str!("../locales/zh/tui.ftl"),
            include_str!("../locales/zh/wx.ftl"),
        ),
        AppLocale::En => (
            include_str!("../locales/en/cli.ftl"),
            include_str!("../locales/en/repl.ftl"),
            include_str!("../locales/en/wizard.ftl"),
            include_str!("../locales/en/tui.ftl"),
            include_str!("../locales/en/wx.ftl"),
        ),
    };
    let mut bundle = FluentBundle::new(vec![lang]);
    for src in [cli, repl, wizard, tui, wx] {
        let res = FluentResource::try_new(src.to_string()).expect("FTL parse");
        bundle.add_resource(res).expect("FTL add_resource");
    }
    bundle
}

fn with_bundle<R>(f: impl FnOnce(&FluentBundle<FluentResource>) -> R) -> R {
    BUNDLE.with(|cell| {
        let mut g = cell.borrow_mut();
        if g.is_none() {
            *g = Some(build_bundle());
        }
        f(g.as_ref().expect("bundle"))
    })
}

fn format_msg(
    bundle: &FluentBundle<FluentResource>,
    id: &str,
    args: Option<&FluentArgs>,
) -> String {
    let mut errs = vec![];
    let Some(msg) = bundle.get_message(id) else {
        return format!("[missing:{id}]");
    };
    let Some(value) = msg.value() else {
        return format!("[no-value:{id}]");
    };
    let cow: Cow<'_, str> = bundle.format_pattern(value, args, &mut errs);
    cow.into_owned()
}

pub fn tr(id: &str) -> String {
    with_bundle(|b| format_msg(b, id, None))
}

#[allow(dead_code)]
pub fn tr_args(id: &str, args: &FluentArgs<'_>) -> String {
    with_bundle(|b| format_msg(b, id, Some(args)))
}

fn localize_run(s: &mut clap::Command) {
    let n = s
        .clone()
        .about(tr("cmd-run-about"))
        .mut_arg("agent", |a| a.help(tr("cmd-run-agent")))
        .mut_arg("prompt", |a| a.help(tr("cmd-run-prompt")))
        .mut_arg("directory", |a| a.help(tr("cmd-run-directory")));
    *s = n;
}

fn localize_repl(s: &mut clap::Command) {
    let n = s
        .clone()
        .about(tr("cmd-repl-about"))
        .mut_arg("agent", |a| a.help(tr("cmd-repl-agent")))
        .mut_arg("directory", |a| a.help(tr("cmd-repl-directory")))
        .mut_arg("model", |a| a.help(tr("cmd-repl-model")));
    *s = n;
}

fn localize_model(m: &mut clap::Command) {
    let mut root = m.clone().about(tr("cmd-model-about"));
    for sub in root.get_subcommands_mut() {
        match sub.get_name() {
            "list" => {
                let n = sub
                    .clone()
                    .about(tr("model-list-about"))
                    .mut_arg("json", |a| a.help(tr("model-list-json")))
                    .mut_arg("plain", |a| a.help(tr("model-list-plain")));
                *sub = n;
            }
            "status" => {
                let n = sub
                    .clone()
                    .about(tr("model-status-about"))
                    .mut_arg("json", |a| a.help(tr("model-status-json")));
                *sub = n;
            }
            "set" => {
                let n = sub
                    .clone()
                    .about(tr("model-set-about"))
                    .mut_arg("model", |a| a.help(tr("model-set-model")));
                *sub = n;
            }
            _ => {}
        }
    }
    *m = root;
}

#[cfg(feature = "mcp-oauth")]
fn localize_mcp(m: &mut clap::Command) {
    let mut root = m.clone().about(tr("cmd-mcp-about"));
    for sub in root.get_subcommands_mut() {
        if sub.get_name() != "oauth-login" {
            continue;
        }
        let n = sub
            .clone()
            .about(tr("mcp-oauth-about"))
            .mut_arg("url", |a| a.help(tr("mcp-oauth-url")))
            .mut_arg("host", |a| a.help(tr("mcp-oauth-host")))
            .mut_arg("port", |a| a.help(tr("mcp-oauth-port")))
            .mut_arg("callback_path", |a| a.help(tr("mcp-oauth-callback")))
            .mut_arg("client_metadata_url", |a| {
                a.help(tr("mcp-oauth-client-metadata"))
            })
            .mut_arg("scopes", |a| a.help(tr("mcp-oauth-scope")))
            .mut_arg("no_browser", |a| a.help(tr("mcp-oauth-no-browser")))
            .mut_arg("write_token", |a| a.help(tr("mcp-oauth-write-token")))
            .mut_arg("credentials_store", |a| {
                a.help(tr("mcp-oauth-credentials-store"))
            });
        *sub = n;
    }
    *m = root;
}

/// Apply localized `about` / `help` to the Clap command tree (must run before `get_matches`).
pub fn localize_cli_command(cmd: &mut clap::Command) {
    let root = cmd
        .clone()
        .about(tr("cli-short"))
        .long_about(tr("cli-long"))
        .mut_arg("debug", |a| a.help(tr("flag-debug")))
        .mut_arg("config", |a| a.help(tr("flag-config")))
        .mut_arg("ignore_approval", |a| a.help(tr("flag-ignore-approval")))
        .mut_arg("model", |a| a.help(tr("flag-model")));
    *cmd = root;

    for sub in cmd.get_subcommands_mut() {
        match sub.get_name() {
            "scheduler" => {
                let n = sub
                    .clone()
                    .about(tr("cmd-scheduler-about"))
                    .mut_arg("directory", |a| a.help(tr("cmd-scheduler-directory")))
                    .mut_arg("reload_secs", |a| a.help(tr("cmd-scheduler-reload-secs")));
                *sub = n;
            }
            "run" => localize_run(sub),
            "repl" => localize_repl(sub),
            "config" => *sub = sub.clone().about(tr("cmd-config-about")),
            "setup" => {
                let n = sub
                    .clone()
                    .about(tr("cmd-setup-about"))
                    .mut_arg("channel", |a| a.help(tr("cmd-setup-channel")))
                    .mut_arg("data_dir", |a| a.help(tr("cmd-setup-data-dir")));
                *sub = n;
            }
            "channel" => {
                let mut root = sub.clone().about(tr("cmd-channel-about"));
                for nested in root.get_subcommands_mut() {
                    match nested.get_name() {
                        "wechat" => {
                            let n = nested
                                .clone()
                                .about(tr("cmd-wechat-about"))
                                .mut_arg("data_dir", |a| a.help(tr("cmd-wechat-data-dir")))
                                .mut_arg("run_as_bridge", |a| a.help(tr("cmd-wechat-bridge-hint")))
                                .mut_arg("agent", |a| a.help(tr("cmd-wechat-agent")));
                            *nested = n;
                        }
                        "telegram" => {
                            let n = nested
                                .clone()
                                .about(tr("cmd-telegram-about"))
                                .mut_arg("bot_token", |a| a.help(tr("cmd-telegram-bot-token")))
                                .mut_arg("chat_id", |a| a.help(tr("cmd-telegram-chat-id")))
                                .mut_arg("agent", |a| a.help(tr("cmd-telegram-agent")))
                                .mut_arg("directory", |a| a.help(tr("cmd-telegram-directory")));
                            *nested = n;
                        }
                        "discord" => {
                            let n = nested
                                .clone()
                                .about(tr("cmd-discord-about"))
                                .mut_arg("bot_token", |a| a.help(tr("cmd-discord-bot-token")))
                                .mut_arg("channel_id", |a| a.help(tr("cmd-discord-channel-id")))
                                .mut_arg("agent", |a| a.help(tr("cmd-discord-agent")))
                                .mut_arg("directory", |a| a.help(tr("cmd-discord-directory")));
                            *nested = n;
                        }
                        _ => {}
                    }
                }
                *sub = root;
            }
            "model" => localize_model(sub),
            "test-security" => {
                let n = sub
                    .clone()
                    .about(tr("cmd-test-security-about"))
                    .mut_arg("tool", |a| a.help(tr("cmd-test-security-tool")))
                    .mut_arg("input", |a| a.help(tr("cmd-test-security-input")));
                *sub = n;
            }
            #[cfg(feature = "mcp-oauth")]
            "mcp" => localize_mcp(sub),
            _ => {}
        }
    }
}
