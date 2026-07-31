//! Live Chromium smoke test (requires ANYCODE_CHROMIUM_PATH or ANYCODE_BROWSER_MCP_ROOT).

use anycode_browser::BrowserService;

#[tokio::test]
async fn navigate_example_com_and_snapshot() {
    if std::env::var("ANYCODE_SKIP_BROWSER_SMOKE").is_ok() {
        return;
    }
    if anycode_browser::resolve_chromium_executable().is_none() {
        eprintln!(
            "SKIP: Chromium not found — set ANYCODE_CHROMIUM_PATH or ANYCODE_BROWSER_MCP_ROOT"
        );
        return;
    }

    let svc = BrowserService::new();
    let session_key = format!("smoke-{}", uuid::Uuid::new_v4());
    let info = match svc
        .create_session("smoke-test", None, Some(&session_key))
        .await
    {
        Ok(info) => info,
        Err(e) => {
            // Binary may exist (system chromium) while launch still fails (no dbus/display).
            eprintln!("SKIP: Chromium launch failed: {e}");
            return;
        }
    };
    let sid = info.session_id;

    let state = svc
        .navigate_user(&sid, "https://example.com")
        .await
        .expect("navigate");
    assert!(
        state.url.contains("example.com"),
        "unexpected url: {}",
        state.url
    );

    let snap = svc.snapshot(&sid, None).await.expect("snapshot");
    assert!(
        snap.yaml.contains("Example Domain") || snap.title.contains("Example"),
        "snapshot missing title; title={} yaml_len={}",
        snap.title,
        snap.yaml.len()
    );

    let shot = svc.screenshot(&sid).await.expect("screenshot");
    assert!(
        !shot.image_base64.is_empty(),
        "screenshot should return base64 png"
    );

    svc.close_session(&sid).await.expect("close");
}
