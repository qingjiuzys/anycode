//! First-run `anycode setup` onboarding flow.

use super::*;
use crate::i18n::tr;
use std::path::PathBuf;

fn has_non_empty_secret(v: &str) -> bool {
    !v.trim().is_empty()
}

fn has_usable_model_config(cfg: &AnyCodeConfig) -> bool {
    if cfg.provider.trim().is_empty() || cfg.model.trim().is_empty() {
        return false;
    }
    if validate_llm_provider(&cfg.provider).is_err() {
        return false;
    }
    if has_non_empty_secret(&cfg.api_key) {
        return true;
    }
    cfg.provider_credentials
        .values()
        .any(|v| has_non_empty_secret(v))
}

/// 首次安装聚合：模型向导 →（TTY）记忆/向量向导 → channel（wechat/telegram/discord）。
pub(crate) async fn run_onboard_flow(
    config_file: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    channel: Option<String>,
    debug: bool,
) -> anyhow::Result<()> {
    crate::workspace::ensure_layout()?;
    {
        let term = console::Term::stdout();
        if term.is_term() {
            // Keep it minimal so it doesn't dominate the setup UX.
            // 单段输出：`println!` 会在整段末尾再补一个 `\n`，格式里不要在最后一行前多加 `\n`，否则会出现「中间空一行」。
            println!(
                "\n    _              ____          __\n   / \\   _ __  _  / ___|___   __/ _| ___\n  / _ \\ | '_ \\| | | |   / _ \\ / _` |/ _ \\\n / ___ \\| | | | |_| |__| (_) | (_| |  __/\n/_/   \\_\\_| |_|\\__, |\\____\\___/ \\__,_|\\___|"
            );
        }
    }
    let existing = load_anycode_config_resolved(config_file.clone())?;
    let already_configured = existing.as_ref().is_some_and(has_usable_model_config);
    let mut reconfigure_model = !already_configured;
    if already_configured {
        println!("检测到已存在可用模型配置，将默认跳过模型配置。");
        println!("如需单独重配，可运行：anycode model");
        let term = console::Term::stdout();
        if term.is_term() {
            use dialoguer::{theme::ColorfulTheme, Select};
            let options = [
                "跳过（使用现有配置，推荐）",
                "现在重配模型（进入 anycode model 简化向导）",
                "退出 setup",
            ];
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("模型配置")
                .items(options)
                .default(0)
                .interact()?;
            match idx {
                0 => reconfigure_model = false,
                1 => reconfigure_model = true,
                _ => anyhow::bail!("setup cancelled"),
            }
        } else {
            reconfigure_model = false;
        }
    }
    if reconfigure_model {
        run_model_onboard_interactive(config_file.clone()).await?;
    }

    let cf_mem = config_file.clone();
    match tokio::task::spawn_blocking(move || crate::setup_memory::run_after_model_step(cf_mem))
        .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::warn!(target: "anycode_cli", "memory setup step skipped: {}", e);
        }
        Err(e) => {
            tracing::warn!(target: "anycode_cli", "memory setup task join failed: {}", e);
        }
    }

    let ch_arg = channel
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());
    let selected: Option<&'static str> = match ch_arg.as_deref() {
        Some("wechat") => Some("wechat"),
        Some("telegram") => Some("telegram"),
        Some("discord") => Some("discord"),
        Some("skip" | "none") => None,
        Some(other) => {
            anyhow::bail!("unsupported setup channel: {other} (expected wechat/telegram/discord or skip/none)")
        }
        None => {
            use dialoguer::{theme::ColorfulTheme, Select};
            let term = console::Term::stdout();
            if !term.is_term() {
                println!("{}", tr("setup-non-tty-skip-hint"));
                None
            } else {
                let opt_skip = tr("setup-channel-opt-skip");
                let options = ["wechat", "telegram", "discord", opt_skip.as_str()];
                let prompt = tr("setup-channel-prompt");
                let idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt(&prompt)
                    .items(options)
                    .default(0)
                    .interact()?;
                match idx {
                    0 => Some("wechat"),
                    1 => Some("telegram"),
                    2 => Some("discord"),
                    _ => None,
                }
            }
        }
    };

    let Some(ch) = selected else {
        println!("{}", tr("setup-channel-skipped-hint"));
        return Ok(());
    };

    match ch {
        "wechat" => crate::channels::wechat::run_onboard(data_dir, config_file, debug).await,
        "telegram" => crate::channels::tg::run_telegram_setup().await,
        "discord" => crate::channels::discord_channel::run_discord_setup().await,
        _ => unreachable!(),
    }
}
