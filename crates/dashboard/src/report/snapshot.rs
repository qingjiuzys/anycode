//! Structured report data from DB (facts only, no rendered bodies).

use crate::report::locale::Lang;
use crate::schema::{
    ProjectEvent, ReportArtifactRow, ReportFailureGroup, ReportGateRow, ReportHighlights,
    ReportSessionRow, ReportSourceCounts, ReportSummary, SessionDetail,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ReportSnapshot {
    pub scope: String,
    pub id: String,
    pub title: String,
    pub lang: Lang,
    pub generated_at: String,
    pub trusted_status: String,
    pub highlights: ReportHighlights,
    pub summary: ReportSummary,
    pub source_counts: ReportSourceCounts,
    pub sessions_recent: Vec<ReportSessionRow>,
    pub sessions_imported_count: i64,
    pub failure_groups: Vec<ReportFailureGroup>,
    pub gates: Vec<ReportGateRow>,
    pub artifacts: Vec<ReportArtifactRow>,
    pub events_sample_limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_summary: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_events: Vec<ReportEventRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportEventRow {
    pub title: String,
    pub event_type: String,
    pub severity: String,
    pub occurred_at: String,
}

impl ReportSnapshot {
    pub fn lang_code(&self) -> String {
        match self.lang {
            Lang::Zh => "zh".into(),
            Lang::En => "en".into(),
        }
    }
}

pub fn event_rows(events: &[ProjectEvent]) -> Vec<ReportEventRow> {
    events
        .iter()
        .rev()
        .take(30)
        .map(|e| ReportEventRow {
            title: e.title.clone(),
            event_type: e.event_type.clone(),
            severity: e.severity.clone(),
            occurred_at: e.occurred_at.clone(),
        })
        .collect()
}

pub fn session_extra(session: &SessionDetail) -> (Option<String>, Option<String>) {
    let prompt = if session.prompt_preview.is_empty() {
        None
    } else {
        Some(session.prompt_preview.clone())
    };
    let summary = if session.summary.is_empty() {
        None
    } else {
        Some(session.summary.clone())
    };
    (prompt, summary)
}
