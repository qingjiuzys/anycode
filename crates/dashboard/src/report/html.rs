//! Self-contained HTML report (inline CSS, print-friendly).

use crate::report::locale::{
    label_session_kind, label_session_status, label_trust_status, strings, Lang,
};
use crate::report::snapshot::ReportSnapshot;

pub fn render_snapshot_html(snap: &ReportSnapshot) -> String {
    let body = render_body(snap);
    wrap_document(&snap.title, &body)
}

pub fn wrap_document(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>{title}</title>
<style>
:root {{ font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; color: #1a1a1a; }}
body {{ margin: 0; padding: 24px 32px; max-width: 960px; line-height: 1.5; font-size: 14px; }}
h1 {{ font-size: 1.35rem; font-weight: 600; margin: 0 0 8px; }}
h2 {{ font-size: 1rem; font-weight: 600; margin: 24px 0 8px; border-bottom: 1px solid #e0e0e0; padding-bottom: 4px; }}
.meta {{ color: #555; font-size: 13px; margin-bottom: 20px; }}
.verdict {{ background: #f4f6f8; border-left: 3px solid #5c6bc0; padding: 12px 14px; margin: 12px 0 20px; }}
.kpi-row {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 10px; margin: 12px 0 20px; }}
.kpi {{ background: #fafafa; border: 1px solid #e8e8e8; border-radius: 6px; padding: 10px; text-align: center; }}
.kpi .v {{ font-size: 1.25rem; font-weight: 600; }}
.kpi .l {{ font-size: 11px; color: #666; text-transform: uppercase; letter-spacing: 0.03em; }}
table {{ width: 100%; border-collapse: collapse; margin: 8px 0 16px; font-size: 13px; }}
th, td {{ border: 1px solid #e0e0e0; padding: 6px 10px; text-align: left; }}
th {{ background: #f5f5f5; font-weight: 600; }}
tr:nth-child(even) td {{ background: #fafafa; }}
.chip {{ display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 12px; font-weight: 500; }}
.chip-verified {{ background: #e8f5e9; color: #2e7d32; }}
.chip-unverified {{ background: #fff8e1; color: #f57f17; }}
.chip-blocked {{ background: #ffebee; color: #c62828; }}
.chip-default {{ background: #eceff1; color: #455a64; }}
pre {{ background: #f5f5f5; padding: 10px; overflow-x: auto; font-size: 12px; border-radius: 4px; }}
@media print {{ body {{ padding: 12px; }} }}
</style>
</head>
<body>
{body}
</body>
</html>"#
    )
}

pub fn wrap_fragment(body: &str, title: &str) -> String {
    wrap_document(title, body)
}

fn render_body(snap: &ReportSnapshot) -> String {
    let s = strings(snap.lang);
    let mut out = String::new();
    out.push_str(&format!("<h1>{}</h1>\n", escape_html(&s.doc_title)));
    out.push_str("<div class=\"meta\">");
    if snap.scope == "project" {
        out.push_str(&format!(
            "<div>{}: {}</div>",
            escape_html(s.label_project),
            escape_html(&snap.title)
        ));
        if let Some(ref root) = snap.root_path {
            out.push_str(&format!(
                "<div>{}: <code>{}</code></div>",
                escape_html(s.label_root),
                escape_html(root)
            ));
        }
    } else {
        out.push_str(&format!(
            "<div>{}: {} (<code>{}</code>)</div>",
            escape_html(s.label_session),
            escape_html(&snap.title),
            escape_html(&snap.id)
        ));
    }
    out.push_str(&format!(
        "<div>{}: {}</div>",
        escape_html(s.label_generated),
        escape_html(&snap.generated_at)
    ));
    out.push_str(&format!(
        "<div>{}: <span class=\"chip {}\">{}</span></div>",
        escape_html(s.summary_trusted),
        trust_chip_class(&snap.trusted_status),
        escape_html(&label_trust_status(snap.lang, &snap.trusted_status))
    ));
    out.push_str("</div>\n");

    out.push_str(&format!("<h2>{}</h2>\n", escape_html(s.section_summary)));
    out.push_str(&format!(
        "<div class=\"verdict\">{}</div>\n",
        escape_html(&snap.highlights.verdict)
    ));
    out.push_str("<div class=\"kpi-row\">");
    out.push_str(&kpi_cell(
        &s.summary_sessions,
        snap.summary.sessions.to_string(),
    ));
    out.push_str(&kpi_cell(
        &s.summary_events_sampled,
        format!("{} / {}", snap.summary.events, snap.events_sample_limit),
    ));
    out.push_str(&kpi_cell(
        &s.summary_failed_gates,
        snap.summary.failed_gates.to_string(),
    ));
    out.push_str(&kpi_cell(
        &s.summary_artifacts,
        snap.summary.artifacts.to_string(),
    ));
    out.push_str("</div>\n");

    if !snap.sessions_recent.is_empty() || snap.sessions_imported_count > 0 {
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(s.section_sessions)));
        if !snap.sessions_recent.is_empty() {
            out.push_str("<table><thead><tr>");
            let cols = session_cols(snap.lang);
            for c in cols {
                out.push_str(&format!("<th>{}</th>", escape_html(c)));
            }
            out.push_str("</tr></thead><tbody>");
            for row in &snap.sessions_recent {
                out.push_str("<tr>");
                out.push_str(&format!("<td>{}</td>", escape_html(&row.title)));
                out.push_str(&format!(
                    "<td>{}</td>",
                    escape_html(&label_session_kind(snap.lang, &row.kind))
                ));
                out.push_str(&format!(
                    "<td><span class=\"chip chip-default\">{}</span></td>",
                    escape_html(&label_session_status(snap.lang, &row.status))
                ));
                out.push_str(&format!(
                    "<td><span class=\"chip {}\">{}</span></td>",
                    trust_chip_class(&row.trusted_status),
                    escape_html(&label_trust_status(snap.lang, &row.trusted_status))
                ));
                out.push_str(&format!(
                    "<td><code>{}</code></td>",
                    escape_html(&row.session_id)
                ));
                out.push_str(&format!("<td>{}</td>", escape_html(&row.started_at)));
                out.push_str("</tr>");
            }
            out.push_str("</tbody></table>\n");
        }
        if snap.sessions_imported_count > 0 {
            let msg = s
                .imported_collapsed
                .replace("{n}", &snap.sessions_imported_count.to_string());
            out.push_str(&format!("<p><em>{}</em></p>\n", escape_html(&msg)));
        }
    }

    if !snap.gates.is_empty() {
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(s.section_gates)));
        out.push_str("<table><thead><tr><th>Name</th><th>Status</th><th>Required</th><th>Output</th></tr></thead><tbody>");
        for g in &snap.gates {
            let req = if g.required {
                s.gate_required
            } else {
                s.gate_optional
            };
            out.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td></tr>",
                escape_html(&g.name),
                escape_html(&g.status),
                escape_html(req),
                escape_html(&g.output_excerpt)
            ));
        }
        out.push_str("</tbody></table>\n");
    }

    if !snap.failure_groups.is_empty() {
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(s.section_failures)));
        out.push_str("<table><thead><tr><th>Title</th><th>Type</th><th>Count</th><th>Last</th></tr></thead><tbody>");
        for g in &snap.failure_groups {
            out.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&g.title),
                escape_html(&g.event_type),
                g.count,
                escape_html(&g.last_at)
            ));
        }
        out.push_str("</tbody></table>\n");
    } else if snap.scope == "project" {
        out.push_str(&format!(
            "<h2>{}</h2>\n<p>{}</p>\n",
            escape_html(s.section_failures),
            escape_html(s.no_failures)
        ));
    }

    if !snap.artifacts.is_empty() {
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(s.section_artifacts)));
        out.push_str(
            "<table><thead><tr><th>Path</th><th>Kind</th><th>Trust</th></tr></thead><tbody>",
        );
        for a in &snap.artifacts {
            out.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                escape_html(&a.path),
                escape_html(&a.kind),
                escape_html(&a.trust_level)
            ));
        }
        out.push_str("</tbody></table>\n");
    }

    if !snap.recent_events.is_empty() {
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(s.section_events)));
        out.push_str("<table><thead><tr><th>Title</th><th>Type</th><th>Severity</th><th>Time</th></tr></thead><tbody>");
        for e in &snap.recent_events {
            out.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&e.title),
                escape_html(&e.event_type),
                escape_html(&e.severity),
                escape_html(&e.occurred_at)
            ));
        }
        out.push_str("</tbody></table>\n");
    }

    if let Some(ref p) = snap.prompt_preview {
        out.push_str(&format!(
            "<h2>{}</h2><pre>{}</pre>\n",
            escape_html(s.section_prompt),
            escape_html(p)
        ));
    }
    if let Some(ref sum) = snap.session_summary {
        out.push_str(&format!(
            "<h2>{}</h2><p>{}</p>\n",
            escape_html(s.section_summary_text),
            escape_html(sum)
        ));
    }

    out.push_str(&format!(
        "<h2>{}</h2>\n<p>{}</p>\n",
        escape_html(s.section_reproduce),
        escape_html(s.reproduce_hint)
    ));
    if snap.scope == "project" {
        let root = snap.root_path.as_deref().unwrap_or(".");
        out.push_str(&format!(
            "<pre>cd \"{}\"\nanycode dashboard --open</pre>\n",
            escape_html(root)
        ));
    } else {
        out.push_str("<pre>anycode dashboard --open</pre>\n");
    }
    out
}

fn session_cols(lang: Lang) -> [&'static str; 6] {
    match lang {
        Lang::Zh => ["标题", "类型", "状态", "信任", "会话 ID", "开始时间"],
        Lang::En => ["Title", "Kind", "Status", "Trust", "Session ID", "Started"],
    }
}

fn kpi_cell(label: &str, value: String) -> String {
    format!(
        "<div class=\"kpi\"><div class=\"v\">{}</div><div class=\"l\">{}</div></div>",
        escape_html(&value),
        escape_html(label)
    )
}

fn trust_chip_class(status: &str) -> &'static str {
    match status {
        "verified" => "chip-verified",
        "blocked" => "chip-blocked",
        "unverified" => "chip-unverified",
        _ => "chip-default",
    }
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_script() {
        assert!(escape_html("<script>").contains("&lt;"));
    }
}
