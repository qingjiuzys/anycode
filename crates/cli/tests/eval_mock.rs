//! Eval harness mock-LLM fixture scenario (production `eval run --mock`).

use std::path::PathBuf;
use std::process::Command;

fn anycode_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_anycode"))
}

#[test]
fn eval_run_mock_fixture_passes() {
    let out = Command::new(anycode_bin())
        .args(["eval", "run", "--mock", "--json"])
        .output()
        .expect("spawn eval run --mock");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("mock-fixture-run") && combined.contains("MOCK_EVAL"),
        "expected mock fixture row: {combined}"
    );
}
