//! Push `approval_request` chat SSE when tool approval registers a pending row.

use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::observability::chat_events::approval_request_event;
use crate::observability::chat_turn_log::persist_and_enrich;
use anycode_dashboard_ipc::approval_ipc::{self, PendingApprovalRecord};
use std::sync::Arc;

/// Wire dashboard SSE for newly registered tool approvals.
pub fn install(events: Arc<EventBus>, db: DashboardDb) {
    approval_ipc::set_register_hook(Box::new(move |rec: &PendingApprovalRecord| {
        let events = Arc::clone(&events);
        let db = db.clone();
        let rec = rec.clone();
        tokio::spawn(async move {
            if let Err(error) = publish_approval_request(&db, &events, &rec).await {
                tracing::warn!(%error, approval_id = %rec.approval_id, "approval_request SSE skipped");
            }
        });
    }));
}

async fn publish_approval_request(
    db: &DashboardDb,
    events: &EventBus,
    rec: &PendingApprovalRecord,
) -> anyhow::Result<()> {
    let session = db
        .get_session(&rec.session_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("session not found for approval"))?;
    let user_turn_id = rec.user_turn_id;
    let chat_evt = approval_request_event(&rec.session_id, &session.project_id, user_turn_id, rec);
    match persist_and_enrich(db, chat_evt, user_turn_id).await {
        Ok(enriched) => events.publish_chat(enriched),
        Err(error) => tracing::warn!(%error, "approval_request persist failed"),
    }
    Ok(())
}

pub async fn publish_approval_resolved(
    db: &DashboardDb,
    events: &EventBus,
    session_id: &str,
    user_turn_id: u32,
    approval_id: &str,
    decision: &str,
) {
    let Ok(Some(session)) = db.get_session(session_id).await else {
        return;
    };
    let chat_evt = crate::observability::chat_events::approval_resolved_event(
        session_id,
        &session.project_id,
        user_turn_id,
        approval_id,
        decision,
    );
    match persist_and_enrich(db, chat_evt, user_turn_id).await {
        Ok(enriched) => events.publish_chat(enriched),
        Err(error) => tracing::warn!(%error, "approval_resolved persist failed"),
    }
}

pub async fn publish_question_resolved(
    db: &DashboardDb,
    events: &EventBus,
    session_id: &str,
    user_turn_id: u32,
    question_id: &str,
) {
    let Ok(Some(session)) = db.get_session(session_id).await else {
        return;
    };
    let chat_evt = crate::observability::chat_events::question_resolved_event(
        session_id,
        &session.project_id,
        user_turn_id,
        question_id,
    );
    match persist_and_enrich(db, chat_evt, user_turn_id).await {
        Ok(enriched) => events.publish_chat(enriched),
        Err(error) => tracing::warn!(%error, "question_resolved persist failed"),
    }
}
