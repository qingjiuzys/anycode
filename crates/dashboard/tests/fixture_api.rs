//! API integration tests against fixture databases.

use anycode_dashboard::server::app_for_test;
use axum::body::Body;
use http_body_util::BodyExt;
use serde_json::{json, Value};
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

#[tokio::test]
async fn fixture_api_smoke() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("fixture.db");
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

    let runtime = get_json(app.clone(), "/api/settings/runtime").await;
    assert_eq!(runtime["runtime"]["auth_mode"], "local_trusted");
    assert!(runtime["runtime"]["db_path"].is_string());

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
}
