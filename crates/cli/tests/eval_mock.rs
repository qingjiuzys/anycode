//! Eval harness mock-LLM fixture scenario (production `eval run --mock`).

use std::path::PathBuf;
use std::process::Command;

fn anycode_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_anycode"))
}

#[test]
fn eval_run_mock_fixture_passes() {
    let out = Command::new(anycode_bin())
        .env("ANYCODE_DASHBOARD_RECORD", "0")
        .args(["eval", "run", "--mock", "--json"])
        .output()
        .expect("spawn eval run --mock");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let rows: Vec<serde_json::Value> = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("invalid mock eval JSON: {e}\nstdout={stdout}"));
    for id in [
        "mock-fixture-greet",
        "mock-fixture-bugfix",
        "mock-fixture-multifile",
        "mock-fixture-test-repair",
    ] {
        let row = rows
            .iter()
            .find(|r| r.get("id").and_then(|x| x.as_str()) == Some(id))
            .unwrap_or_else(|| panic!("missing row {id} in {stdout}"));
        assert_eq!(
            row.get("status").and_then(|x| x.as_str()),
            Some("pass"),
            "row {id}: {row:?}"
        );
        let detail = row.get("detail").and_then(|x| x.as_str()).unwrap_or("");
        assert!(
            detail.contains("trajectory ok"),
            "expected trajectory ok in {id}: {detail}"
        );
    }
    for id in ["mock-fixture-budget-trip"] {
        let row = rows
            .iter()
            .find(|r| r.get("id").and_then(|x| x.as_str()) == Some(id))
            .unwrap_or_else(|| panic!("missing budget row {id} in {stdout}"));
        assert_eq!(
            row.get("status").and_then(|x| x.as_str()),
            Some("pass"),
            "budget trip {id}: {row:?}"
        );
        let detail = row.get("detail").and_then(|x| x.as_str()).unwrap_or("");
        assert!(
            detail.contains("trajectory ok"),
            "expected trajectory ok in {id}: {detail}"
        );
    }
    for id in [
        "mock-trajectory-guard-excess-tools",
        "mock-trajectory-guard-forbidden-tool",
        "mock-trajectory-guard-missing-event",
    ] {
        let row = rows
            .iter()
            .find(|r| r.get("id").and_then(|x| x.as_str()) == Some(id))
            .unwrap_or_else(|| panic!("missing guard row {id} in {stdout}"));
        assert_eq!(
            row.get("status").and_then(|x| x.as_str()),
            Some("pass"),
            "guard {id}: {row:?}"
        );
        let detail = row.get("detail").and_then(|x| x.as_str()).unwrap_or("");
        assert!(
            detail.contains("guard ok"),
            "expected guard ok in {id}: {detail}"
        );
    }
}
