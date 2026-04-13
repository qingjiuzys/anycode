//! `AskUserQuestionHost` implementations: dialoguer (TTY) and mpsc (stream REPL / fullscreen TUI).

use anycode_tools::{
    AskUserQuestionHost, AskUserQuestionHostError, AskUserQuestionRequest, AskUserQuestionResponse,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Blocking dialoguer prompts on stderr (for `run` / non-UI REPL when stdin+stdout are TTY).
pub(crate) struct DialoguerAskUserQuestionHost;

#[async_trait]
impl AskUserQuestionHost for DialoguerAskUserQuestionHost {
    async fn ask_user_question(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
        let labels: Vec<String> = request.options.iter().map(|o| o.label.clone()).collect();
        let descs: Vec<String> = request
            .options
            .iter()
            .map(|o| o.description.clone())
            .collect();
        let multi = request.multi_select;
        let header = request.header;
        let question = request.question;
        tokio::task::spawn_blocking(move || dialoguer_pick(header, question, labels, descs, multi))
            .await
            .map_err(|e| AskUserQuestionHostError(format!("dialoguer join: {e}")))?
    }
}

fn dialoguer_pick(
    header: String,
    question: String,
    labels: Vec<String>,
    descriptions: Vec<String>,
    multi_select: bool,
) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
    use console::Term;
    use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};

    if labels.is_empty() {
        return Err(AskUserQuestionHostError("no options".into()));
    }
    let theme = ColorfulTheme::default();
    let prompt = {
        let h = header.trim();
        let q = question.trim();
        if h.is_empty() {
            q.to_string()
        } else if q.is_empty() {
            h.to_string()
        } else {
            format!("{h}\n{q}")
        }
    };
    let stderr = Term::stderr();
    let items: Vec<String> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let d = descriptions.get(i).map(|s| s.as_str()).unwrap_or("").trim();
            if d.is_empty() {
                l.clone()
            } else {
                format!("{l} — {d}")
            }
        })
        .collect();

    if multi_select {
        let defaults = vec![false; items.len()];
        let picked = MultiSelect::with_theme(&theme)
            .with_prompt(&prompt)
            .items(&items)
            .defaults(&defaults)
            .interact_on(&stderr)
            .map_err(|e| AskUserQuestionHostError(e.to_string()))?;
        if picked.is_empty() {
            return Err(AskUserQuestionHostError("cancelled".into()));
        }
        let selected: Vec<String> = picked.into_iter().map(|i| labels[i].clone()).collect();
        Ok(AskUserQuestionResponse {
            selected_labels: selected,
        })
    } else {
        let sel = Select::with_theme(&theme)
            .with_prompt(&prompt)
            .items(&items)
            .default(0)
            .interact_on(&stderr)
            .map_err(|e| AskUserQuestionHostError(e.to_string()))?;
        Ok(AskUserQuestionResponse {
            selected_labels: vec![labels[sel].clone()],
        })
    }
}

/// Sends [`crate::tui::PendingUserQuestion`] to the UI loop (stream REPL or fullscreen TUI).
pub(crate) struct ChannelAskUserQuestionHost {
    tx: tokio::sync::mpsc::Sender<crate::tui::PendingUserQuestion>,
}

impl ChannelAskUserQuestionHost {
    pub(crate) fn new(tx: tokio::sync::mpsc::Sender<crate::tui::PendingUserQuestion>) -> Self {
        Self { tx }
    }

    pub(crate) fn into_arc(self) -> Arc<dyn AskUserQuestionHost> {
        Arc::new(self) as Arc<dyn AskUserQuestionHost>
    }
}

#[async_trait]
impl AskUserQuestionHost for ChannelAskUserQuestionHost {
    async fn ask_user_question(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
        if request.multi_select {
            return Err(AskUserQuestionHostError(
                "multi_select is not supported in TUI/stream REPL UI; use plain TTY or set multiSelect false"
                    .into(),
            ));
        }
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let option_labels: Vec<_> = request.options.iter().map(|o| o.label.clone()).collect();
        let option_descriptions: Vec<_> = request
            .options
            .iter()
            .map(|o| o.description.clone())
            .collect();
        let pending = crate::tui::PendingUserQuestion {
            header: request.header,
            question: request.question,
            option_labels,
            option_descriptions,
            multi_select: request.multi_select,
            reply: reply_tx,
        };
        self.tx
            .send(pending)
            .await
            .map_err(|_| AskUserQuestionHostError(crate::i18n::tr("ask-user-tui-exited")))?;
        match reply_rx.await {
            Ok(Ok(labels)) => {
                if labels.is_empty() {
                    Err(AskUserQuestionHostError(crate::i18n::tr(
                        "ask-user-empty-selection",
                    )))
                } else {
                    Ok(AskUserQuestionResponse {
                        selected_labels: labels,
                    })
                }
            }
            Ok(Err(())) => Err(AskUserQuestionHostError(crate::i18n::tr(
                "ask-user-cancelled",
            ))),
            Err(_) => Err(AskUserQuestionHostError(crate::i18n::tr(
                "ask-user-tui-exited",
            ))),
        }
    }
}
