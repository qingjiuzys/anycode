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

#[tokio::test]
async fn recorder_begin_attaches_precreated_session_and_preserves_title() {
    use anycode_dashboard::schema::CreateSessionRequest;
    use std::sync::Arc;

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("projects.db");
    let db = DashboardDb::open(&db_path).await.unwrap();
    let work = dir.path().join("repo");
    std::fs::create_dir_all(&work).unwrap();

    let project = db
        .upsert_project(anycode_dashboard::schema::UpsertProjectRequest {
            root_path: work.to_string_lossy().into(),
            name: Some("attach-test".into()),
            description: None,
            create_root: None,
        })
        .await
        .unwrap();

    let planned = db
        .create_planned_session(CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "run".into(),
            task_id: None,
            title: "My custom session name".into(),
            prompt_preview: Some("Do the thing".into()),
            agent_type: None,
            model: None,
            metadata_json: Some(r#"{"source":"test"}"#.into()),
        })
        .await
        .unwrap();
    assert_eq!(planned.status, "pending");

    let _ = db
        .insert_event(anycode_dashboard::schema::InsertEventRequest {
            project_id: project.id.clone(),
            session_id: Some(planned.id.clone()),
            task_id: None,
            agent_id: None,
            event_type: "user_prompt".into(),
            severity: Some("info".into()),
            title: "User prompt".into(),
            body: Some("Do the thing".into()),
            payload: None,
        })
        .await;

    let task_id = Uuid::new_v4();
    let task = Task {
        id: task_id,
        agent_type: AgentType::new("default"),
        prompt: "Do the thing".into(),
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

    std::env::set_var(
        anycode_dashboard::ipc::approval_ipc::SESSION_ENV,
        &planned.id,
    );

    let rec = DashboardRecorder::begin(
        Arc::new(db.clone()),
        RunSessionKind::Run,
        &task,
        "Would overwrite title",
    )
    .await
    .unwrap();

    std::env::remove_var(anycode_dashboard::ipc::approval_ipc::SESSION_ENV);

    assert_eq!(rec.session_id(), planned.id);

    let session = db.get_session(&planned.id).await.unwrap().unwrap();
    assert_eq!(session.title, "My custom session name");
    assert_eq!(session.status, "running");
    assert_eq!(
        session.task_id.as_deref(),
        Some(task_id.to_string().as_str())
    );

    let events = db
        .list_session_events(&planned.id, None, 50, None, None, None)
        .await
        .unwrap();
    let user_prompts: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "user_prompt")
        .collect();
    assert_eq!(user_prompts.len(), 1, "should not duplicate user_prompt");
}
