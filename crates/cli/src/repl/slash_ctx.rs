//! 斜杠补全上下文（底栏与键盘事件共用）。

use crate::repl::line_state::ReplLineState;
use crate::slash_commands;
use crate::tui::input::InputState;

pub(crate) fn cursor_on_first_line(input: &InputState) -> bool {
    !input.chars[..input.cursor].contains(&'\n')
}

pub(crate) fn slash_suggestions_for_ctx(
    state: &ReplLineState,
) -> Vec<slash_commands::SlashSuggestionItem> {
    if state.slash_suppress {
        return Vec::new();
    }
    slash_commands::slash_suggestions_for_first_line(&state.input.as_string())
}

pub(crate) fn apply_slash_pick_to_input(state: &mut ReplLineState) {
    let cands = slash_commands::slash_suggestions_for_first_line(&state.input.as_string());
    if cands.is_empty() {
        return;
    }
    let len = cands.len();
    let pick = state.slash_pick % len;
    let new_first = cands[pick].replacement.clone();
    let new_buf = slash_commands::replace_first_line(&state.input.as_string(), &new_first);
    state.input.set_from_str(&new_buf);
    state.slash_pick = 0;
    state.history_idx = None;
}
