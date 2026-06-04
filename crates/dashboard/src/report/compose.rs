//! Assemble ReportDocument from snapshot + preferences (template / LLM).

use crate::preferences::load_preferences;
use crate::report::html::render_snapshot_html;
use crate::report::llm_writer::try_llm_bodies;
use crate::report::markdown::render_snapshot_markdown;
use crate::report::snapshot::ReportSnapshot;
use crate::schema::ReportDocument;
use tracing::debug;

#[derive(Debug, Clone, Copy)]
pub struct ComposeOptions {
    pub force_template: bool,
}

impl Default for ComposeOptions {
    fn default() -> Self {
        Self {
            force_template: false,
        }
    }
}

pub fn report_prefs() -> (String, String) {
    load_preferences()
        .map(|p| {
            (
                p.report_output_format.clone(),
                p.report_generation_mode.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                crate::schema::default_report_output_format(),
                crate::schema::default_report_generation_mode_pref(),
            )
        })
}

pub async fn compose_document(snap: ReportSnapshot, opts: ComposeOptions) -> ReportDocument {
    let (output_format, generation_mode) = report_prefs();
    compose_document_with_prefs(snap, &output_format, &generation_mode, opts).await
}

pub async fn compose_document_with_prefs(
    snap: ReportSnapshot,
    output_format: &str,
    generation_mode: &str,
    opts: ComposeOptions,
) -> ReportDocument {
    let template_md = render_snapshot_markdown(&snap);
    let template_html = render_snapshot_html(&snap);

    let use_llm = !opts.force_template && generation_mode == "llm";
    let mut mode = "template".to_string();
    let (markdown, html) = if use_llm {
        match try_llm_bodies(&snap, output_format).await {
            Ok(bodies) => {
                mode = "llm".into();
                let html_out = match output_format {
                    "html" => bodies.html.clone().or(Some(template_html.clone())),
                    "both" => bodies.html.clone().or(Some(template_html.clone())),
                    _ => None,
                };
                (bodies.markdown, html_out)
            }
            Err(e) => {
                debug!(error = %e, "LLM report failed, using template fallback");
                mode = "fallback".into();
                pick_template_outputs(output_format, template_md, template_html)
            }
        }
    } else {
        pick_template_outputs(output_format, template_md, template_html)
    };

    let doc_format = match output_format {
        "html" => "html",
        "both" => "both",
        _ => "markdown",
    };

    ReportDocument {
        scope: snap.scope.clone(),
        id: snap.id.clone(),
        title: snap.title.clone(),
        format: doc_format.into(),
        generated_at: snap.generated_at.clone(),
        trusted_status: snap.trusted_status.clone(),
        markdown,
        html,
        generation_mode: mode,
        summary: snap.summary.clone(),
        source_counts: snap.source_counts.clone(),
        lang: snap.lang_code(),
        highlights: snap.highlights.clone(),
        sessions_recent: snap.sessions_recent.clone(),
        sessions_imported_count: snap.sessions_imported_count,
        failure_groups: snap.failure_groups.clone(),
        gates: snap.gates.clone(),
        artifacts: snap.artifacts.clone(),
        project_id: snap.project_id.clone(),
        root_path: snap.root_path.clone(),
        events_sample_limit: snap.events_sample_limit,
    }
}

fn pick_template_outputs(
    output_format: &str,
    md: String,
    html: String,
) -> (String, Option<String>) {
    match output_format {
        "html" => (String::new(), Some(html)),
        "both" => (md, Some(html)),
        _ => (md, None),
    }
}
