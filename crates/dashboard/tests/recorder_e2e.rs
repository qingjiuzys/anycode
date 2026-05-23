//! CLI recorder → SQLite → events integration (no live HTTP).

use anycode_core::{AgentType, DiskTaskOutput, Task, TaskBudget, TaskContext};
use anycode_dashboard::{DashboardDb, DashboardRecorder, RunSessionKind};
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
async fn recorder_begin_inserts_user_prompt_and_ingests_log() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("projects.db");
    let db = DashboardDb::open(&db_path).await.unwrap();
    let work = dir.path().join("repo");
    std::fs::create_dir_all(&work).unwrap();

    let task_id = Uuid::new_v4();
    let task = Task {
        id: task_id,
        agent_type: AgentType::new("default"),
        prompt: "Add a README section for dashboard".into(),
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: work.to_string_lossy().into(),
            environment: Default::default(),
            user_id: None,
            system_prompt_append: None,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
            tool_deny_names: vec![],
            tool_deny_prefixes: vec![],
            budget: TaskBudget::default(),
        },
        created_at: chrono::Utc::now(),
    };

    let mut rec = DashboardRecorder::begin(
        std::sync::Arc::new(db.clone()),
        RunSessionKind::Run,
        &task,
        "Add README",
    )
    .await
    .unwrap();

    let events = db
        .list_session_events(rec.session_id(), None, 50, None, None, None)
        .await
        .unwrap();
    assert!(
        events.iter().any(|e| e.event_type == "user_prompt"),
        "expected user_prompt event at session start"
    );

    let disk = DiskTaskOutput::new(dir.path().join("tasks"));
    disk.ensure_initialized(task_id).unwrap();
    disk.append_line(
        task_id,
        &anycode_core::format_assistant_response_log_line(1, "Done — added section."),
    )
    .unwrap();
    disk.append_line(task_id, "[task_end] status=completed")
        .unwrap();

    rec.ingest_full_log(&disk, task_id).await;
    rec.finish_run(&disk, task_id, Some("ok")).await;

    let events = db
        .list_session_events(rec.session_id(), None, 50, None, None, None)
        .await
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.event_type == "assistant_response" && e.body.contains("added section")),
        "expected assistant_response from log ingest"
    );

    let session = db.get_session(rec.session_id()).await.unwrap().unwrap();
    assert_eq!(session.status, "completed");

    let projects = db.list_projects().await.unwrap();
    assert_eq!(projects.len(), 1);
    assert!(projects[0].root_path.contains("repo"));
}
