//! `anycode model` 交互（全量提供方目录 + anyCode 按任务路由）。

mod provider_flows;
mod routing;

use crate::app_config::prompts::prompt_line;
use crate::app_config::{load_anycode_config_resolved, resolve_config_path};
use crate::i18n::tr;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use provider_flows::{
    apply_quick_auth_choice, run_global_provider_flow, GlobalProviderFlowResult, QUICK_AUTH_CHOICES,
};
use routing::run_routing_agents_flow;
use std::path::PathBuf;

pub(super) fn accent_title(line: &str) {
    println!("{}", console::Style::new().cyan().bold().apply_to(line));
}

pub(super) async fn run(config_file: Option<PathBuf>) -> anyhow::Result<()> {
    let is_tty = console::Term::stdout().is_term();
    let path = resolve_config_path(config_file.clone())?;

    println!("{}", tr("model-banner"));
    println!("{} {}", tr("model-config-path"), path.display());
    println!();

    loop {
        accent_title(&tr("model-main-menu-title"));
        let hub_idx = if is_tty {
            let items = vec![
                tr("model-menu-global"),
                tr("model-menu-routing"),
                tr("model-menu-exit"),
            ];
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt(tr("model-pick-prompt"))
                .default(0)
                .items(&items)
                .interact()?
        } else {
            println!("{}", tr("model-menu-fallback-1"));
            println!("{}", tr("model-menu-fallback-2"));
            println!("{}", tr("model-menu-fallback-0"));
            loop {
                let v = prompt_line(&tr("model-pick-number"))?;
                match v.trim() {
                    "1" => break 0,
                    "2" => break 1,
                    "0" => return Ok(()),
                    _ => println!("{}", tr("model-invalid")),
                }
            }
        };

        match hub_idx {
            0 => {
                let _ = run_global_provider_flow(config_file.clone(), &path, is_tty, false, false)
                    .await?;
            }
            1 => run_routing_agents_flow(config_file.clone(), &path, is_tty).await?,
            _ => return Ok(()),
        }
        println!();
    }
}
/// setup 场景的精简模型向导：直接进入 provider 配置，不展示 routing 菜单。
pub(super) async fn run_onboard(config_file: Option<PathBuf>) -> anyhow::Result<()> {
    let is_tty = console::Term::stdout().is_term();
    let path = resolve_config_path(config_file.clone())?;

    loop {
        let existing = load_anycode_config_resolved(config_file.clone())?;

        println!("模型配置（OpenClaw 风格）");
        if let Some(ref c) = existing {
            println!("当前：provider={} model={}", c.provider, c.model);
        }

        let mode_items = vec![
            "快速配置（常用 auth-choice）".to_string(),
            "完整提供商目录（全量 provider）".to_string(),
        ];
        let mode_idx = if is_tty {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt("请选择配置模式")
                .default(0)
                .items(&mode_items)
                .interact()?
        } else {
            println!("请选择配置模式：");
            for (i, l) in mode_items.iter().enumerate() {
                println!("  {}) {}", i + 1, l);
            }
            loop {
                let v = prompt_line("输入序号")?;
                if let Ok(n) = v.trim().parse::<usize>() {
                    if n >= 1 && n <= mode_items.len() {
                        break n - 1;
                    }
                }
                println!("{}", tr("model-invalid"));
            }
        };

        if mode_idx == 1 {
            match run_global_provider_flow(config_file.clone(), &path, is_tty, true, true).await? {
                GlobalProviderFlowResult::Finished => return Ok(()),
                GlobalProviderFlowResult::SetupBackToModePicker => continue,
            }
        }

        let mut items: Vec<String> = QUICK_AUTH_CHOICES
            .iter()
            .map(|c| format!("{}  ({})", c.label, c.id))
            .collect();
        items.push("切换到完整提供商目录（全量）".to_string());

        let idx = if is_tty {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt("请选择 auth-choice")
                .default(0)
                .items(&items)
                .interact()?
        } else {
            println!("请选择 auth-choice：");
            for (i, l) in items.iter().enumerate() {
                println!("  {}) {}", i + 1, l);
            }
            loop {
                let v = prompt_line("输入序号")?;
                if let Ok(n) = v.trim().parse::<usize>() {
                    if n >= 1 && n <= items.len() {
                        break n - 1;
                    }
                }
                println!("{}", tr("model-invalid"));
            }
        };

        if idx >= QUICK_AUTH_CHOICES.len() {
            match run_global_provider_flow(config_file.clone(), &path, is_tty, true, true).await? {
                GlobalProviderFlowResult::Finished => return Ok(()),
                GlobalProviderFlowResult::SetupBackToModePicker => continue,
            }
        }
        apply_quick_auth_choice(
            config_file.clone(),
            &path,
            is_tty,
            &existing,
            QUICK_AUTH_CHOICES[idx],
        )
        .await?;
        return Ok(());
    }
}
