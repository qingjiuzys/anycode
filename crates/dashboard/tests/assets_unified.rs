//! Unified asset center API tests.

use anycode_dashboard::assets::{artifact_to_asset_item, path_is_media, skill_asset_id};
use anycode_dashboard::db::DashboardDb;
use anycode_dashboard::schema::{ArtifactRecord, UpsertProjectRequest};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn media_path_detection() {
    assert!(path_is_media("/tmp/photo.png"));
    assert!(!path_is_media("/tmp/main.rs"));
}

#[test]
fn artifact_maps_to_deliverable_and_media() {
    let file = ArtifactRecord {
        id: "a1".into(),
        path: "/proj/src/main.rs".into(),
        kind: "file".into(),
        title: "main.rs".into(),
        trust_level: "needs_verify".into(),
        verified_by_gate_id: None,
        session_id: None,
        project_id: Some("p1".into()),
        project_name: None,
        verified_by_gate_name: None,
        session_trusted_status: None,
        updated_at: None,
    };
    let item = artifact_to_asset_item(&file, &json!({}), true);
    assert_eq!(item.asset_kind, "deliverable");

    let media = ArtifactRecord {
        path: "/proj/assets/logo.png".into(),
        ..file.clone()
    };
    let media_item = artifact_to_asset_item(&media, &json!({}), true);
    assert_eq!(media_item.asset_kind, "media");
}

#[tokio::test]
async fn list_unified_assets_includes_skill_and_artifact() {
    let dir = tempdir().unwrap();
    let db = DashboardDb::open(&dir.path().join("test.db"))
        .await
        .unwrap();
    let project = db
        .upsert_project(UpsertProjectRequest {
            root_path: dir.path().to_string_lossy().to_string(),
            name: Some("Demo".into()),
            description: None,
            create_root: None,
            ..Default::default()
        })
        .await
        .unwrap();

    db.upsert_artifact(&project.id, "", "/tmp/demo/output.md", "file", "output.md")
        .await
        .unwrap();
    db.upsert_skill(
        "demo-skill",
        "Demo Skill",
        "A test skill",
        None,
        "/tmp/skills/demo-skill",
        Some("other"),
    )
    .await
    .unwrap();
    db.link_project_skill(&project.id, "demo-skill", true)
        .await
        .unwrap();

    let items = anycode_dashboard::assets::list_unified_assets(
        &db,
        Some(&project.id),
        None,
        Some("all"),
        None,
        None,
        None,
        false,
        false,
        false,
        true,
        50,
    )
    .await
    .unwrap();

    assert!(items
        .iter()
        .any(|a| a.asset_kind == "deliverable" || a.asset_kind == "file"));
    assert!(items.iter().any(|a| a.id == skill_asset_id("demo-skill")));
}
