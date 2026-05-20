//! Minimal production-readiness eval harness.
//!
//! Scenarios are loaded from `scripts/eval/scenarios.json` (shared with `scripts/eval/run.py`).

use crate::eval_manifest::{cli_scenarios, mock_fixture_scenarios, CliEvalScenario};
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
struct EvalRunRow {
    id: String,
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

fn run_scenario(bin: &std::path::Path, s: &CliEvalScenario) -> EvalRunRow {
    let home = std::env::temp_dir().join(format!("anycode-eval-{}", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::create_dir_all(&home) {
        return EvalRunRow {
            id: s.id.clone(),
            status: "fail",
            detail: format!("temp home: {e}"),
            exit_code: -1,
            error_code: Some("internal.unclassified".into()),
        };
    }
    let mut cmd = Command::new(bin);
    for part in &s.command {
        cmd.arg(part);
    }
    cmd.env("HOME", &home);
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&home);
            return EvalRunRow {
                id: s.id.clone(),
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
    let pass = output.status.success() && combined.contains(&s.expect);
    EvalRunRow {
        id: s.id.clone(),
        status: if pass { "pass" } else { "fail" },
        detail: if pass {
            s.acceptance.clone()
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

#[derive(Debug, Serialize)]
struct MockFixtureListRow {
    id: String,
    area: String,
    fixture: String,
    acceptance: String,
    requires_mock: bool,
}

pub(crate) fn list(json: bool) -> anyhow::Result<()> {
    let cli = cli_scenarios();
    let mock_rows: Vec<MockFixtureListRow> = mock_fixture_scenarios()
        .into_iter()
        .map(|m| MockFixtureListRow {
            id: m.id,
            area: m.area,
            fixture: m.fixture,
            acceptance: m.acceptance,
            requires_mock: true,
        })
        .collect();
    if json {
        #[derive(Serialize)]
        struct EvalListPayload {
            cli: Vec<CliEvalScenario>,
            mock_fixtures: Vec<MockFixtureListRow>,
        }
        let payload = EvalListPayload {
            cli,
            mock_fixtures: mock_rows,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }
    for s in cli_scenarios() {
        println!(
            "{} [{}]\n  command: anycode {}\n  acceptance: {}",
            s.id,
            s.area,
            s.command.join(" "),
            s.acceptance
        );
    }
    println!("\nMock fixture repo tasks (run with `anycode eval run --mock`):");
    for m in mock_fixture_scenarios() {
        println!(
            "{} [{}]\n  fixture: scripts/eval/fixtures/{}\n  acceptance: {}",
            m.id, m.area, m.fixture, m.acceptance
        );
    }
    Ok(())
}

pub(crate) fn run(json: bool, include_mock: bool) -> anyhow::Result<()> {
    if std::env::var("ANYCODE_EVAL_TOOLCHAIN_HOME").is_err() {
        if let Ok(home) = std::env::var("HOME") {
            std::env::set_var("ANYCODE_EVAL_TOOLCHAIN_HOME", home);
        }
    }
    let bin = eval_binary()?;
    let mut rows: Vec<EvalRunRow> = cli_scenarios()
        .iter()
        .map(|s| run_scenario(&bin, s))
        .collect();
    if include_mock {
        for mock in super::eval_mock::run_mock_fixture_scenarios(&bin) {
            rows.push(EvalRunRow {
                id: mock.id.to_string(),
                status: mock.status,
                detail: mock.detail,
                exit_code: mock.exit_code,
                error_code: if mock.status == "fail" {
                    Some(
                        crate::commands::cli_error::classify(&anyhow::anyhow!(
                            "eval harness scenario {} failed",
                            mock.id
                        ))
                        .code,
                    )
                } else {
                    None
                },
            });
        }
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
