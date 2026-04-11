//! 命令行冒烟：由 `cargo test -p anycode` / `cargo test --workspace` 执行。
//! Cargo 会注入 `CARGO_BIN_EXE_anycode`（当前为 **debug** 产物，与 `cargo build` 默认一致）。

use std::process::Command;

fn anycode() -> Command {
    Command::new(env!("CARGO_BIN_EXE_anycode"))
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
