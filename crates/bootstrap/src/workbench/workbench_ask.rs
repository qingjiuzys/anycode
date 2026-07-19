//! Dashboard Web `AskUserQuestion` host (file IPC poll).

use anycode_dashboard_ipc::approval_ipc::SESSION_ENV;
use anycode_dashboard_ipc::question_ipc::{self, QuestionOptionRecord};
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

    /// Task-local chat turn context first (embedded chat / triggers); env var
    /// only as legacy fallback for headless single-task CLI processes.
    fn session_id() -> Option<String> {
        anycode_core::current_dashboard_session_id()
            .or_else(|| std::env::var(SESSION_ENV).ok())
            .filter(|s| !s.is_empty())
    }

    fn user_turn_id() -> u32 {
        anycode_core::current_user_turn_id().unwrap_or_else(|| {
            std::env::var(question_ipc::USER_TURN_ENV)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        })
    }

    async fn wait_web(
        question_id: &str,
    ) -> Option<anycode_dashboard_ipc::question_ipc::QuestionResponseRecord> {
        let deadline = tokio::time::Instant::now() + WEB_TIMEOUT;
        let session_id = Self::session_id();
        loop {
            if let Some(resp) = question_ipc::poll_response(question_id) {
                return Some(resp);
            }
            if question_ipc::get_pending(question_id).is_none() {
                return None;
            }
            if session_id
                .as_deref()
                .is_some_and(anycode_dashboard_ipc::cancel_ipc::poll_cancel_requested)
            {
                question_ipc::clear_pending(question_id);
                return None;
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
            Self::user_turn_id(),
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
