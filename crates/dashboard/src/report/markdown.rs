//! Template Markdown export (tables, minimal emphasis).

use crate::report::locale::{
    label_session_kind, label_session_status, label_trust_status, strings, Lang,
};
use crate::report::snapshot::ReportSnapshot;

pub fn render_snapshot_markdown(snap: &ReportSnapshot) -> String {
    if snap.scope == "session" {
        render_session_markdown(snap)
    } else {
        render_project_markdown(snap)
    }
}

fn meta_table_header(lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => "| 字段 | 值 |\n| --- | --- |\n",
        Lang::En => "| Field | Value |\n| --- | --- |\n",
    }
}

fn metric_table_header(lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => "| 指标 | 数值 |\n| --- | ---: |\n",
        Lang::En => "| Metric | Value |\n| --- | ---: |\n",
    }
}

fn render_project_markdown(snap: &ReportSnapshot) -> String {
    let s = strings(snap.lang);
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", s.doc_title));
    md.push_str(meta_table_header(snap.lang));
    md.push_str(&format!("| {} | {} |\n", s.label_project, snap.title));
    if let Some(ref pid) = snap.project_id {
        md.push_str(&format!("| ID | `{}` |\n", pid));
    }
    if let Some(ref root) = snap.root_path {
        md.push_str(&format!("| {} | `{}` |\n", s.label_root, root));
    }
    md.push_str(&format!(
        "| {} | {} |\n",
        s.label_generated, snap.generated_at
    ));
    md.push_str(&format!(
        "| {} | {} (`{}`) |\n\n",
        s.summary_trusted,
        label_trust_status(snap.lang, &snap.trusted_status),
        snap.trusted_status
    ));

    md.push_str(&format!("## {}\n\n", s.section_summary));
    md.push_str(&format!("{}\n\n", snap.highlights.verdict));
    md.push_str(metric_table_header(snap.lang));
    md.push_str(&format!(
        "| {} | {} |\n",
        s.summary_sessions, snap.summary.sessions
    ));
    md.push_str(&format!(
        "| {} | {} (cap {}) |\n",
        s.summary_events_sampled, snap.summary.events, snap.events_sample_limit
    ));
    md.push_str(&format!(
        "| {} | {} |\n",
        s.summary_failed_gates, snap.summary.failed_gates
    ));
    md.push_str(&format!(
        "| {} | {} |\n\n",
        s.summary_artifacts, snap.summary.artifacts
    ));

    md.push_str(&format!("## {}\n\n", s.section_trust));
    let breakdown = match snap.lang {
        Lang::Zh => "会话分布",
        Lang::En => "Session breakdown",
    };
    let trust_hdr = match snap.lang {
        Lang::Zh => "| 状态 | 数量 |\n| --- | ---: |\n",
        Lang::En => "| Status | Count |\n| --- | ---: |\n",
    };
    md.push_str(trust_hdr);
    md.push_str(&format!(
        "| {} | {} |\n",
        label_trust_status(snap.lang, "verified"),
        snap.highlights.trust_verified
    ));
    md.push_str(&format!(
        "| {} | {} |\n",
        label_trust_status(snap.lang, "unverified"),
        snap.highlights.trust_unverified
    ));
    md.push_str(&format!(
        "| {} | {} |\n\n",
        label_trust_status(snap.lang, "blocked"),
        snap.highlights.trust_blocked
    ));
    let _ = breakdown;

    append_sessions_section(&mut md, snap);
    append_gates_section(&mut md, snap);
    append_failures_section(&mut md, snap);
    append_artifacts_section(&mut md, snap);
    append_reproduce_project(&mut md, snap);
    md
}

fn render_session_markdown(snap: &ReportSnapshot) -> String {
    let s = strings(snap.lang);
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", s.doc_title));
    md.push_str(meta_table_header(snap.lang));
    md.push_str(&format!("| {} | {} |\n", s.label_session, snap.title));
    md.push_str(&format!("| ID | `{}` |\n", snap.id));
    if let Some(ref pid) = snap.project_id {
        md.push_str(&format!("| {} | {} |\n", s.label_project, pid));
    }
    md.push_str(&format!(
        "| {} | {} |\n\n",
        s.label_generated, snap.generated_at
    ));

    md.push_str(&format!("## {}\n\n", s.section_summary));
    md.push_str(&format!("{}\n\n", snap.highlights.verdict));
    md.push_str(metric_table_header(snap.lang));
    md.push_str(&format!(
        "| {} | {} |\n",
        s.summary_trusted,
        label_trust_status(snap.lang, &snap.trusted_status)
    ));
    md.push_str(&format!(
        "| {} | {} (cap {}) |\n",
        s.summary_events_sampled, snap.summary.events, snap.events_sample_limit
    ));
    md.push_str(&format!(
        "| {} | {} |\n",
        s.summary_failed_gates, snap.summary.failed_gates
    ));
    md.push_str(&format!(
        "| {} | {} |\n\n",
        s.summary_artifacts, snap.summary.artifacts
    ));

    if let Some(ref p) = snap.prompt_preview {
        md.push_str(&format!("## {}\n\n```\n{}\n```\n\n", s.section_prompt, p));
    }
    if let Some(ref sum) = snap.session_summary {
        md.push_str(&format!("## {}\n\n{}\n\n", s.section_summary_text, sum));
    }

    append_gates_section(&mut md, snap);
    append_events_section(&mut md, snap);
    append_failures_section(&mut md, snap);
    append_artifacts_section(&mut md, snap);
    append_reproduce_session(&mut md, snap);
    md
}

fn append_sessions_section(md: &mut String, snap: &ReportSnapshot) {
    let s = strings(snap.lang);
    md.push_str(&format!("## {}\n\n", s.section_sessions));
    if snap.sessions_recent.is_empty() && snap.sessions_imported_count == 0 {
        md.push_str(&format!("{}\n\n", s.no_sessions));
        return;
    }
    let hdr = match snap.lang {
        Lang::Zh => "| 标题 | 类型 | 状态 | 信任 | 会话 ID | 开始时间 |\n| --- | --- | --- | --- | --- | --- |\n",
        Lang::En => "| Title | Kind | Status | Trust | Session ID | Started |\n| --- | --- | --- | --- | --- | --- |\n",
    };
    md.push_str(hdr);
    for row in &snap.sessions_recent {
        md.push_str(&format!(
            "| {} | {} | {} | {} | `{}` | {} |\n",
            row.title,
            label_session_kind(snap.lang, &row.kind),
            label_session_status(snap.lang, &row.status),
            label_trust_status(snap.lang, &row.trusted_status),
            row.session_id,
            row.started_at
        ));
    }
    if snap.sessions_imported_count > 0 {
        let msg = s
            .imported_collapsed
            .replace("{n}", &snap.sessions_imported_count.to_string());
        md.push_str(&format!("\n_{msg}_\n\n"));
    } else {
        md.push('\n');
    }
}

fn append_gates_section(md: &mut String, snap: &ReportSnapshot) {
    let s = strings(snap.lang);
    if snap.gates.is_empty() {
        return;
    }
    md.push_str(&format!("## {}\n\n", s.section_gates));
    let hdr = match snap.lang {
        Lang::Zh => "| 名称 | 状态 | 必需 | 输出摘要 |\n| --- | --- | --- | --- |\n",
        Lang::En => "| Name | Status | Required | Output |\n| --- | --- | --- | --- |\n",
    };
    md.push_str(hdr);
    for g in &snap.gates {
        let req = if g.required {
            s.gate_required
        } else {
            s.gate_optional
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            g.name, g.status, req, g.output_excerpt
        ));
    }
    md.push('\n');
}

fn append_failures_section(md: &mut String, snap: &ReportSnapshot) {
    let s = strings(snap.lang);
    md.push_str(&format!("## {}\n\n", s.section_failures));
    if snap.failure_groups.is_empty() {
        md.push_str(&format!("{}\n\n", s.no_failures));
        return;
    }
    let hdr = match snap.lang {
        Lang::Zh => "| 标题 | 类型 | 次数 | 最近时间 | 会话 |\n| --- | --- | ---: | --- | --- |\n",
        Lang::En => "| Title | Type | Count | Last | Session |\n| --- | --- | ---: | --- | --- |\n",
    };
    md.push_str(hdr);
    for g in &snap.failure_groups {
        let sid = g.session_id.as_deref().unwrap_or("—");
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            g.title,
            g.event_type,
            g.count,
            format_time_short(&g.last_at),
            sid
        ));
    }
    md.push('\n');
}

fn append_artifacts_section(md: &mut String, snap: &ReportSnapshot) {
    let s = strings(snap.lang);
    if snap.artifacts.is_empty() {
        return;
    }
    md.push_str(&format!("## {}\n\n", s.section_artifacts));
    let hdr = match snap.lang {
        Lang::Zh => "| 路径 | 类型 | 信任 |\n| --- | --- | --- |\n",
        Lang::En => "| Path | Kind | Trust |\n| --- | --- | --- |\n",
    };
    md.push_str(hdr);
    for a in &snap.artifacts {
        md.push_str(&format!(
            "| `{}` | {} | {} |\n",
            a.path, a.kind, a.trust_level
        ));
    }
    md.push('\n');
}

fn append_events_section(md: &mut String, snap: &ReportSnapshot) {
    let s = strings(snap.lang);
    md.push_str(&format!("## {}\n\n", s.section_events));
    if snap.recent_events.is_empty() {
        md.push_str(&format!("{}\n\n", s.no_events));
        return;
    }
    let hdr = match snap.lang {
        Lang::Zh => "| 标题 | 类型 | 级别 | 时间 |\n| --- | --- | --- | --- |\n",
        Lang::En => "| Title | Type | Severity | Time |\n| --- | --- | --- | --- |\n",
    };
    md.push_str(hdr);
    for e in &snap.recent_events {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            e.title,
            e.event_type,
            e.severity,
            format_time_short(&e.occurred_at)
        ));
    }
    md.push('\n');
}

fn append_reproduce_project(md: &mut String, snap: &ReportSnapshot) {
    let s = strings(snap.lang);
    md.push_str(&format!("## {}\n\n", s.section_reproduce));
    md.push_str(&format!("{}\n\n", s.reproduce_hint));
    let root = snap.root_path.as_deref().unwrap_or(".");
    let pid = snap.project_id.as_deref().unwrap_or("");
    md.push_str(&format!(
        "```bash\ncd \"{}\"\nanycode dashboard --open\n# project_id: {}\n```\n\n",
        root, pid
    ));
}

fn append_reproduce_session(md: &mut String, snap: &ReportSnapshot) {
    let s = strings(snap.lang);
    md.push_str(&format!("## {}\n\n", s.section_reproduce));
    md.push_str(&format!("{}\n\n", s.reproduce_hint));
    md.push_str("```bash\nanycode dashboard --open\n");
    md.push_str(&format!("# session_id: {}\n```\n\n", snap.id));
}

fn format_time_short(raw: &str) -> String {
    let normalized = raw.replace('T', " ");
    normalized.chars().take(19).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::locale::Lang;
    use crate::schema::{ReportArtifactRow, ReportHighlights, ReportSourceCounts, ReportSummary};

    fn minimal_snap() -> ReportSnapshot {
        ReportSnapshot {
            scope: "project".into(),
            id: "p1".into(),
            title: "Demo".into(),
            lang: Lang::En,
            generated_at: "2026-06-04T00:00:00Z".into(),
            trusted_status: "verified".into(),
            highlights: ReportHighlights {
                trust_verified: 1,
                trust_unverified: 0,
                trust_blocked: 0,
                failures_unique: 0,
                verdict: "Required gate failures: 0; blocked sessions: 0.".into(),
            },
            summary: ReportSummary {
                sessions: 1,
                events: 2,
                failed_gates: 0,
                artifacts: 0,
            },
            source_counts: ReportSourceCounts {
                sessions: 1,
                events: 2,
                gates: 0,
                artifacts: 0,
            },
            sessions_recent: vec![],
            sessions_imported_count: 0,
            failure_groups: vec![],
            gates: vec![],
            artifacts: vec![],
            events_sample_limit: 50,
            project_id: Some("p1".into()),
            root_path: Some("/tmp/demo".into()),
            prompt_preview: None,
            session_summary: None,
            recent_events: vec![],
        }
    }

    #[test]
    fn markdown_avoids_label_bold_pattern() {
        let md = render_snapshot_markdown(&minimal_snap());
        assert!(!md.contains("**Project:**"));
        assert!(md.contains("| Field | Value |"));
    }
}
