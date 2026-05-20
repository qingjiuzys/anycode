//! Lightweight local diagnostics for production operations.

use crate::app_config::Config;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
struct CheckRow {
    name: String,
    status: String,
    detail: String,
}

fn home_path(rel: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(rel))
}

fn exists_status(path: &Path) -> &'static str {
    if path.exists() {
        "ok"
    } else {
        "missing"
    }
}

fn print_rows(rows: &[CheckRow], json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(rows)?);
    } else {
        for row in rows {
            println!("{}: {} — {}", row.name, row.status, row.detail);
        }
    }
    Ok(())
}

fn memory_rows(config: &Config) -> Vec<CheckRow> {
    let mut rows = vec![
        CheckRow {
            name: "memory.backend".into(),
            status: "info".into(),
            detail: config.memory.backend.clone(),
        },
        CheckRow {
            name: "memory.path".into(),
            status: exists_status(&config.memory.path).into(),
            detail: config.memory.path.display().to_string(),
        },
    ];
    if matches!(config.memory.backend.as_str(), "hybrid" | "pipeline") {
        let sled_path = config.memory.path.join("memory.sled");
        rows.push(CheckRow {
            name: "memory.sled".into(),
            status: exists_status(&sled_path).into(),
            detail: format!(
                "{} (single-writer; stop long-running bridges if WouldBlock appears)",
                sled_path.display()
            ),
        });
    }
    if config.memory.backend == "pipeline" {
        rows.push(CheckRow {
            name: "memory.embedding".into(),
            status: if config.memory.pipeline.embedding_enabled {
                "enabled"
            } else {
                "disabled"
            }
            .into(),
            detail: format!(
                "provider={} model={}",
                config.memory.embedding_provider,
                config
                    .memory
                    .embedding_model
                    .as_deref()
                    .unwrap_or("<unset>")
            ),
        });
    }
    rows
}

fn channel_rows(channel: &str) -> Vec<CheckRow> {
    let mut rows = Vec::new();
    let want = |name: &str| channel == "all" || channel == name;
    if want("wechat") {
        if let Some(p) = home_path(".anycode/wechat") {
            rows.push(CheckRow {
                name: "channel.wechat.data_dir".into(),
                status: exists_status(&p).into(),
                detail: p.display().to_string(),
            });
            let outbound = p.join("outbound.jsonl");
            let stats = crate::wx::outbound_queue::summarize_outbound_log(&outbound);
            rows.push(CheckRow {
                name: "channel.wechat.outbound".into(),
                status: if outbound.exists() { "ok" } else { "empty" }.into(),
                detail: format!(
                    "pending={} sent={} failed={} ({})",
                    stats.pending,
                    stats.sent,
                    stats.failed,
                    outbound.display()
                ),
            });
        }
        if let Some(p) = home_path(".anycode/wechat/cron_notify_target.json") {
            rows.push(CheckRow {
                name: "channel.wechat.cron_target".into(),
                status: exists_status(&p).into(),
                detail: p.display().to_string(),
            });
        }
    }
    if want("telegram") {
        if let Some(p) = home_path(".anycode/channels/telegram.json") {
            rows.push(CheckRow {
                name: "channel.telegram.credentials".into(),
                status: exists_status(&p).into(),
                detail: p.display().to_string(),
            });
        }
    }
    if want("discord") {
        if let Some(p) = home_path(".anycode/channels/discord.json") {
            rows.push(CheckRow {
                name: "channel.discord.credentials".into(),
                status: exists_status(&p).into(),
                detail: p.display().to_string(),
            });
        }
    }
    rows
}

fn mcp_rows() -> Vec<CheckRow> {
    vec![
        CheckRow {
            name: "mcp.policy".into(),
            status: "manual-reconnect".into(),
            detail: "ADR 007: stdio reconnect is controlled and disabled by default".into(),
        },
        CheckRow {
            name: "mcp.env.command".into(),
            status: if std::env::var_os("ANYCODE_MCP_COMMAND").is_some() {
                "configured"
            } else {
                "unset"
            }
            .into(),
            detail: "ANYCODE_MCP_COMMAND".into(),
        },
        CheckRow {
            name: "mcp.env.servers".into(),
            status: if std::env::var_os("ANYCODE_MCP_SERVERS").is_some() {
                "configured"
            } else {
                "unset"
            }
            .into(),
            detail: "ANYCODE_MCP_SERVERS".into(),
        },
    ]
}

pub(crate) fn print_memory(config: &Config, json: bool) -> anyhow::Result<()> {
    print_rows(&memory_rows(config), json)
}

pub(crate) fn print_channel(channel: &str, json: bool) -> anyhow::Result<()> {
    print_rows(&channel_rows(channel), json)
}

pub(crate) fn print_mcp(json: bool) -> anyhow::Result<()> {
    print_rows(&mcp_rows(), json)
}

pub(crate) fn print_all(config: &Config, json: bool) -> anyhow::Result<()> {
    let mut rows = Vec::new();
    rows.extend(memory_rows(config));
    rows.extend(channel_rows("all"));
    rows.extend(mcp_rows());
    if let Some(p) = home_path(".anycode/tasks/scheduler.lock") {
        rows.push(CheckRow {
            name: "scheduler.lock".into(),
            status: exists_status(&p).into(),
            detail: p.display().to_string(),
        });
    }
    print_rows(&rows, json)
}
