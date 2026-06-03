use super::*;

pub async fn list_skills(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    match state.db.list_skills(q.limit).await {
        Ok(skills) => Json(json!({ "skills": skills })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_project_skills(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match state.db.list_skills_for_project(&project_id).await {
        Ok(skills) => Json(json!({ "skills": skills })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_skill_suggestions(State(state): State<AppState>) -> impl IntoResponse {
    match crate::skill_suggestions::build_suggestions(&state.db).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn install_starter_skills(State(state): State<AppState>) -> impl IntoResponse {
    let Some(home) = dirs::home_dir() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "no home directory" })),
        )
            .into_response();
    };
    let dest = home.join(".anycode/skills");
    match anycode_tools::install_starter_skills(&dest) {
        Ok(installed) => {
            let ids: Vec<String> = installed.iter().map(|r| r.id.clone()).collect();
            let _ = skills_scan::sync_skills_to_db(&state.db, &state.workspace_paths).await;
            Json(json!({
                "ok": true,
                "installed": ids,
                "count": ids.len(),
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn rescan_skills(State(state): State<AppState>) -> impl IntoResponse {
    let mut roots = state.workspace_paths.clone();
    if let Ok(rows) =
        sqlx::query_scalar::<_, String>("SELECT root_path FROM projects ORDER BY updated_at DESC")
            .fetch_all(state.db.pool())
            .await
    {
        for r in rows {
            if !roots.iter().any(|x| x == &r) {
                roots.push(r);
            }
        }
    }
    match skills_scan::sync_skills_to_db(&state.db, &roots).await {
        Ok(n) => {
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput::low(
                    "skills_rescan_requested",
                    json!({ "skills_synced": n }),
                ),
            )
            .await;
            Json(json!({ "ok": true, "skills_synced": n })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub project_id: Option<String>,
    pub action: Option<String>,
    pub risk: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

pub async fn get_security_activity(
    State(state): State<AppState>,
    Query(q): Query<SecurityEventsQuery>,
) -> impl IntoResponse {
    match crate::security_events::list_security_events(&state.db, q.project_id.as_deref(), q.limit)
        .await
    {
        Ok(recent) => {
            let (denied_total, pending_total) =
                match crate::security_events::security_event_counts(&state.db).await {
                    Ok(v) => v,
                    Err(_) => (0, 0),
                };
            Json(json!({
                "summary": {
                    "denied_total": denied_total,
                    "pending_total": pending_total,
                    "recent": recent,
                    "read_only": !crate::approval_ipc::web_approvals_enabled(),
                    "note": if crate::approval_ipc::web_approvals_enabled() {
                        "Historical log from output.log. Live pending approvals appear in the Security inbox above."
                    } else {
                        "Observability only — web approval disabled (ANYCODE_DASHBOARD_WEB_APPROVAL=0)."
                    }
                }
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_tool_governance() -> impl IntoResponse {
    let tools: Vec<_> = anycode_core::tool_catalog()
        .iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "category": entry.category,
                "risk_tier": entry.risk_tier,
                "default_agents": entry.default_agents,
                "requires_approval": entry.requires_approval,
                "audit_level": entry.audit_level,
            })
        })
        .collect();
    let high_risk = tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.get("risk_tier").and_then(|v| v.as_str()),
                Some("high" | "critical")
            )
        })
        .count();
    Json(json!({
        "summary": {
            "total": tools.len(),
            "high_risk": high_risk,
            "approval_gaps": 0,
        },
        "tools": tools,
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct PendingApprovalsQuery {
    #[serde(default = "default_pending_limit")]
    pub limit: usize,
    pub session_id: Option<String>,
}

fn default_pending_limit() -> usize {
    20
}

pub async fn list_pending_approvals(
    State(state): State<AppState>,
    Query(q): Query<PendingApprovalsQuery>,
) -> impl IntoResponse {
    let web_enabled = crate::approval_ipc::web_approvals_enabled();
    let respond_allowed = crate::approval_ipc::respond_allowed(&state.host);
    let pending = if web_enabled {
        crate::approval_ipc::list_pending_for_session(q.session_id.as_deref(), q.limit)
    } else {
        vec![]
    };
    Json(json!({
        "pending": pending,
        "web_enabled": web_enabled,
        "respond_allowed": respond_allowed,
    }))
    .into_response()
}

pub async fn get_approval_summary(State(state): State<AppState>) -> impl IntoResponse {
    let web_enabled = crate::approval_ipc::web_approvals_enabled();
    let respond_allowed = crate::approval_ipc::respond_allowed(&state.host);
    let summary = if web_enabled {
        crate::approval_ipc::pending_summary()
    } else {
        crate::approval_ipc::PendingApprovalSummary {
            pending_total: 0,
            by_session: vec![],
        }
    };
    Json(json!({
        "summary": summary,
        "web_enabled": web_enabled,
        "respond_allowed": respond_allowed,
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct ApprovalRespondBody {
    pub decision: String,
}

pub async fn respond_to_approval(
    State(state): State<AppState>,
    Path(approval_id): Path<String>,
    Json(body): Json<ApprovalRespondBody>,
) -> impl IntoResponse {
    if !crate::approval_ipc::respond_allowed(&state.host) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Web approval respond is disabled for this binding. Use loopback or set ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE=1."
            })),
        )
            .into_response();
    }
    let pending = match crate::approval_ipc::get_pending(&approval_id) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "approval not found or already resolved" })),
            )
                .into_response()
        }
    };
    if let Err(e) = crate::approval_ipc::submit_response(&approval_id, &body.decision) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    let _ = crate::audit::record_audit(
        &state.db,
        crate::audit::AuditEventInput {
            project_id: None,
            session_id: Some(pending.session_id.clone()),
            action: "tool_approval_responded".into(),
            risk: "medium".into(),
            detail: json!({
                "approval_id": approval_id,
                "decision": body.decision,
                "tool": pending.tool,
                "source": "dashboard"
            }),
        },
    )
    .await;
    Json(json!({
        "ok": true,
        "approval_id": approval_id,
        "decision": body.decision
    }))
    .into_response()
}

pub async fn list_automation_policies(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match crate::automation_policy::list_policies(&state.db, &project_id).await {
        Ok(policies) => Json(json!({ "policies": policies })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct UpsertPolicyBody {
    pub name: String,
    pub policy_type: String,
    pub config: serde_json::Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub id: Option<String>,
}

pub(super) fn default_true() -> bool {
    true
}

pub async fn upsert_automation_policy(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<UpsertPolicyBody>,
) -> impl IntoResponse {
    match crate::automation_policy::upsert_policy(
        &state.db,
        &project_id,
        &body.name,
        &body.policy_type,
        body.config,
        body.enabled,
        body.id.as_deref(),
    )
    .await
    {
        Ok(policy) => Json(json!({ "policy": policy })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_automation_policy(
    State(state): State<AppState>,
    Path((project_id, policy_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let _ = project_id;
    match crate::automation_policy::delete_policy(&state.db, &policy_id).await {
        Ok(true) => Json(json!({ "ok": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "policy not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_skill_detail(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
) -> impl IntoResponse {
    match crate::skills_governance::get_skill_detail(&state.db, &skill_id).await {
        Ok(Some(detail)) => Json(json!({ "skill": detail })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "skill not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct SetSkillBody {
    pub enabled: bool,
}

pub async fn set_project_skill(
    State(state): State<AppState>,
    Path((project_id, skill_id)): Path<(String, String)>,
    Json(body): Json<SetSkillBody>,
) -> impl IntoResponse {
    match crate::skills_governance::set_project_skill(
        &state.db,
        &project_id,
        &skill_id,
        body.enabled,
    )
    .await
    {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct SkillAllProjectsBody {
    pub enabled: bool,
}

pub async fn set_skill_all_projects(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
    Json(body): Json<SkillAllProjectsBody>,
) -> impl IntoResponse {
    match crate::skills_governance::set_skill_all_projects(&state.db, &skill_id, body.enabled).await
    {
        Ok(count) => Json(json!({ "ok": true, "projects_updated": count })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct NotificationQuery {
    pub project_id: Option<String>,
}
