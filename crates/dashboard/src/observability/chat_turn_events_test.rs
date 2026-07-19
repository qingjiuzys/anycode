use crate::db::DashboardDb;
use crate::observability::chat_turn_log::{
    persist_and_enrich, records_to_transcript_blocks, user_message_event,
};
use crate::schema::{CreateSessionRequest, UpsertProjectRequest};

#[tokio::test]
async fn chat_turn_events_persist_and_hydrate_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let db = DashboardDb::open(&dir.path().join("test.db"))
        .await
        .unwrap();
    let project = db
        .upsert_project(UpsertProjectRequest {
            root_path: "/tmp/chat-turn".into(),
            name: Some("Chat Turn".into()),
            description: None,
            create_root: None,
            ..Default::default()
        })
        .await
        .unwrap();
    let session = db
        .create_session(CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "repl".into(),
            task_id: None,
            title: "Chat".into(),
            prompt_preview: Some("hello".into()),
            agent_type: Some("workspace-assistant".into()),
            model: Some("test".into()),
            metadata_json: None,
        })
        .await
        .unwrap();

    let user_evt = user_message_event(&session.id, &project.id, 1, "hello");
    persist_and_enrich(&db, user_evt, 1).await.unwrap();

    let llm_evt = crate::observability::chat_events::assistant_delta_event(
        &session.id,
        &project.id,
        1,
        1,
        "Hi",
        "Hi",
        false,
    );
    persist_and_enrich(&db, llm_evt, 1).await.unwrap();

    let records = db
        .list_chat_turn_events(&session.id, None, 100)
        .await
        .unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].seq, 1);
    assert_eq!(records[1].seq, 2);
    assert_eq!(records[0].conversation_turn_id, 1);
    assert_eq!(records[1].conversation_turn_id, 1);

    let blocks = records_to_transcript_blocks(&records);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].block_type, "user_message");
    assert_eq!(blocks[0].body, "hello");
    assert_eq!(blocks[1].block_type, "assistant_message");
    assert_eq!(blocks[1].body, "Hi");

    let transcript = crate::session_transcript::session_transcript(&db, &session.id)
        .await
        .unwrap();
    assert_eq!(transcript.max_seq, Some(2));
    assert_eq!(transcript.blocks.len(), 2);
    assert_eq!(transcript.blocks[0].body, "hello");
}

#[tokio::test]
async fn chat_turn_events_replay_after_seq() {
    let dir = tempfile::tempdir().unwrap();
    let db = DashboardDb::open(&dir.path().join("test.db"))
        .await
        .unwrap();
    let project = db
        .upsert_project(UpsertProjectRequest {
            root_path: "/tmp/replay".into(),
            name: Some("Replay".into()),
            description: None,
            create_root: None,
            ..Default::default()
        })
        .await
        .unwrap();
    let session = db
        .create_session(CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "repl".into(),
            task_id: None,
            title: "Replay".into(),
            prompt_preview: Some("one".into()),
            agent_type: Some("workspace-assistant".into()),
            model: Some("test".into()),
            metadata_json: None,
        })
        .await
        .unwrap();

    for (turn, prompt) in [(1, "one"), (2, "two"), (3, "three")] {
        let evt = user_message_event(&session.id, &project.id, turn, prompt);
        persist_and_enrich(&db, evt, turn).await.unwrap();
    }

    let replay = db
        .list_chat_turn_events(&session.id, Some(1), 10)
        .await
        .unwrap();
    assert_eq!(replay.len(), 2);
    assert_eq!(replay[0].body, "two");
    assert_eq!(replay[1].body, "three");
}
