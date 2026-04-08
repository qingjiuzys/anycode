//! `anycode statusline print-schema` — 调试 status line 的 stdin JSON。

use crate::app_config::Config;
use crate::tui::status_line::build_status_line_payload;

pub(crate) fn print_schema(config: &Config) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let cwd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
    let cwd_s = cwd.to_string_lossy();
    let bytes = build_status_line_payload(
        env!("CARGO_PKG_VERSION"),
        "sample-session-id",
        cwd_s.as_ref(),
        cwd_s.as_ref(),
        config.llm.model.as_str(),
        &config.session,
        config.llm.provider.as_str(),
        0,
        None,
    )?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
