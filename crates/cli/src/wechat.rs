//! `anycode channel wechat`：扫码绑定并安装后台服务；`--run-as-bridge` 仅供系统服务调用。

use anyhow::Context;

/// 扫码 → 写入账号 → 安装 LaunchAgent/systemd 并拉起桥接进程。
pub async fn run_onboard(
    data_dir: Option<std::path::PathBuf>,
    config: Option<std::path::PathBuf>,
    debug: bool,
) -> anyhow::Result<()> {
    let dir = data_dir.clone();
    crate::wechat_ilink::run_interactive_setup(data_dir).await?;
    let root = crate::wx::wcc_data_dir(dir);
    crate::wechat_service::install_autostart_after_setup(root, config, debug)?;
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
    crate::wx::run_wechat_daemon(&cfg, config, ignore_approval, data_dir, agent).await
}
