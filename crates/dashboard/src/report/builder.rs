//! Report aggregation and snapshot construction from DB rows.

use crate::report::locale::{format_verdict, Lang};
use crate::report::snapshot::{event_rows, session_extra, ReportEventRow, ReportSnapshot};
use crate::schema::{
    ArtifactRecord, GateRecord, ProjectStatsFailure, ReportArtifactRow, ReportFailureGroup,
    ReportGateRow, ReportHighlights, ReportSessionRow, ReportSourceCounts, ReportSummary,
    SessionDetail, SessionSummary,
};
use std::collections::HashMap;

pub fn is_imported_session(title: &str) -> bool {
    title.starts_with("Imported task")
}

pub fn trust_counts(sessions: &[SessionSummary]) -> (i64, i64, i64) {
    let mut verified = 0i64;
    let mut unverified = 0i64;
    let mut blocked = 0i64;
    for s in sessions {
        match s.trusted_status.as_str() {
            "verified" => verified += 1,
            "blocked" => blocked += 1,
            _ => unverified += 1,
        }
    }
    (verified, unverified, blocked)
}

pub fn overall_trusted_status(
    failed_required_gates: i64,
    trust_blocked: i64,
    trust_unverified: i64,
) -> String {
    if failed_required_gates > 0 || trust_blocked > 0 {
        "blocked".into()
    } else if trust_unverified == 0 {
        "verified".into()
    } else {
        "unverified".into()
    }
}

pub fn partition_sessions(sessions: &[SessionSummary]) -> (Vec<ReportSessionRow>, i64) {
    let mut recent: Vec<&SessionSummary> = sessions
        .iter()
        .filter(|s| !is_imported_session(&s.title))
        .collect();
    recent.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let imported_count = sessions
        .iter()
        .filter(|s| is_imported_session(&s.title))
        .count() as i64;

    let rows = recent
        .into_iter()
        .take(8)
        .map(|s| ReportSessionRow {
            session_id: s.id.clone(),
            title: s.title.clone(),
            kind: s.kind.clone(),
            status: s.status.clone(),
            trusted_status: s.trusted_status.clone(),
            started_at: s.started_at.clone(),
            is_imported: false,
        })
        .collect();

    (rows, imported_count)
}

pub fn aggregate_failures(failures: &[ProjectStatsFailure]) -> Vec<ReportFailureGroup> {
    let mut map: HashMap<(String, String), ReportFailureGroup> = HashMap::new();
    for f in failures {
        let key = (f.title.clone(), f.event_type.clone());
        map.entry(key)
            .and_modify(|g| {
                g.count += 1;
                if f.occurred_at > g.last_at {
                    g.last_at = f.occurred_at.clone();
                }
                if g.session_id.is_none() {
                    g.session_id = f.session_id.clone();
                }
            })
            .or_insert(ReportFailureGroup {
                title: f.title.clone(),
                event_type: f.event_type.clone(),
                count: 1,
                last_at: f.occurred_at.clone(),
                session_id: f.session_id.clone(),
            });
    }
    let mut groups: Vec<_> = map.into_values().collect();
    groups.sort_by(|a, b| b.last_at.cmp(&a.last_at));
    groups.truncate(12);
    groups
}

pub fn gate_rows(gates: &[GateRecord]) -> Vec<ReportGateRow> {
    gates
        .iter()
        .map(|g| ReportGateRow {
            name: g.name.clone(),
            status: g.status.clone(),
            required: g.required,
            output_excerpt: g.output_excerpt.chars().take(120).collect(),
        })
        .collect()
}

pub fn artifact_rows(artifacts: &[ArtifactRecord]) -> Vec<ReportArtifactRow> {
    artifacts
        .iter()
        .take(20)
        .map(|a| ReportArtifactRow {
            path: a.path.clone(),
            kind: a.kind.clone(),
            trust_level: a.trust_level.clone(),
        })
        .collect()
}

pub struct ProjectReportBuildInput<'a> {
    pub lang: Lang,
    pub project_id: &'a str,
    pub project_name: &'a str,
    pub root_path: &'a str,
    pub generated_at: &'a str,
    pub sessions: &'a [SessionSummary],
    pub gates: &'a [GateRecord],
    pub artifacts: &'a [ArtifactRecord],
    pub recent_failures: &'a [ProjectStatsFailure],
    pub events_sample_limit: i64,
    pub events_sampled: i64,
}

pub fn build_project_snapshot(input: ProjectReportBuildInput<'_>) -> ReportSnapshot {
    let failed_gates = input
        .gates
        .iter()
        .filter(|g| g.required && g.status == "failed")
        .count() as i64;
    let (trust_verified, trust_unverified, trust_blocked) = trust_counts(input.sessions);
    let trusted_status = overall_trusted_status(failed_gates, trust_blocked, trust_unverified);
    let failure_groups = aggregate_failures(input.recent_failures);
    let (sessions_recent, sessions_imported_count) = partition_sessions(input.sessions);
    let gate_list = gate_rows(input.gates);
    let artifact_list = artifact_rows(input.artifacts);

    let highlights = ReportHighlights {
        trust_verified,
        trust_unverified,
        trust_blocked,
        failures_unique: failure_groups.len() as i64,
        verdict: format_verdict(
            input.lang,
            trust_verified,
            trust_unverified,
            trust_blocked,
            failed_gates,
        ),
    };

    let summary = ReportSummary {
        sessions: input.sessions.len() as i64,
        events: input.events_sampled,
        failed_gates,
        artifacts: input.artifacts.len() as i64,
    };
    let source_counts = ReportSourceCounts {
        sessions: summary.sessions,
        events: summary.events,
        gates: input.gates.len() as i64,
        artifacts: summary.artifacts,
    };

    ReportSnapshot {
        scope: "project".into(),
        id: input.project_id.to_string(),
        title: input.project_name.to_string(),
        lang: input.lang,
        generated_at: input.generated_at.to_string(),
        trusted_status,
        highlights,
        summary,
        source_counts,
        sessions_recent,
        sessions_imported_count,
        failure_groups,
        gates: gate_list,
        artifacts: artifact_list,
        events_sample_limit: input.events_sample_limit,
        project_id: Some(input.project_id.to_string()),
        root_path: Some(input.root_path.to_string()),
        prompt_preview: None,
        session_summary: None,
        recent_events: Vec::new(),
    }
}

pub struct SessionReportBuildInput<'a> {
    pub lang: Lang,
    pub session: &'a SessionDetail,
    pub gates: &'a [GateRecord],
    pub artifacts: &'a [ArtifactRecord],
    pub events: &'a [crate::schema::ProjectEvent],
    pub events_sample_limit: i64,
}

pub fn build_session_snapshot(input: SessionReportBuildInput<'_>) -> ReportSnapshot {
    let failed_gates = input
        .gates
        .iter()
        .filter(|g| g.required && g.status == "failed")
        .count() as i64;
    let trust_verified = if input.session.trusted_status == "verified" {
        1
    } else {
        0
    };
    let trust_unverified = if input.session.trusted_status == "unverified" {
        1
    } else {
        0
    };
    let trust_blocked = if input.session.trusted_status == "blocked" {
        1
    } else {
        0
    };

    let failure_groups: Vec<ReportFailureGroup> = input
        .events
        .iter()
        .filter(|e| e.severity == "error" || e.severity == "warn" || e.event_type.contains("fail"))
        .map(|e| ReportFailureGroup {
            title: e.title.clone(),
            event_type: e.event_type.clone(),
            count: 1,
            last_at: e.occurred_at.clone(),
            session_id: Some(input.session.id.clone()),
        })
        .collect();
    let failure_groups = aggregate_failures_from_groups(failure_groups);

    let highlights = ReportHighlights {
        trust_verified,
        trust_unverified,
        trust_blocked,
        failures_unique: failure_groups.len() as i64,
        verdict: format_verdict(
            input.lang,
            trust_verified,
            trust_unverified,
            trust_blocked,
            failed_gates,
        ),
    };

    let summary = ReportSummary {
        sessions: 1,
        events: input.events.len() as i64,
        failed_gates,
        artifacts: input.artifacts.len() as i64,
    };
    let source_counts = ReportSourceCounts {
        sessions: 1,
        events: summary.events,
        gates: input.gates.len() as i64,
        artifacts: summary.artifacts,
    };

    let gate_list = gate_rows(input.gates);
    let artifact_list = artifact_rows(input.artifacts);
    let sessions_recent = vec![ReportSessionRow {
        session_id: input.session.id.clone(),
        title: input.session.title.clone(),
        kind: input.session.kind.clone(),
        status: input.session.status.clone(),
        trusted_status: input.session.trusted_status.clone(),
        started_at: input.session.started_at.clone(),
        is_imported: is_imported_session(&input.session.title),
    }];

    let (prompt_preview, session_summary) = session_extra(input.session);
    let recent_events: Vec<ReportEventRow> = event_rows(input.events);

    ReportSnapshot {
        scope: "session".into(),
        id: input.session.id.clone(),
        title: input.session.title.clone(),
        lang: input.lang,
        generated_at: chrono::Utc::now().to_rfc3339(),
        trusted_status: input.session.trusted_status.clone(),
        highlights,
        summary,
        source_counts,
        sessions_recent,
        sessions_imported_count: 0,
        failure_groups,
        gates: gate_list,
        artifacts: artifact_list,
        events_sample_limit: input.events_sample_limit,
        project_id: Some(input.session.project_id.clone()),
        root_path: None,
        prompt_preview,
        session_summary,
        recent_events,
    }
}

fn aggregate_failures_from_groups(groups: Vec<ReportFailureGroup>) -> Vec<ReportFailureGroup> {
    let pseudo: Vec<ProjectStatsFailure> = groups
        .into_iter()
        .map(|g| ProjectStatsFailure {
            id: String::new(),
            title: g.title,
            event_type: g.event_type,
            occurred_at: g.last_at,
            session_id: g.session_id,
        })
        .collect();
    aggregate_failures(&pseudo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(title: &str, trusted: &str, started: &str) -> SessionSummary {
        SessionSummary {
            id: "s1".into(),
            kind: "repl".into(),
            task_id: None,
            title: title.into(),
            status: "completed".into(),
            trusted_status: trusted.into(),
            agent_type: "general-purpose".into(),
            model: "m".into(),
            started_at: started.into(),
            ended_at: None,
            block_reason: None,
            block_kind: None,
        }
    }

    #[test]
    fn imported_sessions_partitioned() {
        let sessions = vec![
            session("Imported task abc", "unverified", "2026-06-01"),
            session("Real work", "verified", "2026-06-03"),
        ];
        let (recent, imported) = partition_sessions(&sessions);
        assert_eq!(imported, 1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title, "Real work");
    }

    #[test]
    fn trust_not_verified_when_many_unverified() {
        let sessions = vec![
            session("a", "verified", "1"),
            session("b", "unverified", "2"),
        ];
        let (v, u, b) = trust_counts(&sessions);
        assert_eq!((v, u, b), (1, 1, 0));
        assert_eq!(overall_trusted_status(0, b, u), "unverified");
    }

    #[test]
    fn failures_aggregate() {
        let failures = vec![
            ProjectStatsFailure {
                id: "1".into(),
                title: "Bash failed".into(),
                event_type: "tool_call_end".into(),
                occurred_at: "2026-05-21 20:23:47".into(),
                session_id: None,
            },
            ProjectStatsFailure {
                id: "2".into(),
                title: "Bash failed".into(),
                event_type: "tool_call_end".into(),
                occurred_at: "2026-05-21 20:23:46".into(),
                session_id: None,
            },
        ];
        let g = aggregate_failures(&failures);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].count, 2);
    }
}
