//! Minimal production-readiness eval harness.
//!
//! This intentionally starts with deterministic, no-credential checks. Model-backed
//! SWE-style evals can build on the same scenario schema later.

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
struct EvalScenario {
    id: &'static str,
    area: &'static str,
    command: &'static str,
    expect: &'static str,
    acceptance: &'static str,
}

const SCENARIOS: &[EvalScenario] = &[
    EvalScenario {
        id: "cli-help",
        area: "cli",
        command: "--help",
        expect: "anyCode",
        acceptance: "prints top-level command help without loading provider credentials",
    },
    EvalScenario {
        id: "status-json",
        area: "config",
        command: "status --json",
        expect: "model",
        acceptance: "prints machine-readable model/mode/security status",
    },
    EvalScenario {
        id: "cron-ledger",
        area: "automation",
        command: "cron runs --limit 5 --json",
        expect: "[",
        acceptance: "reads cron-runs.jsonl or reports an empty ledger",
    },
    EvalScenario {
        id: "doctor-all",
        area: "ops",
        command: "doctor all --json",
        expect: "memory.backend",
        acceptance: "reports local config, memory, channel, and MCP diagnostics",
    },
    EvalScenario {
        id: "memory-doctor",
        area: "memory",
        command: "memory doctor --json",
        expect: "memory.backend",
        acceptance: "reports backend, path, and common lock-risk hints",
    },
    EvalScenario {
        id: "doctor-errors",
        area: "ops",
        command: "doctor errors --json",
        expect: "eval.scenario_failed",
        acceptance: "prints structured CLI error taxonomy reference",
    },
    EvalScenario {
        id: "mcp-status",
        area: "mcp",
        command: "mcp status --json",
        expect: "policy.reconnect",
        acceptance: "reports MCP reconnect policy and env hints without live servers",
    },
];

#[derive(Debug, Serialize)]
struct EvalRunRow {
    id: &'static str,
    status: &'static str,
    detail: String,
    exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
}

fn eval_binary() -> anyhow::Result<std::path::PathBuf> {
    if let Ok(from_env) = std::env::var("ANYCODE_EVAL_BIN") {
        let p = from_env.trim();
        if !p.is_empty() {
            return Ok(std::path::PathBuf::from(p));
        }
    }
    std::env::current_exe().map_err(Into::into)
}

fn run_scenario(bin: &std::path::Path, s: &EvalScenario) -> EvalRunRow {
    let home = std::env::temp_dir().join(format!("anycode-eval-{}", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::create_dir_all(&home) {
        return EvalRunRow {
            id: s.id,
            status: "fail",
            detail: format!("temp home: {e}"),
            exit_code: -1,
            error_code: Some("internal.unclassified".into()),
        };
    }
    let mut cmd = Command::new(bin);
    for part in s.command.split_whitespace() {
        cmd.arg(part);
    }
    cmd.env("HOME", &home);
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&home);
            return EvalRunRow {
                id: s.id,
                status: "fail",
                detail: format!("spawn failed: {e}"),
                exit_code: -1,
                error_code: Some("internal.unclassified".into()),
            };
        }
    };
    let _ = std::fs::remove_dir_all(&home);
    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    let pass = output.status.success() && combined.contains(s.expect);
    EvalRunRow {
        id: s.id,
        status: if pass { "pass" } else { "fail" },
        detail: if pass {
            s.acceptance.to_string()
        } else {
            format!(
                "expected {:?} in output; exit={}; tail={}",
                s.expect,
                output.status.code().unwrap_or(-1),
                combined
                    .chars()
                    .rev()
                    .take(240)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>()
            )
        },
        exit_code: output.status.code().unwrap_or(-1),
        error_code: if pass {
            None
        } else {
            Some(
                crate::commands::cli_error::classify(&anyhow::anyhow!(
                    "eval harness scenario {} failed",
                    s.id
                ))
                .code,
            )
        },
    }
}

pub(crate) fn list(json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(SCENARIOS)?);
        return Ok(());
    }
    for s in SCENARIOS {
        println!(
            "{} [{}]\n  command: anycode {}\n  acceptance: {}",
            s.id, s.area, s.command, s.acceptance
        );
    }
    Ok(())
}

pub(crate) fn run(json: bool, include_mock: bool) -> anyhow::Result<()> {
    let bin = eval_binary()?;
    let mut rows: Vec<EvalRunRow> = SCENARIOS.iter().map(|s| run_scenario(&bin, s)).collect();
    if include_mock {
        let mock = super::eval_mock::run_mock_fixture_task(&bin);
        rows.push(EvalRunRow {
            id: mock.id,
            status: mock.status,
            detail: mock.detail,
            exit_code: mock.exit_code,
            error_code: if mock.status == "fail" {
                Some(
                    crate::commands::cli_error::classify(&anyhow::anyhow!(
                        "eval harness scenario mock-fixture-run failed"
                    ))
                    .code,
                )
            } else {
                None
            },
        });
    }
    let failed = rows.iter().any(|r| r.status == "fail");
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for r in &rows {
            println!("{}: {} — {}", r.id, r.status, r.detail);
        }
    }
    if failed {
        anyhow::bail!("eval harness: one or more scenarios failed");
    }
    Ok(())
}
