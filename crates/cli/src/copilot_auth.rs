//! GitHub 设备码登录，写入 `~/.anycode/credentials/github-oauth.json`（对齐 OpenClaw Copilot 插件 scope）。

use anyhow::Context;
use serde::Deserialize;
use std::io::{stdin, IsTerminal};
use std::time::Duration;

const CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const SCOPE: &str = "read:user";

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

pub async fn run_github_copilot_device_login() -> anyhow::Result<()> {
    if !stdin().is_terminal() {
        anyhow::bail!("GitHub Copilot 设备码登录需要交互式终端（TTY）。");
    }

    let client = reqwest::Client::new();
    let device: DeviceCodeResponse = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
        .send()
        .await
        .context("request device code")?
        .error_for_status()
        .context("device code HTTP")?
        .json()
        .await
        .context("parse device code")?;

    println!();
    println!("在浏览器打开: {}", device.verification_uri);
    println!("输入代码: {}", device.user_code);
    println!();

    let expires_at =
        std::time::Instant::now() + Duration::from_secs(device.expires_in.max(1));
    let mut interval_ms = (device.interval.max(1)) * 1000;

    let access_token = loop {
        if std::time::Instant::now() >= expires_at {
            anyhow::bail!("设备码已过期，请重新运行 `anycode model auth copilot`");
        }
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;

        let text = client
            .post(ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", device.device_code.as_str()),
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code",
                ),
            ])
            .send()
            .await
            .context("poll token")?
            .text()
            .await
            .context("read token body")?;

        let v: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("parse token JSON: {}", text.chars().take(120).collect::<String>()))?;

        if let Some(tok) = v.get("access_token").and_then(|x| x.as_str()) {
            break tok.to_string();
        }
        let err = v
            .get("error")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown");
        match err {
            "authorization_pending" => continue,
            "slow_down" => {
                interval_ms += 2000;
                continue;
            }
            "expired_token" => {
                anyhow::bail!("设备码已过期，请重新运行");
            }
            "access_denied" => {
                anyhow::bail!("已取消授权");
            }
            other => {
                anyhow::bail!("GitHub OAuth: {} — {}", other, text.chars().take(200).collect::<String>());
            }
        }
    };

    let dir = anycode_llm::anycode_credentials_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let path = anycode_llm::github_oauth_token_path();
    let payload = serde_json::json!({ "access_token": access_token });
    std::fs::write(&path, serde_json::to_vec_pretty(&payload)?)
        .with_context(|| format!("write {}", path.display()))?;

    println!("已保存 GitHub token: {}", path.display());
    Ok(())
}
