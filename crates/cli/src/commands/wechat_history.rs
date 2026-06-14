//! Local WeChat chat history setup (wraps `wechat-daily-history` skill script).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn setup_script_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        out.push(PathBuf::from(home).join(".anycode/skills/wechat-daily-history/setup.sh"));
    }
    out.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../skills-starter/wechat-daily-history/setup.sh"),
    );
    out.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/wechat-history-setup.sh"),
    );
    out
}

fn resolve_setup_script() -> anyhow::Result<PathBuf> {
    for p in setup_script_candidates() {
        let canon = p.canonicalize().unwrap_or(p);
        if canon.is_file() {
            return Ok(canon);
        }
    }
    anyhow::bail!(
        "wechat-daily-history setup.sh not found; install skill: anycode skills install-starter"
    )
}

fn run_setup(script: &Path, subcmd: &str, json: bool) -> anyhow::Result<i32> {
    let mut cmd = Command::new(script);
    cmd.arg(subcmd);
    if json {
        cmd.env("ANYCODE_WECHAT_JSON", "1");
    }
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1))
}

pub(crate) fn run_setup_flow(json: bool) -> anyhow::Result<()> {
    let script = resolve_setup_script()?;
    let code = run_setup(&script, "setup", json)?;
    if code != 0 {
        anyhow::bail!("wechat history setup failed (exit {code})");
    }
    Ok(())
}

pub(crate) fn run_status(json: bool) -> anyhow::Result<()> {
    let script = resolve_setup_script()?;
    let code = run_setup(&script, "status", json)?;
    if code != 0 {
        anyhow::bail!("wechat history status failed (exit {code})");
    }
    Ok(())
}
