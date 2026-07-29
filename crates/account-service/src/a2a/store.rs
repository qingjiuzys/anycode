//! A2A persistence (metadata only).

use crate::a2a::models::{
    AgentCard, HandoffKind, HandoffState, HandoffTaskView, TeamPeerView, HANDOFF_TTL_SECS,
    PRESENCE_TTL_SECS,
};
use crate::auth::{hash_token, new_session_token};
use crate::db::AccountDb;
use crate::models::AuthUser;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

pub async fn verify_device_owner(db: &AccountDb, user_id: &str, device_id: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM linked_devices WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
    )
    .bind(device_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;
    Ok(count > 0)
}

pub async fn upsert_presence(
    db: &AccountDb,
    user: &AuthUser,
    card: &AgentCard,
) -> Result<()> {
    if card.organization_id != user.organization_id {
        return Err(anyhow!("organization mismatch"));
    }
    if !verify_device_owner(db, &user.id, &card.device_id).await? {
        return Err(anyhow!("device not linked to user"));
    }
    let card_json = serde_json::to_string(card)?;
    sqlx::query(
        r#"
        INSERT INTO a2a_agent_presence
          (device_id, user_id, organization_id, instance_id, agent_card_json, last_heartbeat_at)
        VALUES (?, ?, ?, ?, ?, NOW(3))
        ON DUPLICATE KEY UPDATE
          user_id = VALUES(user_id),
          organization_id = VALUES(organization_id),
          instance_id = VALUES(instance_id),
          agent_card_json = VALUES(agent_card_json),
          last_heartbeat_at = NOW(3)
        "#,
    )
    .bind(&card.device_id)
    .bind(&user.id)
    .bind(&user.organization_id)
    .bind(&card.instance_id)
    .bind(&card_json)
    .execute(db.pool())
    .await?;

    sqlx::query("UPDATE linked_devices SET last_seen_at = NOW(3) WHERE id = ?")
        .bind(&card.device_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn list_team_peers(db: &AccountDb, org_id: &str) -> Result<Vec<TeamPeerView>> {
    let cutoff = Utc::now() - Duration::seconds(PRESENCE_TTL_SECS);
    let rows = sqlx::query(
        r#"
        SELECT u.id AS user_id, u.display_name, u.email,
               p.device_id, p.instance_id, p.agent_card_json, p.last_heartbeat_at,
               ld.device_name
        FROM a2a_agent_presence p
        JOIN users u ON u.id = p.user_id
        JOIN linked_devices ld ON ld.id = p.device_id AND ld.revoked_at IS NULL
        WHERE p.organization_id = ? AND p.last_heartbeat_at >= ?
        ORDER BY p.last_heartbeat_at DESC
        "#,
    )
    .bind(org_id)
    .bind(cutoff)
    .fetch_all(db.pool())
    .await?;

    let mut peers = Vec::with_capacity(rows.len());
    for r in rows {
        let card_json: String = r.get("agent_card_json");
        let card: AgentCard = serde_json::from_str(&card_json).unwrap_or_else(|_| {
            AgentCard::anycode_desktop(
                r.get::<String, _>("instance_id").as_str(),
                r.get::<String, _>("device_id").as_str(),
                org_id,
                r.get::<String, _>("user_id").as_str(),
                r.get::<String, _>("device_name").as_str(),
                "0.0.0",
            )
        });
        let last_seen: chrono::DateTime<Utc> = r.get("last_heartbeat_at");
        peers.push(TeamPeerView {
            user_id: r.get("user_id"),
            display_name: r.get("display_name"),
            email: r.get("email"),
            device_id: r.get("device_id"),
            instance_id: r.get("instance_id"),
            device_name: r.get("device_name"),
            version: card.version,
            transport: card.transport,
            online: true,
            last_seen,
            capabilities: card.capabilities,
        });
    }
    Ok(peers)
}

pub struct CreateHandoffInput {
    pub kind: HandoffKind,
    pub sender_device_id: String,
    pub sender_instance_id: String,
    pub recipient_device_id: String,
    pub recipient_instance_id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub session_id: Option<String>,
    pub session_title: Option<String>,
    pub target_project_id: Option<String>,
}

pub async fn create_handoff(
    db: &AccountDb,
    sender: &AuthUser,
    input: CreateHandoffInput,
) -> Result<HandoffTaskView> {
    if !verify_device_owner(db, &sender.id, &input.sender_device_id).await? {
        return Err(anyhow!("sender device not linked"));
    }

    let recipient = sqlx::query(
        r#"
        SELECT ld.id, ld.user_id, ld.device_name, u.organization_id, u.display_name
        FROM linked_devices ld
        JOIN users u ON u.id = ld.user_id
        WHERE ld.id = ? AND ld.revoked_at IS NULL
        "#,
    )
    .bind(&input.recipient_device_id)
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| anyhow!("recipient device not found"))?;

    let recipient_org: String = recipient.get("organization_id");
    if recipient_org != sender.organization_id {
        return Err(anyhow!("recipient not in same organization"));
    }

    let id = format!("ho_{}", Uuid::new_v4().simple());
    let expires = Utc::now() + Duration::seconds(HANDOFF_TTL_SECS);
    let kind_str = match input.kind {
        HandoffKind::Project => "project",
        HandoffKind::Session => "session",
    };

    sqlx::query(
        r#"
        INSERT INTO a2a_handoff_tasks
          (id, organization_id, kind, state, sender_user_id, sender_device_id, sender_instance_id,
           recipient_user_id, recipient_device_id, recipient_instance_id,
           project_id, project_name, session_id, session_title, target_project_id,
           created_at, updated_at, expires_at)
        VALUES (?, ?, ?, 'pending_approval', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3), ?)
        "#,
    )
    .bind(&id)
    .bind(&sender.organization_id)
    .bind(kind_str)
    .bind(&sender.id)
    .bind(&input.sender_device_id)
    .bind(&input.sender_instance_id)
    .bind(recipient.get::<String, _>("user_id"))
    .bind(&input.recipient_device_id)
    .bind(&input.recipient_instance_id)
    .bind(&input.project_id)
    .bind(&input.project_name)
    .bind(&input.session_id)
    .bind(&input.session_title)
    .bind(&input.target_project_id)
    .bind(expires)
    .execute(db.pool())
    .await?;

    get_handoff(db, &id).await
}

pub async fn list_incoming(
    db: &AccountDb,
    device_id: &str,
    user_id: &str,
) -> Result<Vec<HandoffTaskView>> {
    let rows = sqlx::query(
        r#"
        SELECT id FROM a2a_handoff_tasks
        WHERE recipient_device_id = ? AND recipient_user_id = ?
          AND state = 'pending_approval' AND expires_at > NOW(3)
        ORDER BY created_at DESC
        "#,
    )
    .bind(device_id)
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;
    let mut out = Vec::new();
    for r in rows {
        out.push(get_handoff(db, &r.get::<String, _>("id")).await?);
    }
    Ok(out)
}

pub async fn list_outgoing(
    db: &AccountDb,
    device_id: &str,
    user_id: &str,
) -> Result<Vec<HandoffTaskView>> {
    let rows = sqlx::query(
        r#"
        SELECT id FROM a2a_handoff_tasks
        WHERE sender_device_id = ? AND sender_user_id = ?
          AND state NOT IN ('completed', 'rejected', 'failed', 'expired')
          AND expires_at > NOW(3)
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .bind(device_id)
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;
    let mut out = Vec::new();
    for r in rows {
        out.push(get_handoff(db, &r.get::<String, _>("id")).await?);
    }
    Ok(out)
}

pub async fn approve_handoff(
    db: &AccountDb,
    handoff_id: &str,
    recipient: &AuthUser,
    recipient_device_id: &str,
    target_project_id: Option<String>,
) -> Result<HandoffTaskView> {
    let row = load_handoff_row(db, handoff_id).await?;
    ensure_recipient(&row, recipient, recipient_device_id)?;
    ensure_state(&row, HandoffState::PendingApproval)?;
    if row_expired(&row) {
        set_state(db, handoff_id, HandoffState::Expired).await?;
        return Err(anyhow!("handoff expired"));
    }

    let stream_token = new_session_token();
    let token_hash = hash_token(&stream_token);

    sqlx::query(
        r#"
        UPDATE a2a_handoff_tasks
        SET state = 'approved', stream_token_hash = ?, stream_token_ephemeral = ?,
            target_project_id = COALESCE(?, target_project_id),
            updated_at = NOW(3)
        WHERE id = ?
        "#,
    )
    .bind(&token_hash)
    .bind(&stream_token)
    .bind(&target_project_id)
    .bind(handoff_id)
    .execute(db.pool())
    .await?;

    let mut view = get_handoff(db, handoff_id).await?;
    view.stream_token = Some(stream_token);
    Ok(view)
}

pub async fn reject_handoff(
    db: &AccountDb,
    handoff_id: &str,
    recipient: &AuthUser,
    recipient_device_id: &str,
) -> Result<HandoffTaskView> {
    let row = load_handoff_row(db, handoff_id).await?;
    ensure_recipient(&row, recipient, recipient_device_id)?;
    set_state(db, handoff_id, HandoffState::Rejected).await?;
    get_handoff(db, handoff_id).await
}

pub async fn update_progress(
    db: &AccountDb,
    handoff_id: &str,
    state: HandoffState,
    progress_pct: u8,
    error: Option<&str>,
) -> Result<()> {
    let clear_token = matches!(
        state,
        HandoffState::Completed
            | HandoffState::Failed
            | HandoffState::Rejected
            | HandoffState::Expired
    );
    if clear_token {
        sqlx::query(
            r#"
            UPDATE a2a_handoff_tasks
            SET state = ?, progress_pct = ?, error_message = ?,
                stream_token_ephemeral = NULL, stream_token_hash = NULL,
                updated_at = NOW(3)
            WHERE id = ?
            "#,
        )
        .bind(state.as_str())
        .bind(progress_pct)
        .bind(error)
        .bind(handoff_id)
        .execute(db.pool())
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE a2a_handoff_tasks
            SET state = ?, progress_pct = ?, error_message = ?, updated_at = NOW(3)
            WHERE id = ?
            "#,
        )
        .bind(state.as_str())
        .bind(progress_pct)
        .bind(error)
        .bind(handoff_id)
        .execute(db.pool())
        .await?;
    }
    Ok(())
}

/// Return ephemeral stream token for an active party (sender or recipient).
pub async fn peek_stream_token(
    db: &AccountDb,
    handoff_id: &str,
    device_id: &str,
) -> Result<Option<String>> {
    let row = sqlx::query(
        r#"
        SELECT sender_device_id, recipient_device_id, stream_token_ephemeral, state, expires_at
        FROM a2a_handoff_tasks WHERE id = ?
        "#,
    )
    .bind(handoff_id)
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| anyhow!("handoff not found"))?;

    let expires: chrono::DateTime<Utc> = row.get("expires_at");
    if expires < Utc::now() {
        return Ok(None);
    }
    let sender: String = row.get("sender_device_id");
    let recipient: String = row.get("recipient_device_id");
    if device_id != sender && device_id != recipient {
        return Err(anyhow!("forbidden"));
    }
    let state: String = row.get("state");
    if !matches!(
        state.as_str(),
        "approved" | "uploading" | "importing"
    ) {
        return Ok(None);
    }
    Ok(row.get("stream_token_ephemeral"))
}

pub async fn verify_stream_token(
    db: &AccountDb,
    handoff_id: &str,
    token: &str,
) -> Result<HandoffTaskView> {
    let row = load_handoff_row(db, handoff_id).await?;
    if row_expired(&row) {
        set_state(db, handoff_id, HandoffState::Expired).await?;
        return Err(anyhow!("stream expired"));
    }
    let state: String = row.get("state");
    if !matches!(state.as_str(), "approved" | "uploading" | "importing") {
        return Err(anyhow!("invalid state for stream"));
    }
    let expected: Option<String> = row.get("stream_token_hash");
    match expected {
        Some(h) if h == hash_token(token) => get_handoff(db, handoff_id).await,
        _ => Err(anyhow!("invalid stream token")),
    }
}

pub async fn get_handoff(db: &AccountDb, handoff_id: &str) -> Result<HandoffTaskView> {
    let row = load_handoff_row(db, handoff_id).await?;
    row_to_view(db, &row, None).await
}

async fn load_handoff_row(db: &AccountDb, handoff_id: &str) -> Result<sqlx::mysql::MySqlRow> {
    sqlx::query("SELECT * FROM a2a_handoff_tasks WHERE id = ?")
        .bind(handoff_id)
        .fetch_optional(db.pool())
        .await?
        .ok_or_else(|| anyhow!("handoff not found"))
}

fn row_expired(row: &sqlx::mysql::MySqlRow) -> bool {
    let expires: chrono::DateTime<Utc> = row.get("expires_at");
    expires < Utc::now()
}

fn ensure_recipient(
    row: &sqlx::mysql::MySqlRow,
    user: &AuthUser,
    device_id: &str,
) -> Result<()> {
    let rid: String = row.get("recipient_user_id");
    let rdid: String = row.get("recipient_device_id");
    if rid != user.id || rdid != device_id {
        return Err(anyhow!("forbidden"));
    }
    Ok(())
}

fn ensure_state(row: &sqlx::mysql::MySqlRow, expected: HandoffState) -> Result<()> {
    let state: String = row.get("state");
    if state != expected.as_str() {
        return Err(anyhow!("invalid state"));
    }
    Ok(())
}

async fn set_state(db: &AccountDb, handoff_id: &str, state: HandoffState) -> Result<()> {
    let terminal = matches!(
        state,
        HandoffState::Completed
            | HandoffState::Failed
            | HandoffState::Rejected
            | HandoffState::Expired
    );
    if terminal {
        sqlx::query(
            r#"
            UPDATE a2a_handoff_tasks
            SET state = ?, stream_token_ephemeral = NULL, stream_token_hash = NULL,
                updated_at = NOW(3)
            WHERE id = ?
            "#,
        )
        .bind(state.as_str())
        .bind(handoff_id)
        .execute(db.pool())
        .await?;
    } else {
        sqlx::query(
            "UPDATE a2a_handoff_tasks SET state = ?, updated_at = NOW(3) WHERE id = ?",
        )
        .bind(state.as_str())
        .bind(handoff_id)
        .execute(db.pool())
        .await?;
    }
    Ok(())
}

async fn row_to_view(
    db: &AccountDb,
    row: &sqlx::mysql::MySqlRow,
    stream_token: Option<String>,
) -> Result<HandoffTaskView> {
    let sender_user_id: String = row.get("sender_user_id");
    let recipient_user_id: String = row.get("recipient_user_id");
    let sender_name = user_display(db, &sender_user_id).await?;
    let recipient_name = user_display(db, &recipient_user_id).await?;
    let kind_str: String = row.get("kind");
    let state_str: String = row.get("state");
    Ok(HandoffTaskView {
        id: row.get("id"),
        kind: if kind_str == "session" {
            HandoffKind::Session
        } else {
            HandoffKind::Project
        },
        state: HandoffState::parse(&state_str).unwrap_or(HandoffState::Failed),
        sender_user_id,
        sender_device_id: row.get("sender_device_id"),
        sender_instance_id: row.get("sender_instance_id"),
        sender_name,
        recipient_user_id,
        recipient_device_id: row.get("recipient_device_id"),
        recipient_instance_id: row.get("recipient_instance_id"),
        recipient_name,
        project_id: row.get("project_id"),
        project_name: row.get("project_name"),
        session_id: row.get("session_id"),
        session_title: row.get("session_title"),
        target_project_id: row.get("target_project_id"),
        stream_token,
        progress_pct: row.get("progress_pct"),
        error: row.get("error_message"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        expires_at: row.get("expires_at"),
    })
}

async fn user_display(db: &AccountDb, user_id: &str) -> Result<String> {
    let name: Option<String> = sqlx::query_scalar(
        "SELECT display_name FROM users WHERE id = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await?;
    Ok(name.unwrap_or_else(|| user_id.to_string()))
}
