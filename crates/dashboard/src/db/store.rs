use crate::db::trusted::compute_trusted_status;
use crate::schema::{
    AgentUsageStat, ArtifactRecord, CreateSessionRequest, GateRecord, InsertEventRequest,
    LabelCount, OverviewStats, ProjectDetail, ProjectEvent, ProjectStats, ProjectStatsFailure,
    ProjectSummary, PruneStaleProjectsReport, PrunedProjectRow, RecentEvent, SessionDetail,
    SessionFacetsResponse, SessionSummary, SessionWithProject, SkillRecord, UpsertProjectRequest,
    LOCAL_ORG_ID,
};
use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
pub struct DashboardDb {
    pub(super) pool: SqlitePool,
    pub(super) path: PathBuf,
}

impl DashboardDb {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn sync_workspace_paths(&self, paths: &[String]) -> Result<usize> {
        let mut n = 0usize;
        for root in paths {
            let name = Path::new(root)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string();
            self.upsert_project(UpsertProjectRequest {
                root_path: root.clone(),
                name: Some(name),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await?;
            n += 1;
        }
        Ok(n)
    }
}

pub mod agents;
mod artifacts;
mod events;
mod gates;
mod open;
mod projects;
mod services;
mod sessions;
mod skills;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, InsertEventRequest, UpsertProjectRequest};
    use tempfile::tempdir;

    #[tokio::test]
    async fn migration_and_event_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = DashboardDb::open(&path).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/demo".into(),
                name: Some("Demo".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "goal".into(),
                task_id: Some("task-1".into()),
                title: "Test goal".into(),
                prompt_preview: None,
                agent_type: Some("goal".into()),
                model: Some("test-model".into()),
                metadata_json: None,
            })
            .await
            .unwrap();
        let project_id = project.id.clone();
        let evt = db
            .insert_event(InsertEventRequest {
                project_id,
                session_id: Some(session.id.clone()),
                task_id: Some("task-1".into()),
                agent_id: None,
                event_type: "tool_call_end".into(),
                severity: Some("info".into()),
                title: "Bash finished".into(),
                body: Some("ok".into()),
                payload: None,
            })
            .await
            .unwrap();
        assert_eq!(evt.event_type, "tool_call_end");
        let listed = db
            .list_session_events(&session.id, None, 10, None, None, None)
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        let fetched = db.get_event(&evt.id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Bash finished");
        assert_eq!(fetched.body, "ok");
        let stats = db.get_project_stats(&project.id).await.unwrap();
        assert!(!stats.event_types.is_empty());
    }

    #[tokio::test]
    async fn failed_required_gate_blocks_trusted() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("gates.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/gate-test".into(),
                name: Some("GateTest".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "goal".into(),
                task_id: Some("t1".into()),
                title: "gate session".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.upsert_gate(
            &project.id,
            &session.id,
            "flutter test",
            "flutter test",
            "failed",
            true,
            "assertion failed",
        )
        .await
        .unwrap();
        db.refresh_session_trusted_status(&session.id)
            .await
            .unwrap();
        let updated = db.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(updated.trusted_status, "blocked");
        let (listed, _) = db
            .list_projects_paged(None, None, 50, 0, "updated_at_desc")
            .await
            .unwrap();
        let summary = listed.iter().find(|p| p.id == project.id).unwrap();
        let score = summary.trust_score.expect("expected trust score");
        assert!(
            score < 0.9,
            "trust score should reflect failed gate: {score}"
        );
    }

    #[tokio::test]
    async fn project_without_activity_has_no_trust_score() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("trust-none.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/trust-none".into(),
                name: Some("NoRuns".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let detail = db.get_project(&project.id).await.unwrap().unwrap();
        assert_eq!(detail.trust_score, None);
        let (listed, _) = db
            .list_projects_paged(None, None, 50, 0, "updated_at_desc")
            .await
            .unwrap();
        let summary = listed.iter().find(|p| p.id == project.id).unwrap();
        assert_eq!(summary.trust_score, None);
    }

    #[tokio::test]
    async fn passed_gates_verify_session_artifacts() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("art-trust.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/art-trust".into(),
                name: Some("ArtTrust".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "goal".into(),
                task_id: Some("t1".into()),
                title: "artifact trust".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.upsert_artifact(
            &project.id,
            &session.id,
            "lib/main.dart",
            "file",
            "main.dart",
        )
        .await
        .unwrap();
        let gate_id = db
            .upsert_gate(
                &project.id,
                &session.id,
                "cargo test",
                "cargo test",
                "passed",
                true,
                "",
            )
            .await
            .unwrap();
        db.refresh_session_trusted_status(&session.id)
            .await
            .unwrap();
        let updated = db.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(updated.trusted_status, "verified");
        let arts = db
            .list_artifacts(None, Some(&session.id), None, None, false, false, 10)
            .await
            .unwrap();
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].trust_level, "verified");
        assert_eq!(
            arts[0].verified_by_gate_id.as_deref(),
            Some(gate_id.as_str())
        );
    }

    #[tokio::test]
    async fn manual_gate_session_is_stable() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("manual-gate.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/manual-gate".into(),
                name: Some("MG".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let a = db.ensure_manual_gate_session(&project.id).await.unwrap();
        let b = db.ensure_manual_gate_session(&project.id).await.unwrap();
        assert_eq!(a, b);
        let session = db.get_session(&a).await.unwrap().unwrap();
        assert_eq!(session.kind, "manual_gate");
    }

    #[tokio::test]
    async fn project_identity_uses_normalized_root_hash() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("identity.db"))
            .await
            .unwrap();
        let root = dir.path().join("wd");
        std::fs::create_dir_all(&root).unwrap();
        let a = db
            .upsert_project(UpsertProjectRequest {
                root_path: root.display().to_string(),
                name: Some("wd".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let b = db
            .upsert_project(UpsertProjectRequest {
                root_path: root.join(".").display().to_string(),
                name: Some("also wd".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(a.id, b.id);
        assert!(a.id.starts_with("proj_"));
        assert_eq!(db.list_projects().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn cancel_running_session_only() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("cancel.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/cancel".into(),
                name: Some("C".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "run".into(),
                task_id: None,
                title: "running".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        assert_eq!(session.status, "running");
        assert!(db.cancel_running_session(&session.id).await.unwrap());
        let updated = db.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(updated.status, "cancelled");
        assert!(!db.cancel_running_session(&session.id).await.unwrap());
    }

    #[tokio::test]
    async fn prune_stale_projects_removes_missing_roots() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("prune.db"))
            .await
            .unwrap();
        let alive = dir.path().join("alive");
        std::fs::create_dir_all(&alive).unwrap();
        db.upsert_project(UpsertProjectRequest {
            root_path: alive.display().to_string(),
            name: Some("alive".into()),
            description: None,
            create_root: None,
            ..Default::default()
        })
        .await
        .unwrap();
        db.upsert_project(UpsertProjectRequest {
            root_path: "/tmp/anycode-prune-missing".into(),
            name: Some("gone".into()),
            description: None,
            create_root: None,
            ..Default::default()
        })
        .await
        .unwrap();

        let preview = db.prune_stale_projects(true).await.unwrap();
        assert_eq!(preview.removed.len(), 1);
        assert_eq!(preview.kept, 1);
        assert_eq!(db.list_projects().await.unwrap().len(), 2);

        let applied = db.prune_stale_projects(false).await.unwrap();
        assert_eq!(applied.removed.len(), 1);
        assert_eq!(applied.kept, 1);
        assert_eq!(db.list_projects().await.unwrap().len(), 1);
        assert_eq!(db.list_projects().await.unwrap()[0].name, "alive");
    }
}
