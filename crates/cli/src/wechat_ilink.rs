//! WeChat iLink QR login (HTTP aligned with wechat-claude-code `src/wechat/login.ts`).

use crate::i18n::{tr, tr_args};
use crate::wx::wcc_data_dir;
use anyhow::Context;
use fluent_bundle::FluentArgs;
use qrcode::render::unicode::Dense1x2;
use qrcode::QrCode;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const POLL_HTTP_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountFile {
    bot_token: String,
    account_id: String,
    base_url: String,
    user_id: String,
    created_at: String,
}

fn json_str<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            return Some(s);
        }
    }
    None
}

async fn fetch_qr(client: &reqwest::Client, base: &str) -> anyhow::Result<(String, String)> {
    let url = format!(
        "{}/ilink/bot/get_bot_qrcode?bot_type=3",
        base.trim_end_matches('/')
    );
    let res = client
        .get(&url)
        .send()
        .await
        .context(tr("wx-ilink-ctx-fetch-qr"))?;
    if !res.status().is_success() {
        let mut a = FluentArgs::new();
        a.set("status", res.status().to_string());
        anyhow::bail!("{}", tr_args("wx-ilink-err-qr-http", &a));
    }
    let v: serde_json::Value = res
        .json()
        .await
        .context(tr("wx-ilink-ctx-parse-qr-json"))?;
    let ret = v.get("ret").and_then(|x| x.as_i64()).unwrap_or(-1);
    let qrcode_id = json_str(&v, &["qrcode"]).map(str::to_string);
    let content = json_str(&v, &["qrcode_img_content", "qrcodeImgContent"]).map(str::to_string);
    if ret != 0 || qrcode_id.is_none() || content.is_none() {
        let mut a = FluentArgs::new();
        a.set("ret", ret);
        anyhow::bail!("{}", tr_args("wx-ilink-err-qr-ret", &a));
    }
    Ok((qrcode_id.unwrap(), content.unwrap()))
}

async fn poll_once(
    client: &reqwest::Client,
    base: &str,
    qrcode_id: &str,
) -> anyhow::Result<serde_json::Value> {
    let url = format!(
        "{}/ilink/bot/get_qrcode_status?qrcode={}",
        base.trim_end_matches('/'),
        urlencoding::encode(qrcode_id)
    );
    let res = client
        .get(&url)
        .timeout(POLL_HTTP_TIMEOUT)
        .send()
        .await
        .context(tr("wx-ilink-ctx-poll-status"))?;
    if !res.status().is_success() {
        let mut a = FluentArgs::new();
        a.set("status", res.status().to_string());
        anyhow::bail!("{}", tr_args("wx-ilink-err-poll-http", &a));
    }
    res.json()
        .await
        .context(tr("wx-ilink-ctx-parse-status-json"))
}

fn validate_account_id(id: &str) -> anyhow::Result<()> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.@=-".contains(c))
    {
        let mut a = FluentArgs::new();
        a.set("id", id.to_string());
        anyhow::bail!("{}", tr_args("wx-ilink-err-bad-account", &a));
    }
    Ok(())
}

fn save_account(data_root: &Path, v: &serde_json::Value) -> anyhow::Result<()> {
    let bot_token = json_str(v, &["bot_token", "botToken"]).context(tr("wx-ilink-err-missing-bot-token"))?;
    let account_id =
        json_str(v, &["ilink_bot_id", "ilinkBotId"]).context(tr("wx-ilink-err-missing-ilink-bot-id"))?;
    let user_id =
        json_str(v, &["ilink_user_id", "ilinkUserId"]).context(tr("wx-ilink-err-missing-ilink-user-id"))?;
    validate_account_id(account_id)?;

    let base_url = json_str(v, &["baseurl", "baseUrl"])
        .unwrap_or(DEFAULT_BASE_URL)
        .to_string();

    let acc = AccountFile {
        bot_token: bot_token.to_string(),
        account_id: account_id.to_string(),
        base_url,
        user_id: user_id.to_string(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };

    let dir = data_root.join("accounts");
    std::fs::create_dir_all(&dir).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", dir.display().to_string());
        tr_args("wx-ilink-ctx-mkdir", &a)
    })?;
    let path = dir.join(format!("{}.json", account_id));
    let raw = serde_json::to_string_pretty(&acc)
        .context(tr("wx-ilink-ctx-serialize-account"))?
        + "\n";
    std::fs::write(&path, raw.as_bytes()).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", path.display().to_string());
        tr_args("wx-ilink-ctx-write-path", &a)
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    tracing::info!(
        path = %path.display(),
        account_id = %account_id,
        "{}",
        tr("wx-ilink-log-account-saved")
    );
    Ok(())
}

/// Unicode half-blocks (▄▀█) like OpenClaw `openclaw-weixin` for denser terminal QR.
fn render_qr_terminal(payload: &str) -> anyhow::Result<String> {
    let code = QrCode::new(payload.as_bytes()).context(tr("wx-ilink-ctx-qr-data"))?;
    Ok(code
        .render::<Dense1x2>()
        .quiet_zone(true)
        .module_dimensions(1, 1)
        .build())
}

async fn wait_confirmed(
    client: &reqwest::Client,
    base: &str,
    qrcode_id: &str,
    data_root: &Path,
) -> anyhow::Result<()> {
    loop {
        let data = match poll_once(client, base, qrcode_id).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(error = %e, "{}", tr("wx-ilink-log-poll-retry"));
                tokio::time::sleep(POLL_INTERVAL).await;
                continue;
            }
        };

        let status = json_str(&data, &["status"]).unwrap_or_default();
        match status {
            "wait" | "scaned" | "scanned" => {}
            "confirmed" => {
                save_account(data_root, &data)?;
                return Ok(());
            }
            "expired" => {
                anyhow::bail!("QR_EXPIRED");
            }
            other => {
                let retmsg = json_str(&data, &["retmsg", "retMsg"]).unwrap_or("");
                let s = format!("{}{}", other, retmsg);
                if s.contains("not_support")
                    || s.contains("version")
                    || s.contains("forbid")
                    || s.contains("reject")
                    || s.contains("cancel")
                {
                    let mut a = FluentArgs::new();
                    a.set("msg", retmsg.to_string());
                    anyhow::bail!("{}", tr_args("wx-ilink-err-scan", &a));
                }
                if !retmsg.is_empty() {
                    let mut a = FluentArgs::new();
                    a.set("msg", retmsg.to_string());
                    anyhow::bail!("{}", tr_args("wx-ilink-err-scan", &a));
                }
                tracing::warn!(status = %other, "{}", tr("wx-ilink-log-unknown-status"));
            }
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

fn write_config_env(data_root: &Path, working_dir: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(data_root).context(tr("wx-ilink-ctx-create-data-root"))?;
    let path = data_root.join("config.env");
    let line = format!("workingDirectory={}\n", working_dir);
    std::fs::write(&path, line.as_bytes()).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", path.display().to_string());
        tr_args("wx-ilink-ctx-write-path", &a)
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

/// Interactive QR setup: refresh expired codes like Node `runSetup`; QR only in terminal + optional URL hint.
pub async fn run_interactive_setup(data_dir: Option<PathBuf>) -> anyhow::Result<()> {
    crate::workspace::ensure_layout()
        .context(tr("wx-ilink-ctx-workspace-init"))?;
    let base = std::env::var("WCC_ILINK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let data_root = wcc_data_dir(data_dir);
    std::fs::create_dir_all(&data_root)
        .context(tr("wx-ilink-ctx-create-data-root"))?;

    let client = reqwest::Client::builder()
        .user_agent("anycode-wechat-ilink/0.1")
        .build()
        .context(tr("wx-ilink-http-client"))?;

    println!("{}\n", tr("wx-ilink-banner"));

    loop {
        let (qrcode_id, payload) = fetch_qr(&client, &base).await?;
        tracing::debug!(qrcode_id = %qrcode_id, "{}", tr("wx-ilink-debug-qr-fetched"));

        println!("{}\n", tr("wx-ilink-scan-title"));
        match render_qr_terminal(&payload) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                let mut a = FluentArgs::new();
                a.set("err", e.to_string());
                println!("{}", tr_args("wx-ilink-qr-render-fail", &a));
            }
        }
        println!();
        if payload.trim_start().starts_with("http://")
            || payload.trim_start().starts_with("https://")
        {
            println!("{}", tr("wx-ilink-browser-hint"));
            println!("{}\n", payload.trim());
        } else {
            println!("{}\n{}\n", tr("wx-ilink-copy-payload-hint"), payload);
        }

        println!("{}", tr("wx-ilink-wait-scan"));

        match wait_confirmed(&client, &base, &qrcode_id, &data_root).await {
            Ok(()) => {
                println!("{}", tr("wx-ilink-bind-ok"));
                break;
            }
            Err(e) => {
                let msg = format!("{:#}", e);
                if msg.contains("QR_EXPIRED") {
                    println!("{}\n", tr("wx-ilink-qr-expired-hint"));
                    continue;
                }
                return Err(e);
            }
        }
    }

    let def = crate::workspace::canonical_root_string();
    let mut intro = FluentArgs::new();
    intro.set("path", def.clone());
    println!("{}\n", tr_args("wx-ilink-workdir-intro", &intro));
    let wd: String = dialoguer::Input::new()
        .with_prompt(tr("wx-ilink-workdir-prompt"))
        .default(def.clone())
        .interact_text()
        .context(tr("wx-ilink-ctx-read-workdir"))?;
    let wd_trim = wd.trim();
    let final_wd = if wd_trim.is_empty() {
        def
    } else {
        wd_trim.to_string()
    };
    write_config_env(&data_root, &final_wd)?;
    let mut wp = FluentArgs::new();
    wp.set("path", data_root.join("config.env").display().to_string());
    println!("{}\n", tr_args("wx-ilink-wrote-env", &wp));

    Ok(())
}
