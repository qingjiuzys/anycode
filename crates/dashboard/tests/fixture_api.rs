//! API integration tests against fixture databases.

use anycode_dashboard::server::{app_for_test, app_for_test_with_host};
use axum::body::Body;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tempfile::tempdir;
use tower::ServiceExt;

async fn get_json(app: axum::Router, path: &str) -> Value {
    let res = app
        .oneshot(
            axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_success(), "GET {path}");
    let body = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

async fn post_json(app: axum::Router, path: &str, body: Value) -> Value {
    let res = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_success(), "POST {path}");
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn put_json(app: axum::Router, path: &str, body: Value) -> Value {
    let res = app
        .oneshot(
            axum::http::Request::builder()
                .method("PUT")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_success(), "PUT {path}");
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn delete_json(app: axum::Router, path: &str) -> Value {
    let res = app
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_success(), "DELETE {path}");
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn patch_json(app: axum::Router, path: &str, body: Value) -> Value {
    let res = app
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_success(), "PATCH {path}");
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn sha256_password_hash(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    format!("sha256${salt}${hex}")
}

#[tokio::test]
async fn non_loopback_login_accepts_hashed_password_and_sets_session_cookie() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("auth.db");
    let password_hash = sha256_password_hash("fixture-salt", "correct-password");
    let db = anycode_dashboard::db::DashboardDb::open(&db_path)
        .await
        .unwrap();
    let updated = sqlx::query("UPDATE users SET password_hash = ? WHERE email = 'local@anycode'")
        .bind(password_hash)
        .execute(db.pool())
        .await
        .unwrap();
    assert_eq!(updated.rows_affected(), 1);
    db.pool().close().await;

    let app = app_for_test_with_host(&db_path, "0.0.0.0").await.unwrap();
    let unauth = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), axum::http::StatusCode::UNAUTHORIZED);

    let login = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": "local@anycode",
                        "password": "correct-password"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let login_status = login.status();
    let login_headers = login.headers().clone();
    let login_body = login.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        login_status,
        axum::http::StatusCode::OK,
        "login body: {}",
        String::from_utf8_lossy(&login_body)
    );
    let cookie = login_headers
        .get(axum::http::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert!(cookie.starts_with("dw_session=sess_"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
}

#[tokio::test]
async fn fixture_api_smoke() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("fixture.db");
    let prefs_path = dir.path().join("dashboard_preferences.json");
    std::env::set_var(
        "ANYCODE_DASHBOARD_PREFERENCES_PATH",
        prefs_path.display().to_string(),
    );
    let app = app_for_test(&db).await.unwrap();

    let health = get_json(app.clone(), "/api/health").await;
    assert_eq!(health["ok"], true);

    let auth_me = get_json(app.clone(), "/api/auth/me").await;
    assert_eq!(auth_me["authenticated"], true);
    assert_eq!(auth_me["user"]["email"], "local@anycode");

    let scan = post_json(app.clone(), "/api/projects/scan", json!({})).await;
    assert_eq!(scan["ok"], true);

    let projects = get_json(app.clone(), "/api/projects").await;
    assert!(projects["projects"].is_array());
    assert!(projects["total"].is_number());
    assert!(projects["limit"].is_number());

    let facets = get_json(app.clone(), "/api/sessions/facets").await;
    assert!(facets["facets"]["status"].is_array());
    assert!(facets["facets"]["kind"].is_array());

    let paged = get_json(app.clone(), "/api/projects?limit=1&offset=0").await;
    assert_eq!(paged["limit"], 1);
    assert!(paged["projects"].as_array().unwrap().len() <= 1);

    let sessions = get_json(app.clone(), "/api/sessions?limit=10").await;
    assert!(sessions["sessions"].is_array());

    let overview = get_json(app.clone(), "/api/overview").await;
    assert!(overview["overview"]["projects_count"].is_number());

    let artifacts = get_json(app.clone(), "/api/artifacts?limit=5").await;
    assert!(artifacts["artifacts"].is_array());

    let bootstrap = get_json(app.clone(), "/api/bootstrap").await;
    assert!(bootstrap["bootstrap"]["next_steps"].is_array());
    assert_eq!(bootstrap["bootstrap"]["workbench_phase"], "v3_week10");
    assert!(bootstrap["bootstrap"]["planning_doc"].is_string());

    let setup = get_json(app.clone(), "/api/setup/status").await;
    assert!(setup["setup"]["ready"].is_boolean());
    assert!(setup["setup"]["steps"].is_array());
    assert!(setup["setup"]["platform"].is_string());

    let setup_ws = post_json(app.clone(), "/api/setup/workspace/ensure", json!({})).await;
    assert_eq!(setup_ws["ok"], true);

    let quick = get_json(app.clone(), "/api/setup/quick-auth").await;
    assert!(quick["presets"].is_array());

    let channels_settings = get_json(app.clone(), "/api/settings/channels").await;
    assert!(channels_settings["channels"]["telegram"]["configured"].is_boolean());
    assert!(channels_settings["channels"]["discord"]["configured"].is_boolean());
    assert!(channels_settings["channels"]["platform"].is_string());
    assert!(channels_settings["channels"]["telegram_start_command"].is_string());

    for (path, body) in [
        (
            "/api/setup/channels/telegram/verify",
            json!({ "bot_token": "not-a-valid-token" }),
        ),
        (
            "/api/setup/channels/telegram/chats",
            json!({ "bot_token": "not-a-valid-token" }),
        ),
        (
            "/api/setup/channels/discord/verify",
            json!({ "bot_token": "not-a-valid-token" }),
        ),
        (
            "/api/setup/channels/discord/test",
            json!({ "bot_token": "not-a-valid-token", "channel_id": "123" }),
        ),
    ] {
        let res = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            axum::http::StatusCode::BAD_REQUEST,
            "POST {path}"
        );
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v["error"].is_string());
    }

    let pending = get_json(app.clone(), "/api/security/approvals/pending?limit=5").await;
    assert!(pending["pending"].is_array());
    assert!(pending["web_enabled"].is_boolean());
    assert!(pending["respond_allowed"].is_boolean());

    let approval_summary = get_json(app.clone(), "/api/security/approvals/summary").await;
    assert!(approval_summary["summary"]["pending_total"].is_number());
    assert!(approval_summary["summary"]["by_session"].is_array());

    let readiness = get_json(app.clone(), "/api/metrics/readiness").await;
    assert!(readiness["readiness"]["status"].is_string());

    let doctor = get_json(app.clone(), "/api/settings/doctor").await;
    assert!(doctor["doctor"]["checks"].is_array());
    assert!(doctor["doctor"]["next_steps"].is_array());
    assert!(doctor["doctor"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"] == "skills_starter_pack"));

    let cron_parse = post_json(
        app.clone(),
        "/api/cron/parse-schedule",
        json!({ "text": "每天8点" }),
    )
    .await;
    assert_eq!(cron_parse["schedule"].as_str().unwrap(), "0 0 8 * * *");

    let facets = get_json(app.clone(), "/api/sessions/facets").await;
    assert!(facets["facets"]["budget_exceeded_7d"].is_number());

    let runtime = get_json(app.clone(), "/api/settings/runtime").await;
    assert_eq!(runtime["runtime"]["auth_mode"], "local_trusted");
    assert!(runtime["runtime"]["db_path"].is_string());

    let catalog = get_json(app.clone(), "/api/settings/model-catalog").await;
    assert!(catalog["providers"].is_array());
    assert!(!catalog["providers"].as_array().unwrap().is_empty());
    assert!(catalog["capabilities"].is_array());
    assert!(catalog["zai_models"].is_array());

    let llm_cfg = get_json(app.clone(), "/api/settings/llm").await;
    assert!(llm_cfg["config_present"].is_boolean());
    assert!(llm_cfg["models"].is_object());

    let models_reg = get_json(app.clone(), "/api/settings/models").await;
    assert!(models_reg["active"].is_object());
    assert!(models_reg["items"].is_array());

    let prefs = get_json(app.clone(), "/api/settings/preferences").await;
    assert!(prefs["preferences"]["active"]["host"].is_string());

    let timeline = get_json(app.clone(), "/api/metrics/timeline?days=7").await;
    assert!(timeline["timeline"]["points"].is_array());

    let usage = get_json(app.clone(), "/api/metrics/usage?days=7").await;
    assert!(usage["usage"]["total_tokens"].is_number());
    assert!(usage["usage"]["estimated_cost_usd"].is_number());
    assert!(usage["by_model"].is_array());

    let saved_hours = get_json(app.clone(), "/api/metrics/kpi/saved-hours?days=7").await;
    assert!(saved_hours["kpi"]["estimated_saved_hours"].is_number());
    assert!(saved_hours["kpi"]["estimated_value_usd"].is_number());

    let export = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/metrics/usage/export?days=7")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(export.status().is_success());
    let csv = export.into_body().collect().await.unwrap().to_bytes();
    let csv = String::from_utf8(csv.to_vec()).unwrap();
    assert!(csv.starts_with("project_id,project_name,"));

    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": std::env::current_dir().unwrap().display().to_string(),
            "name": "fixture-v2"
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap();

    let presets = get_json(
        app.clone(),
        &format!("/api/projects/{project_id}/gates/presets"),
    )
    .await;
    assert!(presets["presets"].is_array());

    let proj_usage = get_json(
        app.clone(),
        &format!("/api/projects/{project_id}/usage?days=7"),
    )
    .await;
    assert!(proj_usage["usage"]["llm_calls"].is_number());
    assert!(proj_usage["by_model"].is_array());

    let gate_run = post_json(
        app.clone(),
        &format!("/api/projects/{project_id}/gates/execute"),
        json!({ "command": "echo GATE_FIXTURE_OK", "name": "echo" }),
    )
    .await;
    assert_eq!(gate_run["result"]["status"], "passed");
    assert!(gate_run["result"]["output_excerpt"]
        .as_str()
        .unwrap()
        .contains("GATE_FIXTURE_OK"));

    let start = post_json(
        app.clone(),
        &format!("/api/projects/{project_id}/conversations/start"),
        json!({
            "title": "Fixture conversation",
            "prompt": "echo fixture start conversation",
            "kind": "run"
        }),
    )
    .await;
    assert!(start["session"]["id"].is_string());
    assert_eq!(start["session"]["title"], "Fixture conversation");
    assert_eq!(start["session"]["status"], "pending");
    assert!(start["chat"]["pid"].is_number());

    let gates = get_json(app.clone(), &format!("/api/projects/{project_id}/gates")).await;
    assert!(gates["gates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["name"].as_str() == Some("echo")));

    let sess = post_json(
        app.clone(),
        "/api/sessions",
        json!({
            "project_id": project_id,
            "kind": "run",
            "title": "fixture-cancel"
        }),
    )
    .await;
    let session_id = sess["session"]["id"].as_str().unwrap();
    let cancel = post_json(
        app.clone(),
        &format!("/api/sessions/{session_id}/cancel"),
        json!({}),
    )
    .await;
    assert_eq!(cancel["ok"], true);

    let sess_usage = get_json(app.clone(), &format!("/api/sessions/{session_id}/usage")).await;
    assert!(sess_usage["usage"]["total_tokens"].is_number());
    assert!(sess_usage["by_model"].is_array());

    let gh_conn = post_json(
        app.clone(),
        "/api/settings/connectors",
        json!({
            "source_type": "github",
            "name": "test-gh",
            "config": { "repo": "not-a-repo" },
            "enabled": true
        }),
    )
    .await;
    let conn_id = gh_conn["connector"]["id"].as_str().unwrap();
    let gh_res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/settings/connectors/{conn_id}/github/issues"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(gh_res.status(), axum::http::StatusCode::BAD_GATEWAY);

    let linear_conn = post_json(
        app.clone(),
        "/api/settings/connectors",
        json!({
            "source_type": "linear",
            "name": "test-linear",
            "config": { "team_key": "ENG" },
            "enabled": true
        }),
    )
    .await;
    let linear_id = linear_conn["connector"]["id"].as_str().unwrap();
    let linear_res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!(
                    "/api/settings/connectors/{linear_id}/linear/issues"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(linear_res.status(), axum::http::StatusCode::BAD_GATEWAY);

    let notifications = get_json(app.clone(), "/api/notifications/recent?limit=5").await;
    assert!(notifications["notifications"].is_array());

    let put_prefs = put_json(
        app.clone(),
        "/api/settings/preferences",
        json!({
            "host": "127.0.0.1",
            "port": 43180,
            "db_path": db.display().to_string(),
        }),
    )
    .await;
    assert_eq!(put_prefs["ok"], true);

    let reports = get_json(app.clone(), "/api/reports/recent").await;
    assert!(reports["reports"].is_array());

    let ops = get_json(app.clone(), "/api/settings/db-operations").await;
    assert!(ops["operations"]["migrations"].is_array());

    let notify = post_json(
        app.clone(),
        "/api/settings/notifications/test",
        json!({ "event_type": "session_report_generated" }),
    )
    .await;
    assert_eq!(notify["ok"], true);

    let policy = post_json(
        app.clone(),
        "/api/settings/notifications",
        json!({
            "event_type": "gate_failed",
            "channel": "local_log",
            "config": {},
            "enabled": true
        }),
    )
    .await;
    let policy_id = policy["policy"]["id"].as_str().unwrap();

    let disabled = patch_json(
        app.clone(),
        &format!("/api/settings/notifications/{policy_id}/enabled"),
        json!({ "enabled": false }),
    )
    .await;
    assert_eq!(disabled["policy"]["enabled"], false);

    let deleted = delete_json(
        app.clone(),
        &format!("/api/settings/notifications/{policy_id}"),
    )
    .await;
    assert_eq!(deleted["ok"], true);

    let skills = get_json(app.clone(), "/api/skills?limit=5").await;
    if let Some(skill_id) = skills["skills"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|s| s["id"].as_str())
    {
        let all_on = post_json(
            app.clone(),
            &format!("/api/skills/{skill_id}/all-projects"),
            json!({ "enabled": true }),
        )
        .await;
        assert_eq!(all_on["ok"], true);
        assert!(all_on["projects_updated"].is_number());
    }

    let skills_after = get_json(app.clone(), "/api/skills?limit=5").await;
    if let Some(skill_id) = skills_after["skills"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|s| s["id"].as_str())
    {
        let detail = get_json(app, &format!("/api/skills/{skill_id}")).await;
        assert!(detail["skill"]["projects"].is_array());
    }
    std::env::remove_var("ANYCODE_DASHBOARD_PREFERENCES_PATH");
}

#[tokio::test]
async fn projects_pagination_and_missing_root_create() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("pagination.db");
    let app = app_for_test(&db).await.unwrap();

    for i in 0..3 {
        post_json(
            app.clone(),
            "/api/projects",
            json!({
                "root_path": dir.path().join(format!("proj-{i}")).display().to_string(),
                "name": format!("Project {i}"),
                "create_root": true
            }),
        )
        .await;
    }

    let page0 = get_json(app.clone(), "/api/projects?limit=2&offset=0").await;
    assert_eq!(page0["limit"], 2);
    assert_eq!(page0["projects"].as_array().unwrap().len(), 2);
    assert!(page0["total"].as_i64().unwrap() >= 3);

    let page1 = get_json(app.clone(), "/api/projects?limit=2&offset=2").await;
    assert!(page1["projects"].as_array().unwrap().len() >= 1);

    let missing_root = dir.path().join("auto-create-chat");
    std::fs::create_dir_all(&missing_root).unwrap();
    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": missing_root.display().to_string(),
            "name": "Auto create chat",
            "create_root": true
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap();
    std::fs::remove_dir_all(&missing_root).unwrap();
    assert!(!missing_root.exists());

    let session = post_json(
        app.clone(),
        "/api/sessions",
        json!({
            "project_id": project_id,
            "kind": "repl",
            "title": "chat",
            "prompt_preview": "hello"
        }),
    )
    .await;
    let session_id = session["session"]["id"].as_str().unwrap();

    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{session_id}/message"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "prompt": "hello from fixture" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        missing_root.is_dir(),
        "expected auto-created project root, status={}",
        res.status()
    );
}

#[tokio::test]
async fn project_rename_updates_name() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("rename.db");
    let app = app_for_test(&db).await.unwrap();

    let root = dir.path().join("rename-proj");
    std::fs::create_dir_all(&root).unwrap();
    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": root.display().to_string(),
            "name": "Before rename",
            "create_root": true
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap().to_string();

    let renamed = patch_json(
        app.clone(),
        &format!("/api/projects/{project_id}"),
        json!({ "name": "  After rename  " }),
    )
    .await;
    assert_eq!(renamed["ok"], true);
    assert_eq!(renamed["name"], "After rename");

    let detail = get_json(app.clone(), &format!("/api/projects/{project_id}")).await;
    assert_eq!(detail["project"]["name"], "After rename");

    // Empty (whitespace-only) name is rejected.
    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri(format!("/api/projects/{project_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "   " }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);

    // Overly long name is rejected.
    let long_name = "x".repeat(121);
    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri(format!("/api/projects/{project_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": long_name }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);

    // Unknown project id returns 404.
    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri("/api/projects/does-not-exist")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "whatever" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn session_message_rejects_invalid_skills() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("message_skills.db");
    let app = app_for_test(&db).await.unwrap();
    let root = dir.path().join("skills-proj");
    std::fs::create_dir_all(&root).unwrap();
    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": root.display().to_string(),
            "name": "Skills validation",
            "create_root": true
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap();
    let session = post_json(
        app.clone(),
        "/api/sessions",
        json!({
            "project_id": project_id,
            "kind": "repl",
            "title": "chat",
            "prompt_preview": "hi"
        }),
    )
    .await;
    let session_id = session["session"]["id"].as_str().unwrap();
    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{session_id}/message"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "prompt": "hello",
                        "skills": ["bad skill id!!!"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn session_message_rejects_empty_prompt() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("message_empty.db");
    let app = app_for_test(&db).await.unwrap();
    let root = dir.path().join("empty-proj");
    std::fs::create_dir_all(&root).unwrap();
    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": root.display().to_string(),
            "name": "Empty prompt",
            "create_root": true
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap();
    let session = post_json(
        app.clone(),
        "/api/sessions",
        json!({
            "project_id": project_id,
            "kind": "repl",
            "title": "chat",
            "prompt_preview": "hi"
        }),
    )
    .await;
    let session_id = session["session"]["id"].as_str().unwrap();
    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{session_id}/message"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "prompt": "   " }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn session_message_unknown_session_is_404() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("message_404.db");
    let app = app_for_test(&db).await.unwrap();
    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/sessions/sess_missing/message")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "prompt": "hello" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn session_transcript_api_returns_blocks() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("transcript_api.db");
    let app = app_for_test(&db).await.unwrap();
    let root = dir.path().join("transcript-proj");
    std::fs::create_dir_all(&root).unwrap();
    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": root.display().to_string(),
            "name": "Transcript",
            "create_root": true
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap();
    let session = post_json(
        app.clone(),
        "/api/sessions",
        json!({
            "project_id": project_id,
            "kind": "repl",
            "title": "chat",
            "prompt_preview": "hi"
        }),
    )
    .await;
    let session_id = session["session"]["id"].as_str().unwrap();
    post_json(
        app.clone(),
        &format!("/api/projects/{project_id}/events"),
        json!({
            "project_id": project_id,
            "session_id": session_id,
            "event_type": "user_prompt",
            "title": "User prompt",
            "body": "hello"
        }),
    )
    .await;
    post_json(
        app.clone(),
        &format!("/api/projects/{project_id}/events"),
        json!({
            "project_id": project_id,
            "session_id": session_id,
            "event_type": "assistant_response",
            "title": "Assistant",
            "body": "world"
        }),
    )
    .await;

    let transcript = get_json(app, &format!("/api/sessions/{session_id}/transcript")).await;
    assert!(transcript["transcript"]["blocks"].is_array());
    assert!(transcript["transcript"]["blocks"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn artifacts_kind_and_exclude_kind_filters() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("artifact_filters.db");
    let work = dir.path().join("repo");
    std::fs::create_dir_all(&work).unwrap();

    let db = anycode_dashboard::db::DashboardDb::open(&db_path)
        .await
        .unwrap();
    let project = db
        .upsert_project(anycode_dashboard::schema::UpsertProjectRequest {
            root_path: work.to_string_lossy().into(),
            name: Some("artifact-filter-test".into()),
            description: None,
            create_root: None,
            ..Default::default()
        })
        .await
        .unwrap();
    db.upsert_artifact(&project.id, "", "src/main.rs", "file", "main.rs")
        .await
        .unwrap();
    db.upsert_artifact(
        &project.id,
        "",
        "dashboard/reports/project/r1/t.md",
        "report",
        "Report: r1",
    )
    .await
    .unwrap();
    db.pool().close().await;

    let app = app_for_test(&db_path).await.unwrap();

    let all = get_json(app.clone(), "/api/artifacts?limit=10").await;
    assert_eq!(all["artifacts"].as_array().unwrap().len(), 2);

    let reports = get_json(app.clone(), "/api/artifacts?kind=report&limit=10").await;
    let reports = reports["artifacts"].as_array().unwrap().clone();
    assert_eq!(reports.len(), 1);
    assert!(reports.iter().all(|a| a["kind"] == "report"));

    let deliverables = get_json(app.clone(), "/api/artifacts?exclude_kind=report&limit=10").await;
    let deliverables = deliverables["artifacts"].as_array().unwrap().clone();
    assert_eq!(deliverables.len(), 1);
    assert!(deliverables.iter().all(|a| a["kind"] != "report"));

    let scoped = get_json(
        app.clone(),
        &format!(
            "/api/projects/{}/artifacts?exclude_kind=report&limit=10",
            project.id
        ),
    )
    .await;
    assert_eq!(scoped["artifacts"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn memory_retention_preview_api() {
    let bin = std::env::var("CARGO_BIN_EXE_anycode")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let candidate =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/anycode");
            candidate.is_file().then_some(candidate)
        });
    let Some(bin) = bin else {
        eprintln!("skip memory_retention_preview_api: anycode binary not found");
        return;
    };
    std::env::set_var("ANYCODE_BIN", &bin);
    let dir = tempdir().unwrap();
    let db = dir.path().join("mem_retention.db");
    let app = app_for_test(&db).await.unwrap();
    let v = get_json(app, "/api/settings/memory/retention?older_than_days=3650").await;
    assert!(v.get("summary").is_some(), "expected summary: {v:?}");
    assert_eq!(
        v.get("older_than_days").and_then(|x| x.as_i64()),
        Some(3650)
    );
}

#[tokio::test]
async fn setup_memory_and_complete_with_isolated_home() {
    let home = tempdir().unwrap();
    let cfg_dir = home.path().join(".anycode");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.json"),
        r#"{"provider":"openai","model":"gpt-4o","api_key":"sk-test"}"#,
    )
    .unwrap();
    std::env::set_var("HOME", home.path());

    let dir = tempdir().unwrap();
    let db = dir.path().join("setup_mem.db");
    let app = app_for_test(&db).await.unwrap();

    let hybrid = patch_json(
        app.clone(),
        "/api/setup/memory",
        json!({ "preset": "hybrid" }),
    )
    .await;
    assert_eq!(hybrid["ok"], true);
    let saved: Value =
        serde_json::from_slice(&std::fs::read(cfg_dir.join("config.json")).unwrap()).unwrap();
    assert_eq!(saved["memory"]["backend"], "hybrid");

    let bad = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri("/api/setup/memory")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "preset": "pipeline_http" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad.status(), axum::http::StatusCode::BAD_REQUEST);

    let complete = post_json(
        app,
        "/api/setup/complete",
        json!({ "scan_projects": false }),
    )
    .await;
    assert_eq!(complete["ok"], true);
    assert!(complete["setup_completed_at"].is_string());
}

#[tokio::test]
async fn skills_market_and_scan_roots() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("skills_market.db");
    let app = app_for_test(&db).await.unwrap();

    let market = get_json(app.clone(), "/api/skills/market").await;
    assert!(market["market"]["entries"].is_array());

    let skills = get_json(app.clone(), "/api/skills?limit=10").await;
    assert!(skills["skills"].is_array());
    assert!(skills["scan_roots"].is_number());
}

#[tokio::test]
async fn security_questions_roundtrip() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("questions.db");
    let state_dir = dir.path().join("dashboard-state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::env::set_var("ANYCODE_DASHBOARD_STATE_DIR", &state_dir);

    let app = app_for_test(&db).await.unwrap();
    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": dir.path().join("proj").display().to_string(),
            "name": "q-proj",
            "create_root": true
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap();
    let session = post_json(
        app.clone(),
        "/api/sessions",
        json!({
            "project_id": project_id,
            "kind": "repl",
            "title": "q-session"
        }),
    )
    .await;
    let session_id = session["session"]["id"].as_str().unwrap();

    let qid = anycode_dashboard::ipc::question_ipc::register_pending(
        session_id,
        "Pick one",
        "Choice",
        &[anycode_dashboard::ipc::question_ipc::QuestionOptionRecord {
            label: "A".into(),
            description: String::new(),
        }],
        false,
    )
    .unwrap();

    let pending = get_json(app.clone(), "/api/security/questions/pending?limit=5").await;
    let list = pending["pending"].as_array().unwrap();
    assert!(list.iter().any(|q| q["question_id"] == qid));

    let responded = post_json(
        app.clone(),
        &format!("/api/security/questions/{qid}/respond"),
        json!({ "selected_labels": ["A"] }),
    )
    .await;
    assert_eq!(responded["ok"], true);
}

#[tokio::test]
async fn session_message_with_text_files() {
    let bin = std::env::var("CARGO_BIN_EXE_anycode")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let candidate =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/anycode");
            candidate.is_file().then_some(candidate)
        });
    let Some(bin) = bin else {
        eprintln!("skip session_message_with_text_files: anycode binary not found");
        return;
    };
    std::env::set_var("ANYCODE_BIN", &bin);

    let dir = tempdir().unwrap();
    let state_dir = dir.path().join("dashboard-state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::env::set_var("ANYCODE_DASHBOARD_STATE_DIR", &state_dir);

    let db = dir.path().join("text_files.db");
    let app = app_for_test(&db).await.unwrap();
    let project = post_json(
        app.clone(),
        "/api/projects",
        json!({
            "root_path": dir.path().join("tf-proj").display().to_string(),
            "name": "tf",
            "create_root": true
        }),
    )
    .await;
    let project_id = project["project"]["id"].as_str().unwrap();
    let session = post_json(
        app.clone(),
        "/api/sessions",
        json!({
            "project_id": project_id,
            "kind": "repl",
            "title": "tf-session"
        }),
    )
    .await;
    let session_id = session["session"]["id"].as_str().unwrap();

    let res = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{session_id}/message"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "prompt": "read attached",
                        "text_files": [{
                            "filename": "note.txt",
                            "content": "hello from upload"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.status().is_success() || res.status() == axum::http::StatusCode::ACCEPTED,
        "message status={}",
        res.status()
    );
}

#[tokio::test]
async fn quick_auth_presets_match_setup_crate() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("quick_auth.db");
    let app = app_for_test(&db).await.unwrap();
    let quick = get_json(app, "/api/setup/quick-auth").await;
    let presets = quick["presets"].as_array().unwrap();
    assert_eq!(presets.len(), anycode_setup::QUICK_AUTH_CHOICES.len());
}
