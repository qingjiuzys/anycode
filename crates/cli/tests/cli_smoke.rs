//! 命令行冒烟：由 `cargo test -p anycode` / `cargo test --workspace` 执行。
//! Cargo 会注入 `CARGO_BIN_EXE_anycode`（当前为 **debug** 产物，与 `cargo build` 默认一致）。

use std::io::Write;
use std::process::{Command, Stdio};

fn anycode() -> Command {
    Command::new(env!("CARGO_BIN_EXE_anycode"))
}

/// Isolated `$HOME` so REPL smoke tests do not open the user's `~/.anycode/memory.sled`
/// (e.g. when a WeChat bridge with `memory.backend=hybrid` is running).
fn anycode_with_isolated_home() -> (tempfile::TempDir, Command) {
    let home = tempfile::TempDir::new().expect("temp home");
    let mut cmd = anycode();
    cmd.env("HOME", home.path());
    (home, cmd)
}

#[test]
fn help_prints_usage() {
    let out = anycode()
        .arg("--help")
        .output()
        .expect("spawn anycode --help");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage:"), "stdout={stdout}");
    assert!(
        stdout.contains("anyCode") || stdout.contains("anycode"),
        "stdout={stdout}"
    );
}

#[test]
fn version_flag_exits_zero() {
    let out = anycode()
        .arg("--version")
        .output()
        .expect("spawn anycode --version");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("anycode") && stdout.contains('.'),
        "stdout={stdout}"
    );
}

#[test]
fn status_exits_zero() {
    let out = anycode()
        .arg("status")
        .output()
        .expect("spawn anycode status");
    assert!(
        out.status.success(),
        "status failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("provider:") || combined.contains("anyCode"),
        "unexpected output: {combined}"
    );
}

fn run_json_subcommand(args: &[&str]) -> serde_json::Value {
    let (_home, mut cmd) = anycode_with_isolated_home();
    let out = cmd
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn anycode {args:?}: {e}"));
    assert!(
        out.status.success(),
        "command {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "command {args:?} did not print JSON: {e}; stdout={}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

#[test]
fn production_eval_list_json_exits_zero() {
    let v = run_json_subcommand(&["eval", "list", "--json"]);
    let rows = v
        .get("cli")
        .and_then(|c| c.as_array())
        .expect("eval list cli array");
    assert!(
        rows.iter()
            .any(|row| row.get("id").and_then(|v| v.as_str()) == Some("doctor-all")),
        "expected doctor-all scenario: {v}"
    );
    let mock_rows = v
        .get("mock_fixtures")
        .and_then(|c| c.as_array())
        .expect("eval list mock_fixtures array");
    assert!(
        mock_rows
            .iter()
            .any(|row| row.get("id").and_then(|v| v.as_str()) == Some("mock-fixture-bugfix")),
        "expected mock-fixture-bugfix: {v}"
    );
}

#[test]
fn mcp_status_json_exits_zero() {
    let v = run_json_subcommand(&["mcp", "status", "--json"]);
    let rows = v.as_array().expect("mcp status rows");
    assert!(
        rows.iter()
            .any(|row| { row.get("name").and_then(|v| v.as_str()) == Some("policy.reconnect") }),
        "expected MCP reconnect policy row: {v}"
    );
}

#[test]
fn doctor_errors_taxonomy_json_exits_zero() {
    let v = run_json_subcommand(&["doctor", "errors", "--json"]);
    let rows = v.as_array().expect("doctor errors taxonomy");
    assert!(
        rows.iter().any(|row| {
            row.get("code").and_then(|v| v.as_str()) == Some("eval.scenario_failed")
        }),
        "expected eval.scenario_failed taxonomy row: {v}"
    );
}

#[test]
fn doctor_all_json_exits_zero() {
    let v = run_json_subcommand(&["doctor", "all", "--json"]);
    let rows = v.as_array().expect("doctor rows array");
    assert!(
        rows.iter()
            .any(|row| row.get("name").and_then(|v| v.as_str()) == Some("memory.backend")),
        "expected memory backend diagnostic: {v}"
    );
}

#[test]
fn audit_tail_json_empty_log_exits_zero() {
    let v = run_json_subcommand(&["audit", "tail", "--limit", "1", "--json"]);
    assert_eq!(
        v.as_array().map(Vec::len),
        Some(0),
        "expected empty audit log: {v}"
    );
}

#[test]
fn cron_runs_session_filter_matches_ledger() {
    let (home, mut cmd) = anycode_with_isolated_home();
    let log_dir = home.path().join(".anycode/logs");
    std::fs::create_dir_all(&log_dir).expect("log dir");
    let ledger = log_dir.join("cron-runs.jsonl");
    std::fs::write(
        &ledger,
        r#"{"job_id":"j1","session_id":"sess-a","fired_at":"2026-05-20T00:00:00Z","status":"ok","detail":""}
{"job_id":"j2","session_id":"sess-b","fired_at":"2026-05-20T00:01:00Z","status":"error","detail":"boom"}
"#,
    )
    .expect("ledger");
    cmd.args(["cron", "runs", "--session", "sess-a", "--json"]);
    let out = cmd.output().expect("cron runs");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("json array from cron runs");
    let rows = v.as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]
            .get("raw")
            .and_then(|r| r.get("job_id"))
            .and_then(|v| v.as_str()),
        Some("j1")
    );
}

#[test]
fn cron_runs_json_empty_ledger_exits_zero() {
    let v = run_json_subcommand(&["cron", "runs", "--limit", "1", "--json"]);
    assert_eq!(
        v.as_array().map(Vec::len),
        Some(0),
        "expected empty ledger: {v}"
    );
}

#[test]
fn memory_doctor_json_exits_zero() {
    let v = run_json_subcommand(&["memory", "doctor", "--json"]);
    let rows = v.as_array().expect("memory doctor rows");
    assert!(
        rows.iter()
            .any(|row| row.get("name").and_then(|v| v.as_str()) == Some("memory.path")),
        "expected memory path diagnostic: {v}"
    );
}

#[test]
fn channel_status_json_exits_zero() {
    let v = run_json_subcommand(&["channel", "status", "--json"]);
    let rows = v.as_array().expect("channel status rows");
    assert!(
        rows.iter().any(|row| row
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(|name| name.starts_with("channel."))),
        "expected channel diagnostics: {v}"
    );
}

/// 非 TTY 默认入口为行式 REPL；`/help` 中 run 示例须为真换行（Fluent 不会把 `\n` 当转义）。
#[test]
fn line_repl_help_no_literal_backslash_n() {
    let (_home, mut cmd) = anycode_with_isolated_home();
    let mut child = cmd
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn anycode (non-TTY line REPL)");
    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(stdin, "/help").unwrap();
    writeln!(stdin, "/exit").unwrap();
    drop(stdin);
    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("\\n"),
        "help must not show literal \\\\n in text: {stdout}"
    );
    assert!(
        stdout.contains("anycode run"),
        "expected run hint in help: {stdout}"
    );
}
