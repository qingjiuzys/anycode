//! WeChat bridge user autostart: macOS LaunchAgent (RunAtLoad + KeepAlive); Linux systemd --user.
//! Installed after `anycode channel wechat` bind success; service uses hidden `--run-as-bridge`.

use crate::i18n::{tr, tr_args};
use anyhow::{Context, Result};
use fluent_bundle::FluentArgs;
use std::fs;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "macos")]
pub const LAUNCHD_LABEL: &str = "dev.anycode.wechat";

#[cfg(target_os = "linux")]
const SYSTEMD_UNIT: &str = "anycode-wechat.service";

#[derive(Debug, Clone)]
pub struct WechatServiceSpec {
    pub binary: PathBuf,
    pub config: Option<PathBuf>,
    pub debug: bool,
    pub agent: String,
    pub data_dir: Option<PathBuf>,
    pub register_only: bool,
}

#[cfg(target_os = "macos")]
fn xml_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

fn build_argv(spec: &WechatServiceSpec) -> Result<Vec<String>> {
    let abs = spec.binary.canonicalize().with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", format!("{:?}", spec.binary));
        tr_args("wx-svc-ctx-resolve-binary", &a)
    })?;
    if !abs.is_file() {
        let mut a = FluentArgs::new();
        a.set("path", abs.display().to_string());
        anyhow::bail!("{}", tr_args("wx-svc-err-binary-missing", &a));
    }

    let mut v = vec![abs.to_string_lossy().to_string()];
    if spec.debug {
        v.push("--debug".into());
    }
    if let Some(ref c) = spec.config {
        let c = c.canonicalize().with_context(|| {
            let mut a = FluentArgs::new();
            a.set("path", format!("{:?}", c));
            tr_args("wx-svc-ctx-resolve-config", &a)
        })?;
        v.push("-c".into());
        v.push(c.to_string_lossy().to_string());
    }
    v.push("channel".into());
    v.push("wechat".into());
    v.push("--run-as-bridge".into());
    v.push("--agent".into());
    v.push(spec.agent.clone());
    if let Some(ref d) = spec.data_dir {
        v.push("--data-dir".into());
        v.push(d.to_string_lossy().to_string());
    }
    Ok(v)
}

#[cfg(target_os = "macos")]
fn log_paths() -> Result<(PathBuf, PathBuf)> {
    let home = dirs::home_dir().context(tr("wx-svc-err-no-home"))?;
    let log_dir = home.join(".anycode").join("logs");
    fs::create_dir_all(&log_dir).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", log_dir.display().to_string());
        tr_args("wx-svc-ctx-mkdir-logs", &a)
    })?;
    Ok((
        log_dir.join("wechat-bridge.out.log"),
        log_dir.join("wechat-bridge.err.log"),
    ))
}

#[cfg(target_os = "macos")]
fn launch_agents_plist_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context(tr("wx-svc-ctx-home-short"))?;
    Ok(home
        .join("Library/LaunchAgents")
        .join(format!("{LAUNCHD_LABEL}.plist")))
}

#[cfg(target_os = "macos")]
fn launchctl_output(args: &[&str]) -> Result<std::process::Output> {
    Command::new("launchctl")
        .args(args)
        .output()
        .with_context(|| {
            let mut a = FluentArgs::new();
            a.set("cmd", format!("launchctl {}", args.join(" ")));
            tr_args("wx-svc-ctx-launchctl-run", &a)
        })
}

#[cfg(target_os = "macos")]
fn launchd_job_loaded(domain: &str, label: &str) -> bool {
    let target = format!("{domain}/{label}");
    launchctl_output(&["print", &target])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 卸载已加载的 LaunchAgent（首次安装时 bootout 会失败，属正常）。
#[cfg(target_os = "macos")]
fn launchd_bootout(domain: &str, plist_path: &Path, label: &str) {
    let target = format!("{domain}/{label}");
    let plist_s = plist_path.to_string_lossy();
    let _ = launchctl_output(&["bootout", &target]);
    let _ = launchctl_output(&["bootout", domain, &plist_s]);
}

#[cfg(target_os = "macos")]
fn os_uid() -> Result<u32> {
    let o = Command::new("id")
        .arg("-u")
        .output()
        .context(tr("wx-svc-ctx-run-id-u"))?;
    if !o.status.success() {
        anyhow::bail!("{}", tr("wx-svc-err-id-u"));
    }
    String::from_utf8_lossy(&o.stdout)
        .trim()
        .parse()
        .context(tr("wx-svc-ctx-parse-uid"))
}

#[cfg(target_os = "macos")]
fn render_launchd_plist(
    argv: &[String],
    out_log: &Path,
    err_log: &Path,
    env_block: &str,
) -> String {
    let mut args_xml = String::new();
    for a in argv {
        args_xml.push_str("    <string>");
        args_xml.push_str(&xml_escape(a));
        args_xml.push_str("</string>\n");
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
{args_xml}  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>10</integer>
  <key>WorkingDirectory</key>
  <string>/</string>
  <key>ProcessType</key>
  <string>Interactive</string>
  <key>StandardOutPath</key>
  <string>{out}</string>
  <key>StandardErrorPath</key>
  <string>{err}</string>
{env_block}
</dict>
</plist>
"#,
        label = xml_escape(LAUNCHD_LABEL),
        args_xml = args_xml,
        out = xml_escape(&out_log.to_string_lossy()),
        err = xml_escape(&err_log.to_string_lossy()),
        env_block = env_block
    )
}

#[cfg(target_os = "macos")]
fn macos_env_plist_block(spec: &WechatServiceSpec) -> String {
    let mut pairs: Vec<(String, String)> = Vec::new();
    pairs.push((
        "PATH".into(),
        std::env::var("PATH")
            .unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin".into()),
    ));
    if let Some(ref d) = spec.data_dir {
        if let Some(s) = d.to_str() {
            pairs.push(("WCC_DATA_DIR".into(), s.to_string()));
        }
    }
    if pairs.is_empty() {
        return String::new();
    }
    let mut s = String::from("  <key>EnvironmentVariables</key>\n  <dict>\n");
    for (k, v) in pairs {
        s.push_str("    <key>");
        s.push_str(&xml_escape(&k));
        s.push_str("</key>\n    <string>");
        s.push_str(&xml_escape(&v));
        s.push_str("</string>\n");
    }
    s.push_str("  </dict>\n");
    s
}

#[cfg(target_os = "macos")]
pub fn install(spec: WechatServiceSpec) -> Result<()> {
    let argv = build_argv(&spec)?;
    let (out_log, err_log) = log_paths()?;
    let env_block = macos_env_plist_block(&spec);
    let plist = render_launchd_plist(&argv, &out_log, &err_log, &env_block);
    let plist_path = launch_agents_plist_path()?;
    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            let mut a = FluentArgs::new();
            a.set("path", parent.display().to_string());
            tr_args("wx-svc-ctx-mkdir", &a)
        })?;
    }
    fs::write(&plist_path, plist.as_bytes()).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", plist_path.display().to_string());
        tr_args("wx-svc-ctx-write", &a)
    })?;

    let domain = format!("gui/{}", os_uid()?);
    let target = format!("{}/{}", domain, LAUNCHD_LABEL);

    launchd_bootout(&domain, &plist_path, LAUNCHD_LABEL);

    let plist_s = plist_path.to_str().with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", format!("{:?}", plist_path));
        tr_args("wx-svc-ctx-plist-utf8", &a)
    })?;

    if !launchd_job_loaded(&domain, LAUNCHD_LABEL) {
        let out = launchctl_output(&["bootstrap", &domain, plist_s])
            .context(tr("wx-svc-ctx-launchctl-bootstrap"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            // 已加载时 bootstrap 常报 exit 5（Input/output error）；若 print 显示已存在则视为成功。
            if launchd_job_loaded(&domain, LAUNCHD_LABEL) {
                eprintln!("{}", tr("wx-svc-warn-bootstrap-already-loaded"));
            } else {
                let mut a = FluentArgs::new();
                a.set("code", format!("{:?}", out.status.code()));
                a.set(
                    "detail",
                    if stderr.is_empty() {
                        tr("wx-svc-err-bootstrap-no-detail")
                    } else {
                        stderr
                    },
                );
                anyhow::bail!("{}", tr_args("wx-svc-err-bootstrap-detail", &a));
            }
        }
    } else {
        eprintln!("{}", tr("wx-svc-warn-bootstrap-already-loaded"));
    }

    let _ = launchctl_output(&["enable", &target]);

    if !spec.register_only {
        let out =
            launchctl_output(&["kickstart", "-k", &target]).context(tr("wx-svc-ctx-kickstart"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let mut a = FluentArgs::new();
            a.set("code", format!("{:?}", out.status.code()));
            a.set(
                "detail",
                if stderr.is_empty() {
                    tr("wx-svc-err-kickstart-no-detail")
                } else {
                    stderr
                },
            );
            anyhow::bail!("{}", tr_args("wx-svc-err-kickstart-detail", &a));
        }
    }

    let mut p1 = FluentArgs::new();
    p1.set("path", plist_path.display().to_string());
    println!("{}", tr_args("wx-svc-out-launched", &p1));
    let mut p2 = FluentArgs::new();
    p2.set("label", LAUNCHD_LABEL.to_string());
    println!("{}", tr_args("wx-svc-out-label", &p2));
    if !spec.register_only {
        println!("{}", tr("wx-svc-out-kickstart"));
    }
    let mut pl = FluentArgs::new();
    pl.set("out", out_log.display().to_string());
    pl.set("err", err_log.display().to_string());
    println!("{}", tr_args("wx-svc-out-logs", &pl));
    Ok(())
}

#[cfg(target_os = "linux")]
fn systemd_user_unit_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context(tr("wx-svc-ctx-home-short"))?;
    let dir = home.join(".config/systemd/user");
    fs::create_dir_all(&dir).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", dir.display().to_string());
        tr_args("wx-svc-ctx-mkdir", &a)
    })?;
    Ok(dir.join(SYSTEMD_UNIT))
}

#[cfg(target_os = "linux")]
fn systemd_exec_line(argv: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for p in argv {
        let q = if p.contains(' ') || p.contains('"') || p.contains('\'') {
            format!("'{}'", p.replace('\'', "'\"'\"'"))
        } else {
            p.clone()
        };
        parts.push(q);
    }
    let joined = parts.join(" ");
    format!("/bin/bash -lc 'exec {joined}'")
}

#[cfg(target_os = "linux")]
pub fn install(spec: WechatServiceSpec) -> Result<()> {
    let argv = build_argv(&spec)?;
    let exec_line = systemd_exec_line(&argv);
    let unit_path = systemd_user_unit_path()?;
    let env_line = spec
        .data_dir
        .as_ref()
        .and_then(|d| d.to_str())
        .map(|s| {
            if s.chars().any(|c| c.is_whitespace()) {
                format!("Environment=\"WCC_DATA_DIR={}\"\n", s.replace('"', "\\\""))
            } else {
                format!("Environment=WCC_DATA_DIR={}\n", s)
            }
        })
        .unwrap_or_default();
    let unit = format!(
        r#"[Unit]
Description=anyCode WeChat iLink bridge
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
{env_line}ExecStart={exec_line}
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
"#
    );

    fs::write(&unit_path, unit.as_bytes()).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", unit_path.display().to_string());
        tr_args("wx-svc-ctx-write", &a)
    })?;

    let st = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .context(tr("wx-svc-ctx-systemd-reload"))?;
    if !st.success() {
        anyhow::bail!("{}", tr("wx-svc-err-daemon-reload"));
    }

    let st = Command::new("systemctl")
        .args(["--user", "enable", SYSTEMD_UNIT])
        .status()
        .context(tr("wx-svc-ctx-systemd-enable"))?;
    if !st.success() {
        anyhow::bail!("{}", tr("wx-svc-err-enable"));
    }

    if !spec.register_only {
        let st = Command::new("systemctl")
            .args(["--user", "restart", SYSTEMD_UNIT])
            .status()
            .context(tr("wx-svc-ctx-systemd-restart"))?;
        if !st.success() {
            let _ = Command::new("systemctl")
                .args(["--user", "start", SYSTEMD_UNIT])
                .status();
        }
    }

    let mut u1 = FluentArgs::new();
    u1.set("path", unit_path.display().to_string());
    println!("{}", tr_args("wx-svc-out-systemd", &u1));
    let mut u2 = FluentArgs::new();
    u2.set("unit", SYSTEMD_UNIT.to_string());
    println!("{}", tr_args("wx-svc-out-unit", &u2));
    if !spec.register_only {
        println!("{}", tr("wx-svc-out-restarted"));
    }
    println!("{}", tr("wx-svc-out-linger"));
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn install(_spec: WechatServiceSpec) -> Result<()> {
    anyhow::bail!("{}", tr("wx-svc-err-unsupported"));
}

/// After `anycode channel wechat` scan success: install autostart for current exe + data root.
pub fn install_autostart_after_setup(
    data_root: PathBuf,
    config: Option<PathBuf>,
    debug: bool,
) -> Result<()> {
    let binary = std::env::current_exe().context(tr("wx-svc-ctx-current-exe"))?;
    install(WechatServiceSpec {
        binary,
        config,
        debug,
        agent: "workspace-assistant".into(),
        data_dir: Some(data_root),
        register_only: false,
    })
}
