//! Cross-entity search for the workbench UI.

use crate::db::DashboardDb;
use crate::schema::{SearchHit, SearchResults};
use anyhow::Result;

pub async fn search(db: &DashboardDb, query: &str, limit: i64) -> Result<SearchResults> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(SearchResults::default());
    }
    let pattern = format!("%{q}%");
    let per_kind = limit.max(1).min(30);

    let projects = sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT id, name, root_path FROM projects
        WHERE name LIKE ? OR root_path LIKE ? OR description LIKE ?
        ORDER BY updated_at DESC
        LIMIT ?
        "#,
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(per_kind)
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(|(id, name, root_path)| {
        let href = format!("/projects/{id}");
        SearchHit {
            kind: "project".into(),
            id: id.clone(),
            title: name,
            subtitle: root_path,
            href: Some(href),
            project_id: Some(id),
            session_id: None,
        }
    })
    .collect();

    let sessions = sqlx::query_as::<_, (String, String, String, String)>(
        r#"
        SELECT s.id, s.title, s.kind, p.name AS project_name
        FROM sessions s
        JOIN projects p ON p.id = s.project_id
        WHERE s.title LIKE ? OR s.prompt_preview LIKE ? OR s.summary LIKE ?
        ORDER BY s.started_at DESC
        LIMIT ?
        "#,
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(per_kind)
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(|(id, title, kind, project_name)| SearchHit {
        kind: "session".into(),
        id: id.clone(),
        title,
        subtitle: format!("{project_name} · {kind}"),
        href: Some(format!("/sessions/{id}")),
        project_id: None,
        session_id: Some(id),
    })
    .collect();

    let events = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
        r#"
        SELECT e.id, e.title, e.event_type, p.name AS project_name, e.project_id, e.session_id
        FROM project_events e
        JOIN projects p ON p.id = e.project_id
        WHERE e.title LIKE ? OR e.body LIKE ? OR e.event_type LIKE ?
        ORDER BY e.occurred_at DESC
        LIMIT ?
        "#,
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(per_kind)
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(
        |(id, title, event_type, project_name, project_id, session_id)| SearchHit {
            kind: "event".into(),
            id: id.clone(),
            title,
            subtitle: format!("{project_name} · {event_type}"),
            href: Some(format!("/events/{id}")),
            project_id: Some(project_id),
            session_id,
        },
    )
    .collect();

    Ok(SearchResults {
        query: q.to_string(),
        projects,
        sessions,
        events,
    })
}
