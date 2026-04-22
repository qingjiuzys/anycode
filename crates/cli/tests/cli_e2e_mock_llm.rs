//! 端到端 CLI 测试：本地 TCP mock 返回 OpenAI 兼容 `chat/completions` JSON，无需真实 API key。
//! 覆盖 `anycode run`、带 `--workflow` 的多步任务、非 TTY 行式 REPL 两轮与三轮自然语言输入。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

fn anycode_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_anycode"))
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

/// 读完一次 HTTP 请求（含 body），避免 keep-alive / 大块 tool schema 截断。
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
        r#"{{"id":"mock-{seq}","choices":[{{"message":{{"role":"assistant","content":{content_json}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}}}"#,
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

/// 子进程结束后置位 `client_done`，避免 mock 线程在 `accept` 上永久阻塞导致 `join` 死锁。
fn start_mock_server(
    client_done: Arc<AtomicBool>,
    min_requests: usize,
    label: &'static str,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    listener.set_nonblocking(true).expect("nonblocking");
    let port = listener.local_addr().expect("addr").port();
    let seq = Arc::new(AtomicUsize::new(0));
    let seq_c = Arc::clone(&seq);
    let done_c = Arc::clone(&client_done);
    let handle = thread::spawn(move || {
        let mut handled = 0usize;
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if Instant::now() > deadline {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = read_one_http_request(&mut stream);
                    let n = seq_c.fetch_add(1, Ordering::SeqCst);
                    let body = json_response(n, &format!("MOCK_{label}_{n}"));
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

fn write_min_config(dir: &std::path::Path, port: u16) {
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
    std::fs::write(dir.join("config.json"), cfg).expect("write config");
}

#[test]
fn e2e_run_single_task_mock_llm() {
    let tmp = TempDir::new().expect("tempdir");
    let wd = tmp.path().join("wd");
    std::fs::create_dir_all(&wd).unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let (port, mock_join) = start_mock_server(Arc::clone(&done), 1, "run");
    write_min_config(tmp.path(), port);

    let out = Command::new(anycode_bin())
        .args([
            "-c",
            tmp.path().join("config.json").to_str().unwrap(),
            "--ignore",
            "run",
            "-C",
            wd.to_str().unwrap(),
            "--agent",
            "general-purpose",
            "hello mock",
        ])
        .output()
        .expect("spawn run");

    done.store(true, Ordering::Release);
    mock_join.join().expect("mock join");

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
        combined.contains("MOCK_run_0"),
        "expected mock reply in output: {combined}"
    );
}

#[test]
fn e2e_run_workflow_two_steps_mock_llm() {
    let tmp = TempDir::new().expect("tempdir");
    let wd = tmp.path().join("wd");
    std::fs::create_dir_all(&wd).unwrap();
    let wf = wd.join("wf.yaml");
    std::fs::write(
        &wf,
        r#"name: e2e-wf
steps:
  - id: s1
    prompt: First step for mock
  - id: s2
    prompt: Second step for mock
"#,
    )
    .unwrap();

    let done = Arc::new(AtomicBool::new(false));
    let (port, mock_join) = start_mock_server(Arc::clone(&done), 2, "wf");
    write_min_config(tmp.path(), port);

    let out = Command::new(anycode_bin())
        .args([
            "-c",
            tmp.path().join("config.json").to_str().unwrap(),
            "--ignore",
            "run",
            "-C",
            wd.to_str().unwrap(),
            "--workflow",
            wf.to_str().unwrap(),
            "--agent",
            "general-purpose",
            "user ctx",
        ])
        .output()
        .expect("spawn workflow");

    done.store(true, Ordering::Release);
    mock_join.join().expect("mock join");

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
        combined.contains("workflow: e2e-wf"),
        "expected workflow banner: {combined}"
    );
    let wf_mocks = combined.matches("MOCK_wf_").count();
    assert!(
        wf_mocks >= 2,
        "expected at least two MOCK_wf_* replies; got {wf_mocks} hits: {combined}"
    );
}

#[test]
fn e2e_line_repl_two_natural_turns_mock_llm() {
    let tmp = TempDir::new().expect("tempdir");
    let wd = tmp.path().join("wd");
    std::fs::create_dir_all(&wd).unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let (port, mock_join) = start_mock_server(Arc::clone(&done), 2, "repl");
    write_min_config(tmp.path(), port);

    let mut child = Command::new(anycode_bin())
        .args([
            "-c",
            tmp.path().join("config.json").to_str().unwrap(),
            "--ignore",
            "-C",
            wd.to_str().unwrap(),
            "--agent",
            "general-purpose",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn repl");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");

    let drain_out = thread::spawn(move || {
        let mut s = String::new();
        std::io::BufReader::new(stdout).read_to_string(&mut s).ok();
        s
    });
    let drain_err = thread::spawn(move || {
        let mut s = String::new();
        std::io::BufReader::new(stderr).read_to_string(&mut s).ok();
        s
    });

    writeln!(stdin, "first line to mock").unwrap();
    thread::sleep(Duration::from_millis(1500));
    writeln!(stdin, "second line to mock").unwrap();
    thread::sleep(Duration::from_millis(1500));
    writeln!(stdin, "/exit").unwrap();
    drop(stdin);

    let status = child.wait().expect("wait");
    done.store(true, Ordering::Release);
    mock_join.join().expect("mock join");

    let out = drain_out.join().expect("out join");
    let err = drain_err.join().expect("err join");

    assert!(status.success(), "stderr combined={out}{err}");
    // 启动时可能多一次探测/会话请求，序号不一定从 0 起；只要求 stdout 里出现两次及以上 mock 回复。
    let mock_lines = out.matches("MOCK_repl_").count();
    assert!(
        mock_lines >= 2,
        "expected at least two MOCK_repl_* lines in stdout; got {mock_lines} hits; out:\n{out}\nstderr:\n{err}"
    );
}

/// 三轮自然语言 + `/exit`，压测行式 REPL 循环与 mock 序列为 0..3。
#[test]
fn e2e_line_repl_three_natural_turns_mock_llm() {
    let tmp = TempDir::new().expect("tempdir");
    let wd = tmp.path().join("wd");
    std::fs::create_dir_all(&wd).unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let (port, mock_join) = start_mock_server(Arc::clone(&done), 3, "repl3");
    write_min_config(tmp.path(), port);

    let mut child = Command::new(anycode_bin())
        .args([
            "-c",
            tmp.path().join("config.json").to_str().unwrap(),
            "--ignore",
            "-C",
            wd.to_str().unwrap(),
            "--agent",
            "general-purpose",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn repl");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");

    let drain_out = thread::spawn(move || {
        let mut s = String::new();
        std::io::BufReader::new(stdout).read_to_string(&mut s).ok();
        s
    });
    let drain_err = thread::spawn(move || {
        let mut s = String::new();
        std::io::BufReader::new(stderr).read_to_string(&mut s).ok();
        s
    });

    for line in ["turn one", "turn two", "turn three"] {
        writeln!(stdin, "{line}").unwrap();
        thread::sleep(Duration::from_millis(1500));
    }
    writeln!(stdin, "/exit").unwrap();
    drop(stdin);

    let status = child.wait().expect("wait");
    done.store(true, Ordering::Release);
    mock_join.join().expect("mock join");

    let out = drain_out.join().expect("out join");
    let err = drain_err.join().expect("err join");

    assert!(status.success(), "stderr combined={out}{err}");
    let mock_lines = out.matches("MOCK_repl3_").count();
    assert!(
        mock_lines >= 3,
        "expected at least three MOCK_repl3_* lines in stdout; got {mock_lines} hits; out:\n{out}\nstderr:\n{err}"
    );
}
