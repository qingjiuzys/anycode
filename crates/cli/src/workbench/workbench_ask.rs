//! Dashboard Web `AskUserQuestion` host (file IPC poll).

use anycode_dashboard::approval_ipc::SESSION_ENV;
use anycode_dashboard::question_ipc::{self, QuestionOptionRecord};
use anycode_tools::{
    AskUserQuestionHost, AskUserQuestionHostError, AskUserQuestionRequest, AskUserQuestionResponse,
};
use async_trait::async_trait;
use std::time::Duration;

const WEB_POLL_MS: u64 = 400;
const WEB_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub struct WorkbenchAskUserQuestionHost;

impl WorkbenchAskUserQuestionHost {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn session_id() -> Option<String> {
        std::env::var(SESSION_ENV).ok().filter(|s| !s.is_empty())
    }

    async fn wait_web(
        question_id: &str,
    ) -> Option<anycode_dashboard::question_ipc::QuestionResponseRecord> {
        let deadline = tokio::time::Instant::now() + WEB_TIMEOUT;
        loop {
            if let Some(resp) = question_ipc::poll_response(question_id) {
                return Some(resp);
            }
            if tokio::time::Instant::now() >= deadline {
                question_ipc::clear_pending(question_id);
                return None;
            }
            tokio::time::sleep(Duration::from_millis(WEB_POLL_MS)).await;
        }
    }
}

#[async_trait]
impl AskUserQuestionHost for WorkbenchAskUserQuestionHost {
    async fn ask_user_question(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
        if request.options.is_empty() {
            return Err(AskUserQuestionHostError("no options".into()));
        }
        let Some(sid) = Self::session_id() else {
            return Err(AskUserQuestionHostError(
                "AskUserQuestion requires ANYCODE_DASHBOARD_SESSION_ID".into(),
            ));
        };
        if !question_ipc::web_questions_enabled() {
            return Err(AskUserQuestionHostError(
                "Web AskUserQuestion disabled (ANYCODE_DASHBOARD_WEB_QUESTION=0)".into(),
            ));
        }
        let options: Vec<QuestionOptionRecord> = request
            .options
            .iter()
            .map(|o| QuestionOptionRecord {
                label: o.label.clone(),
                description: o.description.clone(),
            })
            .collect();
        let question_id = question_ipc::register_pending(
            &sid,
            &request.question,
            &request.header,
            &options,
            request.multi_select,
        )
        .map_err(|e| AskUserQuestionHostError(e.to_string()))?;
        tracing::info!(
            target: "anycode_dashboard",
            session_id = %sid,
            question_id = %question_id,
            "AskUserQuestion pending — respond in dashboard"
        );
        let resp = Self::wait_web(&question_id)
            .await
            .ok_or_else(|| AskUserQuestionHostError("question timed out or cancelled".into()))?;
        let mut labels = resp.selected_labels;
        if let Some(other) = resp.other_text.filter(|t| !t.trim().is_empty()) {
            labels.push(other);
        }
        if labels.is_empty() {
            return Err(AskUserQuestionHostError("no selection".into()));
        }
        Ok(AskUserQuestionResponse {
            selected_labels: labels,
        })
    }
}
