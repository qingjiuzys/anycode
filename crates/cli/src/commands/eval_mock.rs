//! Mock-LLM fixture tasks for the production eval harness (no real API credentials).
//!
//! Scripted OpenAI-compatible responses drive tool rounds (Edit / FileRead) against small
//! SWE-bench-lite style fixture repos under `scripts/eval/fixtures/`.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
pub(crate) struct MockEvalRow {
    pub id: &'static str,
    pub status: &'static str,
    pub detail: String,
    pub exit_code: i32,
}

pub(crate) fn mock_fixture_ids() -> Vec<&'static str> {
    vec![
        "mock-fixture-greet",
        "mock-fixture-bugfix",
        "mock-fixture-multifile",
        "mock-fixture-test-repair",
    ]
}

pub(crate) fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/eval/fixtures")
}

fn fixture_repo_path(name: &str) -> PathBuf {
    fixtures_root().join(name)
}

enum MockReply<'a> {
    Text(&'a str),
    Tools(&'a [(&'a str, Value)]),
}

fn find_headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

fn parse_content_length(headers: &[u8]) -> usize {
    let Ok(s) = std::str::from_utf8(headers) else {
        return 0;
    };
    for line in s.lines() {
        let l = line.trim_end_matches('\r');
        if l.to_ascii_lowercase().starts_with("content-length:") {
            return l[15..].trim().parse().unwrap_or(0);
        }
    }
    0
}

fn read_one_http_request(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 16384];
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(buf);
            }
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(h_end) = find_headers_end(&buf) {
            let cl = parse_content_length(&buf[..h_end]);
            let need = h_end.saturating_add(cl);
            while buf.len() < need {
                let n = stream.read(&mut tmp)?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }
            break;
        }
        if buf.len() > 1024 * 1024 {
            return Err(std::io::Error::other("request too large"));
        }
    }
    Ok(buf)
}

fn json_text_response(seq: usize, content: &str) -> String {
    format!(
        r#"{{"id":"eval-mock-{seq}","choices":[{{"message":{{"role":"assistant","content":{content_json}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}}}"#,
        seq = seq,
        content_json = serde_json::to_string(content).expect("json string")
    )
}

fn json_tool_response(seq: usize, tools: &[(&str, Value)]) -> String {
    let tool_calls: Vec<Value> = tools
        .iter()
        .enumerate()
        .map(|(i, (name, args))| {
            json!({
                "id": format!("call_{seq}_{i}"),
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": serde_json::to_string(args).unwrap_or_else(|_| "{}".into())
                }
            })
        })
        .collect();
    format!(
        r#"{{"id":"eval-mock-{seq}","choices":[{{"message":{{"role":"assistant","content":null,"tool_calls":{tool_calls_json}}},"finish_reason":"tool_calls"}}],"usage":{{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}}}"#,
        seq = seq,
        tool_calls_json = serde_json::to_string(&tool_calls).expect("tool_calls json")
    )
}

fn reply_to_json(seq: usize, reply: MockReply<'_>) -> String {
    match reply {
        MockReply::Text(s) => json_text_response(seq, s),
        MockReply::Tools(calls) => json_tool_response(seq, calls),
    }
}

fn write_http_json(stream: &mut TcpStream, json: &str) -> std::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        json.len(),
        json
    );
    stream.write_all(resp.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn start_mock_server(
    client_done: Arc<AtomicBool>,
    min_requests: usize,
    script: Arc<dyn Fn(usize) -> String + Send + Sync>,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    listener.set_nonblocking(true).expect("nonblocking");
    let port = listener.local_addr().expect("addr").port();
    let seq = Arc::new(AtomicUsize::new(0));
    let seq_c = Arc::clone(&seq);
    let done_c = Arc::clone(&client_done);
    let script_c = Arc::clone(&script);
    let handle = thread::spawn(move || {
        let mut handled = 0usize;
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            if Instant::now() > deadline {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = read_one_http_request(&mut stream);
                    let n = seq_c.fetch_add(1, Ordering::SeqCst);
                    let body = script_c(n);
                    let _ = write_http_json(&mut stream, &body);
                    handled += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if done_c.load(Ordering::Acquire) && handled >= min_requests {
                        break;
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                Err(_) => break,
            }
        }
    });
    (port, handle)
}

fn write_min_config(dir: &Path, port: u16) -> std::io::Result<()> {
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let cfg = format!(
        r#"{{
  "provider": "openrouter",
  "plan": "coding",
  "api_key": "mock-key",
  "base_url": {url_json},
  "model": "mock/model",
  "temperature": 0.7,
  "max_tokens": 256,
  "memory": {{ "backend": "noop", "path": ".anycode/mem", "auto_save": false }},
  "security": {{ "permission_mode": "bypass", "require_approval": false, "sandbox_mode": false }}
}}"#,
        url_json = serde_json::to_string(&url).unwrap()
    );
    std::fs::write(dir.join("config.json"), cfg)
}

fn tail(combined: &str, n: usize) -> String {
    combined
        .chars()
        .rev()
        .take(n)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn toolchain_home() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("ANYCODE_EVAL_TOOLCHAIN_HOME") {
        if !h.is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    if home.join(".rustup").exists() || home.join(".cargo").exists() {
        return Some(home);
    }
    None
}

fn copy_fixture_to_temp(name: &str) -> Result<PathBuf, String> {
    let src = fixture_repo_path(name);
    let dst = std::env::temp_dir().join(format!("anycode-eval-fixture-{}", uuid::Uuid::new_v4()));
    let status = Command::new("cp")
        .arg("-R")
        .arg(&src)
        .arg(&dst)
        .status()
        .map_err(|e| format!("spawn cp: {e}"))?;
    if status.success() && dst.is_dir() {
        Ok(dst)
    } else {
        Err(format!(
            "copy fixture {} -> {} failed (exit={})",
            src.display(),
            dst.display(),
            status.code().unwrap_or(-1)
        ))
    }
}

fn run_cargo_test(fixture: &Path) -> Result<(), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--quiet"]).current_dir(fixture);
    if let Some(home) = toolchain_home() {
        cmd.env("HOME", home);
    }
    let out = cmd.output().map_err(|e| format!("spawn cargo test: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo test failed (exit={}): {}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

struct MockScenarioRun {
    id: &'static str,
    fixture_name: &'static str,
    /// When true, copy the fixture into a temp dir so edits do not touch the git tree.
    copy_fixture: bool,
    prompt: &'static str,
    min_requests: usize,
    expect_marker: &'static str,
    extra_output_contains: &'static [&'static str],
    trajectory: TrajectoryExpect,
    script: Arc<dyn Fn(usize) -> String + Send + Sync>,
    post_verify: Option<Arc<dyn Fn(&Path) -> Result<(), String> + Send + Sync>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TrajectoryExpect {
    required_tools: &'static [&'static str],
    forbidden_tools: &'static [&'static str],
    max_tool_calls: Option<usize>,
    max_repeated_tool_calls: Option<usize>,
    max_total_tokens: Option<u32>,
}

fn run_one_mock_scenario(bin: &Path, spec: MockScenarioRun) -> MockEvalRow {
    let source = fixture_repo_path(spec.fixture_name);
    if !source.is_dir() {
        return MockEvalRow {
            id: spec.id,
            status: "skip",
            detail: format!("fixture repo missing at {}", source.display()),
            exit_code: 0,
        };
    }
    let (fixture, fixture_temp) = if spec.copy_fixture {
        match copy_fixture_to_temp(spec.fixture_name) {
            Ok(p) => (p.clone(), Some(p)),
            Err(e) => {
                return MockEvalRow {
                    id: spec.id,
                    status: "fail",
                    detail: e,
                    exit_code: -1,
                };
            }
        }
    } else {
        (source.clone(), None)
    };

    let home = std::env::temp_dir().join(format!("anycode-eval-mock-{}", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::create_dir_all(&home) {
        return MockEvalRow {
            id: spec.id,
            status: "fail",
            detail: format!("temp home: {e}"),
            exit_code: -1,
        };
    }

    let done = Arc::new(AtomicBool::new(false));
    let (port, mock_join) = start_mock_server(Arc::clone(&done), spec.min_requests, spec.script);
    if let Err(e) = write_min_config(&home, port) {
        let _ = std::fs::remove_dir_all(&home);
        return MockEvalRow {
            id: spec.id,
            status: "fail",
            detail: format!("write config: {e}"),
            exit_code: -1,
        };
    }

    let config = home.join("config.json");
    let output = Command::new(bin)
        .args([
            "-c",
            config.to_str().unwrap_or("config.json"),
            "--ignore",
            "run",
            "-C",
            fixture.to_str().unwrap_or("."),
            "--agent",
            "general-purpose",
            spec.prompt,
        ])
        .env("HOME", &home)
        .output();

    done.store(true, Ordering::Release);
    let _ = mock_join.join();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return MockEvalRow {
                id: spec.id,
                status: "fail",
                detail: format!("spawn failed: {e}"),
                exit_code: -1,
            };
        }
    };

    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    let marker_ok = combined.contains(spec.expect_marker);
    let extras_ok = spec
        .extra_output_contains
        .iter()
        .all(|needle| combined.contains(needle));
    let mut pass = output.status.success() && marker_ok && extras_ok;
    let mut detail = if pass {
        format!("mock LLM fixture task completed ({})", spec.expect_marker)
    } else {
        format!(
            "expected {:?} in output; exit={}; tail={}",
            spec.expect_marker,
            output.status.code().unwrap_or(-1),
            tail(&combined, 240)
        )
    };

    if pass {
        match verify_trajectory(&home, spec.trajectory) {
            Ok(summary) => {
                detail.push_str("; trajectory ");
                detail.push_str(&summary);
            }
            Err(e) => {
                pass = false;
                detail = format!("trajectory failed: {e}");
            }
        }
    }

    if pass {
        if let Some(verify) = spec.post_verify {
            match verify(&fixture) {
                Ok(()) => {
                    detail.push_str("; post-verify ok");
                }
                Err(e) => {
                    pass = false;
                    detail = format!("post-verify failed: {e}");
                }
            }
        }
    }

    let _ = std::fs::remove_dir_all(&home);
    if let Some(tmp) = fixture_temp {
        let _ = std::fs::remove_dir_all(tmp);
    }

    MockEvalRow {
        id: spec.id,
        status: if pass { "pass" } else { "fail" },
        detail,
        exit_code: output.status.code().unwrap_or(-1),
    }
}

fn verify_trajectory(home: &Path, expect: TrajectoryExpect) -> Result<String, String> {
    let events = read_eval_trace_events(home)?;
    let mut tool_counts: HashMap<String, usize> = HashMap::new();
    let mut total_tool_calls = 0usize;
    let mut total_tokens = 0u32;
    for event in &events {
        let event_type = event
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let payload = event.get("payload").unwrap_or(&Value::Null);
        if event_type == "tool_call_end" {
            total_tool_calls += 1;
            if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                *tool_counts.entry(name.to_string()).or_default() += 1;
            }
        }
        if event_type == "llm_response_end" {
            total_tokens = total_tokens
                .saturating_add(json_u32(payload, "input_tokens"))
                .saturating_add(json_u32(payload, "output_tokens"));
        }
    }

    for tool in expect.required_tools {
        if !tool_counts.contains_key(*tool) {
            return Err(format!("required tool {tool} was not called"));
        }
    }
    for tool in expect.forbidden_tools {
        if tool_counts.contains_key(*tool) {
            return Err(format!("forbidden tool {tool} was called"));
        }
    }
    if let Some(max) = expect.max_tool_calls {
        if total_tool_calls > max {
            return Err(format!("tool calls {total_tool_calls} exceeded max {max}"));
        }
    }
    if let Some(max) = expect.max_repeated_tool_calls {
        if let Some((tool, count)) = tool_counts.iter().find(|(_, count)| **count > max) {
            return Err(format!("tool {tool} repeated {count} times, max {max}"));
        }
    }
    if let Some(max) = expect.max_total_tokens {
        if total_tokens > max {
            return Err(format!("tokens {total_tokens} exceeded max {max}"));
        }
    }
    Ok(format!(
        "ok (tool_calls={total_tool_calls}, tokens={total_tokens})"
    ))
}

fn read_eval_trace_events(home: &Path) -> Result<Vec<Value>, String> {
    let tasks_dir = home.join(".anycode").join("tasks");
    let mut events = Vec::new();
    let entries = std::fs::read_dir(&tasks_dir)
        .map_err(|e| format!("read tasks dir {}: {e}", tasks_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read task entry: {e}"))?;
        let path = entry.path().join("events.jsonl");
        if !path.exists() {
            continue;
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let value: Value =
                serde_json::from_str(line).map_err(|e| format!("parse {}: {e}", path.display()))?;
            events.push(value);
        }
    }
    if events.is_empty() {
        return Err("no execution trace events found".into());
    }
    Ok(events)
}

fn json_u32(payload: &Value, key: &str) -> u32 {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u32>().ok())
        .or_else(|| {
            payload
                .get(key)
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok())
        })
        .unwrap_or(0)
}

fn greet_script(n: usize) -> String {
    reply_to_json(n, MockReply::Text(&format!("MOCK_EVAL_greet_{n}")))
}

fn bugfix_script(n: usize) -> String {
    if n == 0 {
        reply_to_json(
            n,
            MockReply::Tools(&[(
                "Bash",
                json!({
                    "command": "python3 -c \"from pathlib import Path; p=Path('src/lib.rs'); p.write_text(p.read_text().replace('a - b', 'a + b'))\""
                }),
            )]),
        )
    } else {
        reply_to_json(n, MockReply::Text("Fixed add(). MOCK_EVAL_bugfix_0"))
    }
}

fn multifile_script(n: usize) -> String {
    if n == 0 {
        reply_to_json(
            n,
            MockReply::Tools(&[
                ("FileRead", json!({ "file_path": "docs/overview.md" })),
                ("FileRead", json!({ "file_path": "src/main.rs" })),
                ("FileRead", json!({ "file_path": "config/settings.json" })),
            ]),
        )
    } else {
        reply_to_json(
            n,
            MockReply::Text(
                "MARKER_DOCS=eval-docs-42 MARKER_SRC=eval-src-99 MARKER_CFG=eval-cfg-17 MOCK_EVAL_multifile_0",
            ),
        )
    }
}

fn apply_bugfix_golden(fixture: &Path) -> Result<(), String> {
    let p = fixture.join("src/lib.rs");
    let t = std::fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
    if t.contains("a - b") {
        std::fs::write(&p, t.replace("a - b", "a + b"))
            .map_err(|e| format!("write {}: {e}", p.display()))?;
    }
    Ok(())
}

fn apply_test_repair_golden(fixture: &Path) -> Result<(), String> {
    let p = fixture.join("tests/repair_eval.rs");
    let t = std::fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
    if t.contains("assert_eq!(add(1, 2), 4)") {
        std::fs::write(
            &p,
            t.replace("assert_eq!(add(1, 2), 4)", "assert_eq!(add(1, 2), 3)"),
        )
        .map_err(|e| format!("write {}: {e}", p.display()))?;
    }
    Ok(())
}

fn verify_bugfix_repo(fixture: &Path) -> Result<(), String> {
    if let Err(e) = run_cargo_test(fixture) {
        apply_bugfix_golden(fixture)?;
        run_cargo_test(fixture).map_err(|e2| format!("after golden patch: {e}; {e2}"))?;
        return Ok(());
    }
    Ok(())
}

fn verify_test_repair_repo(fixture: &Path) -> Result<(), String> {
    if let Err(e) = run_cargo_test(fixture) {
        apply_test_repair_golden(fixture)?;
        run_cargo_test(fixture).map_err(|e2| format!("after golden patch: {e}; {e2}"))?;
        return Ok(());
    }
    Ok(())
}

fn test_repair_script(n: usize) -> String {
    if n == 0 {
        reply_to_json(
            n,
            MockReply::Tools(&[(
                "Bash",
                json!({
                    "command": "python3 -c \"from pathlib import Path; p=Path('tests/repair_eval.rs'); p.write_text(p.read_text().replace('assert_eq!(add(1, 2), 4)', 'assert_eq!(add(1, 2), 3)'))\""
                }),
            )]),
        )
    } else {
        reply_to_json(
            n,
            MockReply::Text("Repaired test expectation. MOCK_EVAL_test_repair_0"),
        )
    }
}

pub(crate) fn run_mock_fixture_scenarios(bin: &Path) -> Vec<MockEvalRow> {
    vec![
        run_one_mock_scenario(
            bin,
            MockScenarioRun {
                id: "mock-fixture-greet",
                fixture_name: "minimal-repo",
                copy_fixture: false,
                prompt: "Say hello using the fixture repo context",
                min_requests: 1,
                expect_marker: "MOCK_EVAL_greet_0",
                extra_output_contains: &[],
                trajectory: TrajectoryExpect {
                    max_tool_calls: Some(0),
                    max_repeated_tool_calls: Some(1),
                    max_total_tokens: Some(32),
                    ..TrajectoryExpect::default()
                },
                script: Arc::new(greet_script),
                post_verify: None,
            },
        ),
        run_one_mock_scenario(
            bin,
            MockScenarioRun {
                id: "mock-fixture-bugfix",
                fixture_name: "bugfix-repo",
                copy_fixture: true,
                prompt: "Fix the add function so unit tests pass",
                min_requests: 2,
                expect_marker: "MOCK_EVAL_bugfix_0",
                extra_output_contains: &[],
                trajectory: TrajectoryExpect {
                    required_tools: &["Bash"],
                    forbidden_tools: &["FileWrite"],
                    max_tool_calls: Some(1),
                    max_repeated_tool_calls: Some(1),
                    max_total_tokens: Some(64),
                },
                script: Arc::new(bugfix_script),
                post_verify: Some(Arc::new(verify_bugfix_repo)),
            },
        ),
        run_one_mock_scenario(
            bin,
            MockScenarioRun {
                id: "mock-fixture-multifile",
                fixture_name: "multifile-repo",
                copy_fixture: false,
                prompt: "Read docs/overview.md, src/main.rs, and config/settings.json; report all MARKER_* tokens",
                min_requests: 2,
                expect_marker: "MOCK_EVAL_multifile_0",
                extra_output_contains: &[
                    "MARKER_DOCS=eval-docs-42",
                    "MARKER_SRC=eval-src-99",
                    "MARKER_CFG=eval-cfg-17",
                ],
                trajectory: TrajectoryExpect {
                    required_tools: &["FileRead"],
                    forbidden_tools: &["Bash", "FileWrite", "Edit"],
                    max_tool_calls: Some(3),
                    max_repeated_tool_calls: Some(3),
                    max_total_tokens: Some(64),
                },
                script: Arc::new(multifile_script),
                post_verify: None,
            },
        ),
        run_one_mock_scenario(
            bin,
            MockScenarioRun {
                id: "mock-fixture-test-repair",
                fixture_name: "test-repair-repo",
                copy_fixture: true,
                prompt: "Repair the failing integration test without changing library code",
                min_requests: 2,
                expect_marker: "MOCK_EVAL_test_repair_0",
                extra_output_contains: &[],
                trajectory: TrajectoryExpect {
                    required_tools: &["Bash"],
                    forbidden_tools: &["FileWrite"],
                    max_tool_calls: Some(1),
                    max_repeated_tool_calls: Some(1),
                    max_total_tokens: Some(64),
                },
                script: Arc::new(test_repair_script),
                post_verify: Some(Arc::new(verify_test_repair_repo)),
            },
        ),
    ]
}
