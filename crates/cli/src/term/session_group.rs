//! Stream REPL transcript section headers and dock labels for session / cron correlation ids.

use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::repl::ReplLineState;

/// Active correlation id for grouping and dock display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamSessionCorrelation {
    pub id: String,
    pub is_cron: bool,
}

/// `ANYCODE_CRON_SESSION_ID` when set; otherwise persisted REPL session file id.
pub(crate) fn resolve_stream_session_correlation(
    session_file_id: Uuid,
) -> StreamSessionCorrelation {
    if let Ok(id) = std::env::var("ANYCODE_CRON_SESSION_ID") {
        let t = id.trim();
        if !t.is_empty() {
            return StreamSessionCorrelation {
                id: t.to_string(),
                is_cron: true,
            };
        }
    }
    StreamSessionCorrelation {
        id: session_file_id.to_string(),
        is_cron: false,
    }
}

/// Short stable label for footers (first 8 chars of uuid or opaque id).
pub(crate) fn short_correlation_id(id: &str) -> String {
    let t = id.trim();
    if t.len() <= 8 {
        t.to_string()
    } else {
        t.chars().take(8).collect()
    }
}

/// Plain-text divider inserted into stream transcript before a new correlation group.
pub(crate) fn session_section_header_line(correlation: &StreamSessionCorrelation) -> String {
    let short = short_correlation_id(&correlation.id);
    let kind = if correlation.is_cron {
        "cron"
    } else {
        "session"
    };
    format!("── {kind} · {short} ──")
}

/// Left footer fragment: `cron abc12345` or `sess abc12345`.
pub(crate) fn dock_correlation_fragment(correlation: &StreamSessionCorrelation) -> String {
    let short = short_correlation_id(&correlation.id);
    let kind = if correlation.is_cron { "cron" } else { "sess" };
    format!("{kind} {short}")
}

/// Store active correlation on line state (dock); does not write transcript.
pub(crate) fn set_stream_session_correlation(
    state: &Arc<Mutex<ReplLineState>>,
    correlation: StreamSessionCorrelation,
) {
    if let Ok(mut st) = state.lock() {
        st.stream_session_correlation = Some(correlation.id);
        st.stream_session_is_cron = correlation.is_cron;
    }
}

/// Append a section header when correlation changes; no-op if same as last header in this REPL.
pub(crate) fn maybe_append_stream_session_header(
    state: &Arc<Mutex<ReplLineState>>,
    correlation: &StreamSessionCorrelation,
) {
    let should_append = {
        let Ok(st) = state.lock() else {
            return;
        };
        st.stream_last_section_correlation.as_deref() != Some(correlation.id.as_str())
    };
    if !should_append {
        return;
    }
    let line = session_section_header_line(correlation);
    if let Ok(mut st) = state.lock() {
        st.stream_last_section_correlation = Some(correlation.id.clone());
        st.stream_session_correlation = Some(correlation.id.clone());
        st.stream_session_is_cron = correlation.is_cron;
        if let Ok(mut t) = st.transcript.lock() {
            if !t.is_empty() && !t.ends_with('\n') {
                t.push('\n');
            }
            if !t.is_empty() {
                t.push('\n');
            }
            t.push_str(&line);
            t.push('\n');
        }
    }
}

pub(crate) fn clear_stream_session_grouping(state: &Arc<Mutex<ReplLineState>>) {
    if let Ok(mut st) = state.lock() {
        st.stream_last_section_correlation = None;
        st.stream_session_correlation = None;
        st.stream_session_is_cron = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_header_uses_cron_kind() {
        let c = StreamSessionCorrelation {
            id: "sess-a1b2c3d4-e5f6-7890-abcd-ef1234567890".into(),
            is_cron: true,
        };
        assert_eq!(session_section_header_line(&c), "── cron · sess-a1b ──");
    }

    #[test]
    fn section_header_uses_session_kind() {
        let c = StreamSessionCorrelation {
            id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
            is_cron: false,
        };
        assert_eq!(session_section_header_line(&c), "── session · aaaaaaaa ──");
    }

    #[test]
    fn dock_fragment_short_id() {
        let c = StreamSessionCorrelation {
            id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
            is_cron: false,
        };
        assert_eq!(dock_correlation_fragment(&c), "sess aaaaaaaa");
    }

    #[test]
    fn maybe_append_writes_header_once() {
        let state = Arc::new(Mutex::new(ReplLineState::default()));
        let c = StreamSessionCorrelation {
            id: "cron-sess-1".into(),
            is_cron: true,
        };
        maybe_append_stream_session_header(&state, &c);
        let t1 = state.lock().unwrap().transcript.lock().unwrap().clone();
        assert!(t1.contains("── cron · cron-ses ──"));
        maybe_append_stream_session_header(&state, &c);
        let t2 = state.lock().unwrap().transcript.lock().unwrap().clone();
        assert_eq!(t1, t2);
    }
}
