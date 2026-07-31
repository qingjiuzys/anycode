//! Push `question_request` chat SSE when AskUserQuestion registers a pending form.

use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::observability::chat_events::question_request_event;
use crate::observability::chat_turn_log::persist_and_enrich;
use anycode_dashboard_ipc::question_ipc::{self, PendingQuestionRecord};
use std::sync::Arc;

/// Wire dashboard SSE for newly registered AskUserQuestion forms.
pub fn install(events: Arc<EventBus>, db: DashboardDb) {
    question_ipc::set_register_hook(Box::new(move |rec: &PendingQuestionRecord| {
        let events = Arc::clone(&events);
        let db = db.clone();
        let rec = rec.clone();
        tokio::spawn(async move {
            if let Err(error) = publish_question_request(&db, &events, &rec).await {
                tracing::warn!(%error, question_id = %rec.question_id, "question_request SSE skipped");
            }
        });
    }));
}

async fn publish_question_request(
    db: &DashboardDb,
    events: &EventBus,
    rec: &PendingQuestionRecord,
) -> anyhow::Result<()> {
    let session = db
        .get_session(&rec.session_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("session not found for question"))?;
    let chat_evt =
        question_request_event(&rec.session_id, &session.project_id, rec.user_turn_id, rec);
    match persist_and_enrich(db, chat_evt.clone(), rec.user_turn_id).await {
        Ok(enriched) => events.publish_chat(enriched),
        Err(error) => {
            tracing::warn!(
                %error,
                question_id = %rec.question_id,
                "question_request persist failed — publishing live SSE only"
            );
            events.publish_chat(chat_evt);
        }
    }
    Ok(())
}
