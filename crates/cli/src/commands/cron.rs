//! Cron observability commands.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
struct CronRunRow {
    line_no: usize,
    raw: serde_json::Value,
}

fn cron_runs_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/logs/cron-runs.jsonl"))
}

pub(crate) fn print_runs(
    job: Option<String>,
    session: Option<String>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    let Some(path) = cron_runs_path() else {
        if json {
            println!("[]");
        }
        return Ok(());
    };
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut rows = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let raw: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|_| {
            serde_json::json!({
                "parse_error": true,
                "line": line,
            })
        });
        if let Some(ref job_id) = job {
            if raw.get("job_id").and_then(|v| v.as_str()) != Some(job_id.as_str()) {
                continue;
            }
        }
        if let Some(ref session_id) = session {
            if raw.get("session_id").and_then(|v| v.as_str()) != Some(session_id.as_str()) {
                continue;
            }
        }
        rows.push(CronRunRow {
            line_no: idx + 1,
            raw,
        });
    }
    let keep = limit.max(1);
    if rows.len() > keep {
        rows = rows.split_off(rows.len() - keep);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else if rows.is_empty() {
        println!("No cron run entries found at {}.", path.display());
    } else {
        for row in rows {
            println!("#{} {}", row.line_no, row.raw);
        }
    }
    Ok(())
}
