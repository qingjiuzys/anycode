//! Mock-LLM fixture task for the production eval harness (no real API credentials).

use serde::Serialize;
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

pub(crate) fn fixture_repo_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/eval/fixtures/minimal-repo")
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

fn json_response(seq: usize, content: &str) -> String {
    format!(
        r#"{{"id":"eval-mock-{seq}","choices":[{{"message":{{"role":"assistant","content":{content_json}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}}}"#,
        seq = seq,
        content_json = serde_json::to_string(content).expect("json string")
    )
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

fn start_mock_server(client_done: Arc<AtomicBool>) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    listener.set_nonblocking(true).expect("nonblocking");
    let port = listener.local_addr().expect("addr").port();
    let seq = Arc::new(AtomicUsize::new(0));
    let seq_c = Arc::clone(&seq);
    let done_c = Arc::clone(&client_done);
    let handle = thread::spawn(move || {
        let mut handled = 0usize;
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if Instant::now() > deadline {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = read_one_http_request(&mut stream);
                    let n = seq_c.fetch_add(1, Ordering::SeqCst);
                    let body = json_response(n, &format!("MOCK_EVAL_{n}"));
                    let _ = write_http_json(&mut stream, &body);
                    handled += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if done_c.load(Ordering::Acquire) && handled >= 1 {
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

pub(crate) fn run_mock_fixture_task(bin: &Path) -> MockEvalRow {
    let fixture = fixture_repo_path();
    if !fixture.is_dir() {
        return MockEvalRow {
            id: "mock-fixture-run",
            status: "skip",
            detail: format!("fixture repo missing at {}", fixture.display()),
            exit_code: 0,
        };
    }

    let home = std::env::temp_dir().join(format!("anycode-eval-mock-{}", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::create_dir_all(&home) {
        return MockEvalRow {
            id: "mock-fixture-run",
            status: "fail",
            detail: format!("temp home: {e}"),
            exit_code: -1,
        };
    }

    let done = Arc::new(AtomicBool::new(false));
    let (port, mock_join) = start_mock_server(Arc::clone(&done));
    if let Err(e) = write_min_config(&home, port) {
        let _ = std::fs::remove_dir_all(&home);
        return MockEvalRow {
            id: "mock-fixture-run",
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
            "Say hello using the fixture repo context",
        ])
        .env("HOME", &home)
        .output();

    done.store(true, Ordering::Release);
    let _ = mock_join.join();
    let _ = std::fs::remove_dir_all(&home);

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return MockEvalRow {
                id: "mock-fixture-run",
                status: "fail",
                detail: format!("spawn failed: {e}"),
                exit_code: -1,
            };
        }
    };

    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    let pass = output.status.success() && combined.contains("MOCK_EVAL_0");
    MockEvalRow {
        id: "mock-fixture-run",
        status: if pass { "pass" } else { "fail" },
        detail: if pass {
            "mock LLM fixture repo task completed with MOCK_EVAL_0 in output".into()
        } else {
            format!(
                "expected MOCK_EVAL_0; exit={}; tail={}",
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
    }
}
