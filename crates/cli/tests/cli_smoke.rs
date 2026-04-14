//! 命令行冒烟：由 `cargo test -p anycode` / `cargo test --workspace` 执行。
//! Cargo 会注入 `CARGO_BIN_EXE_anycode`（当前为 **debug** 产物，与 `cargo build` 默认一致）。

use std::io::Write;
use std::process::{Command, Stdio};

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

/// 非 TTY 默认入口为行式 REPL；`/help` 中 run 示例须为真换行（Fluent 不会把 `\n` 当转义）。
#[test]
fn line_repl_help_no_literal_backslash_n() {
    let mut child = anycode()
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
