//! Fluent-based UI strings (cli / repl / wizards). Locale from [`anycode_locale::resolve_locale`].
//! `FluentBundle` is not `Sync`; we use `thread_local` (CLI is effectively single-threaded for UI).

use anycode_locale::AppLocale;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

thread_local! {
    static BUNDLE: RefCell<Option<FluentBundle<FluentResource>>> = const { RefCell::new(None) };
    /// 翻译结果缓存（thread_local，避免全局锁）
    static TRANSLATION_CACHE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// 全局翻译缓存统计（用于性能监控）
static TRANSLATION_STATS: Lazy<Mutex<TranslationStats>> =
    Lazy::new(|| Mutex::new(TranslationStats::new()));

#[derive(Default)]
struct TranslationStats {
    cache_hits: u64,
    cache_misses: u64,
    total_translations: u64,
}

impl TranslationStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_hit(&mut self) {
        self.cache_hits += 1;
        self.total_translations += 1;
    }

    fn record_miss(&mut self) {
        self.cache_misses += 1;
        self.total_translations += 1;
    }

    fn hit_rate(&self) -> f64 {
        if self.total_translations == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_translations as f64
        }
    }
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
    // 尝试从缓存获取
    let cache_key = id.to_string();

    TRANSLATION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(cached) = cache.get(&cache_key) {
            // 缓存命中
            if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
                stats.record_hit();
            }
            return cached.clone();
        }

        // 缓存未命中，执行翻译
        let result = with_bundle(|b| format_msg(b, id, None));

        // 只缓存常用的翻译（避免内存膨胀）
        if is_common_translation_id(id) {
            cache.insert(cache_key, result.clone());
        }

        if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
            stats.record_miss();
        }

        result
    })
}

#[allow(dead_code)]
pub fn tr_args(id: &str, args: &FluentArgs<'_>) -> String {
    // 对于带参数的翻译，生成不同的缓存键
    let args_str = args
        .iter()
        .map(|(k, v)| format!("{}={:?}", k, v))
        .collect::<Vec<_>>()
        .join(",");
    let cache_key = format!("{}|{}", id, args_str);

    TRANSLATION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(cached) = cache.get(&cache_key) {
            if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
                stats.record_hit();
            }
            return cached.clone();
        }

        let result = with_bundle(|b| format_msg(b, id, Some(args)));

        // 带参数的翻译也缓存
        if is_common_translation_id(id) {
            cache.insert(cache_key, result.clone());
        }

        if let Ok(mut stats) = TRANSLATION_STATS.try_lock() {
            stats.record_miss();
        }

        result
    })
}

/// 判断是否是常用的翻译 ID（用于缓存策略）
fn is_common_translation_id(id: &str) -> bool {
    // 常见的 UI 文本，值得缓存
    const COMMON_PREFIXES: &[&str] = &[
        "tui-", "cli-", "repl-", "status-", "error-", "warn-", "confirm-", "cancel-",
    ];

    // 长度适中的翻译 ID 更可能是常用翻译
    if id.len() > 30 || id.len() < 5 {
        return false;
    }

    // 检查是否是常用前缀
    COMMON_PREFIXES.iter().any(|&prefix| id.starts_with(prefix))
}

/// 清空翻译缓存（用于切换语言或释放内存）
#[allow(dead_code)]
pub fn clear_translation_cache() {
    TRANSLATION_CACHE.with(|cache| {
        cache.borrow_mut().clear();
    });
}

/// 获取翻译缓存统计信息（用于性能监控）
#[allow(dead_code)]
pub fn get_translation_stats() -> (u64, u64, f64) {
    if let Ok(stats) = TRANSLATION_STATS.try_lock() {
        (stats.cache_hits, stats.cache_misses, stats.hit_rate())
    } else {
        (0, 0, 0.0)
    }
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
