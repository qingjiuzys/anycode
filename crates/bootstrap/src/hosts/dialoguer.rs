//! Blocking dialoguer prompts on stderr (optional; feature `dialoguer-host`).

use anycode_tools::{
    AskUserQuestionHost, AskUserQuestionHostError, AskUserQuestionRequest, AskUserQuestionResponse,
};
use async_trait::async_trait;

pub struct DialoguerAskUserQuestionHost;

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
