//! Organization team setup and email invites — required before A2A colleague discovery.

use crate::auth::{hash_token, new_session_token};
use crate::db::AccountDb;
use crate::models::AuthUser;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamGate {
    SetupRequired,
    InviteRequired,
    Ready,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamStatusView {
    pub gate: TeamGate,
    pub organization_name: String,
    pub member_count: u64,
    pub team_setup: bool,
    pub pending_invites: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrgInviteView {
    pub id: String,
    pub email: String,
    pub status: String,
    pub expires_at: String,
    pub created_at: String,
}

pub async fn team_status(db: &AccountDb, org_id: &str) -> Result<TeamStatusView> {
    let row = sqlx::query(
        "SELECT name, team_setup_at FROM organizations WHERE id = ? LIMIT 1",
    )
    .bind(org_id)
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| anyhow!("organization not found"))?;

    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE organization_id = ? AND status != 'disabled'")
            .bind(org_id)
            .fetch_one(db.pool())
            .await?;

    let pending_invites: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM org_invites WHERE organization_id = ? AND status = 'pending' AND expires_at > NOW(3)",
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;

    let team_setup = row.get::<Option<chrono::DateTime<Utc>>, _>("team_setup_at").is_some();
    let member_count = member_count.max(0) as u64;
    let gate = if !team_setup {
        TeamGate::SetupRequired
    } else if member_count < 2 {
        TeamGate::InviteRequired
    } else {
        TeamGate::Ready
    };

    Ok(TeamStatusView {
        gate,
        organization_name: row.get("name"),
        member_count,
        team_setup,
        pending_invites: pending_invites.max(0) as u64,
    })
}

pub async fn collaboration_enabled(db: &AccountDb, org_id: &str) -> Result<bool> {
    Ok(team_status(db, org_id).await?.gate == TeamGate::Ready)
}

pub async fn setup_team(db: &AccountDb, user: &AuthUser, name: &str) -> Result<TeamStatusView> {
    if user.role != "owner" {
        return Err(anyhow!("only organization owner can create a team"));
    }
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("team name required"));
    }
    sqlx::query(
        "UPDATE organizations SET name = ?, team_setup_at = COALESCE(team_setup_at, NOW(3)), updated_at = NOW(3) WHERE id = ?",
    )
    .bind(name)
    .bind(&user.organization_id)
    .execute(db.pool())
    .await?;
    team_status(db, &user.organization_id).await
}

pub struct CreateInviteResult {
    pub invite: OrgInviteView,
    pub accept_token: String,
}

pub async fn create_invite(
    db: &AccountDb,
    user: &AuthUser,
    email: &str,
) -> Result<CreateInviteResult> {
    if user.role != "owner" {
        return Err(anyhow!("only organization owner can invite members"));
    }
    let status = team_status(db, &user.organization_id).await?;
    if !status.team_setup {
        return Err(anyhow!("create the team before inviting colleagues"));
    }

    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(anyhow!("valid email required"));
    }

    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE organization_id = ? AND LOWER(email) = ?",
    )
    .bind(&user.organization_id)
    .bind(&email)
    .fetch_one(db.pool())
    .await?;
    if existing > 0 {
        return Err(anyhow!("user is already a team member"));
    }

    let limits = crate::plan::limits_for_plan(db, "free").await;
    let seat_limit = limits.seat_limit.max(1) as i64;
    let active_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE organization_id = ? AND status != 'disabled'",
    )
    .bind(&user.organization_id)
    .fetch_one(db.pool())
    .await?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM org_invites WHERE organization_id = ? AND status = 'pending' AND expires_at > NOW(3)",
    )
    .bind(&user.organization_id)
    .fetch_one(db.pool())
    .await?;
    if active_members + pending >= seat_limit {
        return Err(anyhow!("team seat limit reached"));
    }

    let token = new_session_token();
    let token_hash = hash_token(&token);
    let id = format!("inv_{}", Uuid::new_v4().simple());
    let expires = Utc::now() + Duration::days(7);

    sqlx::query(
        r#"
        INSERT INTO org_invites (id, organization_id, email, invited_by_user_id, token_hash, status, expires_at)
        VALUES (?, ?, ?, ?, ?, 'pending', ?)
        ON DUPLICATE KEY UPDATE
          invited_by_user_id = VALUES(invited_by_user_id),
          token_hash = VALUES(token_hash),
          status = 'pending',
          expires_at = VALUES(expires_at),
          accepted_at = NULL,
          created_at = NOW(3)
        "#,
    )
    .bind(&id)
    .bind(&user.organization_id)
    .bind(&email)
    .bind(&user.id)
    .bind(&token_hash)
    .bind(expires)
    .execute(db.pool())
    .await?;

    let invite = get_invite_by_email(db, &user.organization_id, &email).await?;
    Ok(CreateInviteResult {
        invite,
        accept_token: token,
    })
}

pub async fn list_invites(db: &AccountDb, org_id: &str) -> Result<Vec<OrgInviteView>> {
    let rows = sqlx::query(
        r#"
        SELECT id, email, status, expires_at, created_at
        FROM org_invites
        WHERE organization_id = ? AND status = 'pending' AND expires_at > NOW(3)
        ORDER BY created_at DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows.into_iter().map(row_to_invite).collect())
}

pub async fn accept_invite(db: &AccountDb, user: &AuthUser, token: &str) -> Result<TeamStatusView> {
    let token_hash = hash_token(token.trim());
    let row = sqlx::query(
        r#"
        SELECT id, organization_id, email, status, expires_at
        FROM org_invites
        WHERE token_hash = ? AND status = 'pending'
        LIMIT 1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(|| anyhow!("invite not found or expired"))?;

    let expires: chrono::DateTime<Utc> = row.get("expires_at");
    if expires < Utc::now() {
        return Err(anyhow!("invite expired"));
    }

    let invite_email: String = row.get("email");
    if user.email.to_lowercase() != invite_email.to_lowercase() {
        return Err(anyhow!("invite email does not match signed-in account"));
    }

    let target_org: String = row.get("organization_id");
    if target_org == user.organization_id {
        sqlx::query(
            "UPDATE org_invites SET status = 'accepted', accepted_at = NOW(3) WHERE id = ?",
        )
        .bind(row.get::<String, _>("id"))
        .execute(db.pool())
        .await?;
        return team_status(db, &target_org).await;
    }

    let solo: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE organization_id = ? AND status != 'disabled'",
    )
    .bind(&user.organization_id)
    .fetch_one(db.pool())
    .await?;
    if solo != 1 {
        return Err(anyhow!("leave your current team before accepting this invite"));
    }
    if user.role != "owner" {
        return Err(anyhow!("only solo workspace owners can accept a team invite"));
    }

    let old_org = user.organization_id.clone();
    let invite_id: String = row.get("id");
    let mut tx = db.pool().begin().await?;

    sqlx::query("UPDATE users SET organization_id = ?, role = 'member', updated_at = NOW(3) WHERE id = ?")
        .bind(&target_org)
        .bind(&user.id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "UPDATE org_invites SET status = 'accepted', accepted_at = NOW(3) WHERE id = ?",
    )
    .bind(&invite_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM organizations WHERE id = ?")
        .bind(&old_org)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    team_status(db, &target_org).await
}

async fn get_invite_by_email(
    db: &AccountDb,
    org_id: &str,
    email: &str,
) -> Result<OrgInviteView> {
    let row = sqlx::query(
        r#"
        SELECT id, email, status, expires_at, created_at
        FROM org_invites
        WHERE organization_id = ? AND email = ?
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(org_id)
    .bind(email)
    .fetch_one(db.pool())
    .await?;
    Ok(row_to_invite(row))
}

fn row_to_invite(row: sqlx::mysql::MySqlRow) -> OrgInviteView {
    let expires: chrono::DateTime<Utc> = row.get("expires_at");
    let created: chrono::DateTime<Utc> = row.get("created_at");
    OrgInviteView {
        id: row.get("id"),
        email: row.get("email"),
        status: row.get("status"),
        expires_at: expires.to_rfc3339(),
        created_at: created.to_rfc3339(),
    }
}
