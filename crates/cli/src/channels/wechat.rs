//! `anycode channel wechat`：扫码绑定并安装后台服务；`--run-as-bridge` 仅供系统服务调用。

use anyhow::Context;
use serde::Serialize;

/// 扫码 → 写入账号 → 安装 LaunchAgent/systemd 并拉起桥接进程。
pub async fn run_onboard(
    data_dir: Option<std::path::PathBuf>,
    config: Option<std::path::PathBuf>,
    debug: bool,
) -> anyhow::Result<()> {
    let dir = data_dir.clone();
    super::wechat_ilink::run_interactive_setup(data_dir).await?;
    let root = super::wx::wcc_data_dir(dir);
    super::wechat_service::install_autostart_after_setup(root, config, debug)?;
    println!("\n✅ 微信桥已注册为登录自启动服务，并已尝试在后台运行。");
    println!("   之后重启或重新登录后会自动拉起；日志见 ~/.anycode/logs/wechat-bridge.*.log");
    Ok(())
}

pub async fn run_bridged_start(
    config: Option<std::path::PathBuf>,
    agent: String,
    data_dir: Option<std::path::PathBuf>,
    ignore_approval: bool,
) -> anyhow::Result<()> {
    let mut cfg = crate::app_config::load_config_for_session(config.clone(), ignore_approval)
        .await
        .context("加载 anycode 配置")?;
    crate::app_config::apply_wechat_bridge_no_tool_approval(&mut cfg);
    super::wx::run_wechat_daemon(&cfg, config, ignore_approval, data_dir, agent).await
}

#[derive(Serialize)]
struct WechatSendTestOutput {
    ok: bool,
    channel: &'static str,
    marker: Option<String>,
    data_dir: String,
    cron_target: &'static str,
    outbound_log: String,
    message_chars: usize,
}

pub async fn send_test_message(
    data_dir: Option<std::path::PathBuf>,
    message: String,
    json: bool,
) -> anyhow::Result<()> {
    let sent = super::wx::outbound::send_wechat_text(data_dir, message.clone()).await?;
    let data_root = std::path::PathBuf::from(&sent.data_dir);
    let outbound_log = super::wx::outbound_queue::wechat_outbound_log_path(&data_root);
    let marker = message
        .split_whitespace()
        .find(|part| part.starts_with("[anycode-e2e:"))
        .map(|s| s.trim_matches(&['[', ']'][..]).to_string());
    let out = WechatSendTestOutput {
        ok: true,
        channel: "wechat",
        marker,
        data_dir: sent.data_dir,
        cron_target: "present",
        outbound_log: outbound_log.display().to_string(),
        message_chars: sent.message_chars,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("wechat test message sent ({})", out.message_chars);
    }
    Ok(())
}

#[derive(Serialize)]
struct WechatSendMediaTestOutput {
    ok: bool,
    channel: &'static str,
    path: String,
    file_name: String,
    bytes: u64,
    delivery: anycode_tools::WeChatMediaDelivery,
    data_dir: String,
    target_refreshed: bool,
}

pub async fn send_test_media(
    data_dir: Option<std::path::PathBuf>,
    path: String,
    caption: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let sent = super::wx::outbound::send_wechat_media(data_dir, &path, caption.as_deref()).await?;
    let out = WechatSendMediaTestOutput {
        ok: sent.ok,
        channel: "wechat",
        path: sent.path,
        file_name: sent.file_name,
        bytes: sent.bytes,
        delivery: sent.delivery,
        data_dir: sent.data_dir,
        target_refreshed: sent.target_refreshed,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "wechat media sent: {} ({} bytes, delivery={:?})",
            out.file_name, out.bytes, out.delivery
        );
    }
    Ok(())
}
