//! Lightweight local diagnostics for production operations.

use crate::app_config::Config;
use anycode_llm::{normalize_provider_id, read_model_fallback, string_field};
use serde::Serialize;
use serde_json::Value;
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

fn fallback_configured(fb: &anycode_llm::ModelFallbackConfig) -> bool {
    fb.provider.as_ref().is_some_and(|s| !s.trim().is_empty())
        && fb.model.as_ref().is_some_and(|s| !s.trim().is_empty())
}

fn load_config_json_for_doctor() -> Option<(PathBuf, Value)> {
    let path = home_path(".anycode/config.json")?;
    let cfg = if path.is_file() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };
    Some((path, cfg))
}

fn llm_rows(config: &Config) -> Vec<CheckRow> {
    let mut rows = Vec::new();
    if let Some((path, cfg)) = load_config_json_for_doctor() {
        rows.push(CheckRow {
            name: "llm.config_path".into(),
            status: exists_status(&path).into(),
            detail: path.display().to_string(),
        });
        let provider = string_field(&cfg, "provider", "provider")
            .unwrap_or_else(|| config.llm.provider.clone());
        let model =
            string_field(&cfg, "model", "model").unwrap_or_else(|| config.llm.model.clone());
        rows.push(CheckRow {
            name: "llm.provider".into(),
            status: if provider.trim().is_empty() {
                "missing"
            } else {
                "ok"
            }
            .into(),
            detail: provider,
        });
        rows.push(CheckRow {
            name: "llm.model".into(),
            status: if model.trim().is_empty() {
                "missing"
            } else {
                "ok"
            }
            .into(),
            detail: model.clone(),
        });
        let api_key = string_field(&cfg, "api_key", "api_key");
        let key_ok = api_key.as_ref().is_some_and(|k| !k.is_empty())
            || !config.llm.api_key.trim().is_empty();
        rows.push(CheckRow {
            name: "llm.api_key".into(),
            status: if key_ok { "ok" } else { "missing" }.into(),
            detail: if key_ok {
                "configured".into()
            } else {
                "not set in config.json".into()
            },
        });
        let fb = read_model_fallback(&cfg);
        let fb_from_runtime = config.runtime.model_fallback.as_ref();
        let fb_ok = fallback_configured(&fb) || fb_from_runtime.is_some_and(fallback_configured);
        rows.push(CheckRow {
            name: "llm.model_fallback".into(),
            status: if fb_ok { "ok" } else { "unset" }.into(),
            detail: if fb_ok {
                let src = fb_from_runtime.unwrap_or(&fb);
                format!(
                    "provider={} model={} on={:?}",
                    src.provider.as_deref().unwrap_or("?"),
                    src.model.as_deref().unwrap_or("?"),
                    src.on
                )
            } else {
                "runtime.model_fallback not configured".into()
            },
        });
        let norm = normalize_provider_id(
            &string_field(&cfg, "provider", "provider")
                .unwrap_or_else(|| config.llm.provider.clone()),
        );
        if norm == "google" && !fb_ok {
            rows.push(CheckRow {
                name: "llm.google_fallback".into(),
                status: "warn".into(),
                detail: "Google provider without model_fallback — failover recommended".into(),
            });
        }
    }
    rows
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
        let sled_path = crate::bootstrap::memory_sled_path_for_diagnostics(&config.memory.path);
        rows.push(CheckRow {
            name: "memory.sled".into(),
            status: exists_status(&sled_path).into(),
            detail: format!(
                "{} (exclusive attach: channel bridges; local REPL/run use shared=file on same path)",
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
            let stats = crate::channels::wx::outbound_queue::summarize_outbound_log(&outbound);
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
        if let Some(p) = home_path(".anycode/tasks/scheduler.lock") {
            rows.push(CheckRow {
                name: "channel.wechat.scheduler".into(),
                status: exists_status(&p).into(),
                detail: format!(
                    "embedded scheduler active when lock present ({})",
                    p.display()
                ),
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
        CheckRow {
            name: "mcp.policy.strict".into(),
            status: if std::env::var_os("ANYCODE_MCP_STRICT").is_some() {
                "configured"
            } else {
                "compat"
            }
            .into(),
            detail: "ANYCODE_MCP_STRICT + ANYCODE_MCP_ALLOWED_TOOLS".into(),
        },
        CheckRow {
            name: "mcp.policy.quota".into(),
            status: if std::env::var_os("ANYCODE_MCP_MAX_CALLS_PER_SERVER").is_some() {
                "configured"
            } else {
                "unlimited"
            }
            .into(),
            detail: "ANYCODE_MCP_MAX_CALLS_PER_SERVER".into(),
        },
    ]
}

fn tool_rows() -> Vec<CheckRow> {
    let mut rows = Vec::new();
    let catalog = anycode_tools::tool_catalog();
    let total = catalog.len();
    let high_risk = catalog
        .iter()
        .filter(|entry| matches!(entry.risk_tier, "high" | "critical"))
        .count();
    let approval_gaps = anycode_tools::catalog::SECURITY_SENSITIVE_TOOL_IDS
        .iter()
        .filter(|id| {
            anycode_tools::tool_catalog_entry(id)
                .map(|entry| !entry.requires_approval)
                .unwrap_or(true)
        })
        .count();
    rows.push(CheckRow {
        name: "tools.catalog.total".into(),
        status: "ok".into(),
        detail: format!("{total} default tool metadata entries"),
    });
    rows.push(CheckRow {
        name: "tools.catalog.high_risk".into(),
        status: if high_risk > 0 { "warn" } else { "ok" }.into(),
        detail: format!("{high_risk} high/critical tools require close approval review"),
    });
    rows.push(CheckRow {
        name: "tools.catalog.approval_gaps".into(),
        status: if approval_gaps == 0 { "ok" } else { "error" }.into(),
        detail: format!("{approval_gaps} sensitive tools missing approval metadata"),
    });
    for entry in catalog {
        rows.push(CheckRow {
            name: format!("tool.{}", entry.id),
            status: entry.risk_tier.into(),
            detail: format!(
                "category={} approval={} audit={} agents={}",
                entry.category,
                entry.requires_approval,
                entry.audit_level,
                entry.default_agents.join(",")
            ),
        });
    }
    rows
}

pub(crate) fn print_tools(json: bool) -> anyhow::Result<()> {
    print_rows(&tool_rows(), json)
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
    rows.extend(llm_rows(config));
    rows.extend(memory_rows(config));
    rows.extend(channel_rows("all"));
    rows.extend(mcp_rows());
    rows.extend(tool_rows());
    if let Some(p) = home_path(".anycode/tasks/scheduler.lock") {
        rows.push(CheckRow {
            name: "scheduler.lock".into(),
            status: exists_status(&p).into(),
            detail: p.display().to_string(),
        });
    }
    print_rows(&rows, json)
}
