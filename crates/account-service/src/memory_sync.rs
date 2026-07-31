//! Opaque E2EE memory sync: server stores ciphertext envelopes only.

use crate::db::AccountDb;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEnvelopeIn {
    pub id: String,
    pub ciphertext_b64: String,
    pub nonce_b64: String,
    pub content_hash: String,
    #[serde(default)]
    pub version_vector: HashMap<String, u64>,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEnvelopeOut {
    pub id: String,
    pub ciphertext_b64: String,
    pub nonce_b64: String,
    pub content_hash: String,
    pub version_vector_json: String,
    pub deleted: bool,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PushBody {
    pub device_id: String,
    pub envelopes: Vec<MemoryEnvelopeIn>,
}

#[derive(Debug, Serialize)]
pub struct PushResult {
    pub accepted: usize,
    pub tombstones_applied: usize,
}

#[derive(Debug, Serialize)]
pub struct PullResult {
    pub envelopes: Vec<MemoryEnvelopeOut>,
    pub tombstones: Vec<String>,
}

pub async fn push_envelopes(db: &AccountDb, user_id: &str, body: PushBody) -> Result<PushResult> {
    let mut accepted = 0usize;
    let mut tombstones_applied = 0usize;
    for env in body.envelopes {
        let vv = serde_json::to_string(&env.version_vector).unwrap_or_else(|_| "{}".into());
        if env.deleted {
            sqlx::query(
                r#"
                INSERT INTO memory_sync_tombstones (user_id, envelope_id, device_id, deleted_at)
                VALUES (?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE device_id = VALUES(device_id), deleted_at = VALUES(deleted_at)
                "#,
            )
            .bind(user_id)
            .bind(&env.id)
            .bind(&body.device_id)
            .bind(Utc::now())
            .execute(db.pool())
            .await?;
            sqlx::query("DELETE FROM memory_sync_envelopes WHERE user_id = ? AND envelope_id = ?")
                .bind(user_id)
                .bind(&env.id)
                .execute(db.pool())
                .await?;
            tombstones_applied += 1;
            continue;
        }
        // Version-vector guard: a stale device must not clobber a newer envelope.
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT version_vector_json FROM memory_sync_envelopes WHERE user_id = ? AND envelope_id = ?",
        )
        .bind(user_id)
        .bind(&env.id)
        .fetch_optional(db.pool())
        .await?;
        if let Some((stored_json,)) = existing {
            let stored: HashMap<String, u64> =
                serde_json::from_str(&stored_json).unwrap_or_default();
            if dominates(&stored, &env.version_vector) {
                continue; // stored is strictly newer — drop the stale write
            }
        }
        sqlx::query(
            r#"
            INSERT INTO memory_sync_envelopes
              (user_id, envelope_id, device_id, ciphertext_b64, nonce_b64, content_hash, version_vector_json, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
              device_id = VALUES(device_id),
              ciphertext_b64 = VALUES(ciphertext_b64),
              nonce_b64 = VALUES(nonce_b64),
              content_hash = VALUES(content_hash),
              version_vector_json = VALUES(version_vector_json),
              updated_at = VALUES(updated_at)
            "#,
        )
        .bind(user_id)
        .bind(&env.id)
        .bind(&body.device_id)
        .bind(&env.ciphertext_b64)
        .bind(&env.nonce_b64)
        .bind(&env.content_hash)
        .bind(&vv)
        .bind(Utc::now())
        .execute(db.pool())
        .await?;
        accepted += 1;
    }
    Ok(PushResult {
        accepted,
        tombstones_applied,
    })
}

/// `a` dominates `b` when every counter in `a` is >= `b`'s and at least one is
/// strictly greater. Empty vectors never dominate.
fn dominates(a: &HashMap<String, u64>, b: &HashMap<String, u64>) -> bool {
    if a.is_empty() {
        return false;
    }
    let mut strictly_greater = false;
    for (k, av) in a {
        let bv = b.get(k).copied().unwrap_or(0);
        if av < &bv {
            return false;
        }
        if av > &bv {
            strictly_greater = true;
        }
    }
    // Any key present only in `b` means `a` is missing history — not dominant.
    if b.keys().any(|k| !a.contains_key(k)) {
        return false;
    }
    strictly_greater
}

pub async fn pull_envelopes(db: &AccountDb, user_id: &str) -> Result<PullResult> {
    let rows = sqlx::query(
        r#"
        SELECT envelope_id, ciphertext_b64, nonce_b64, content_hash, version_vector_json, updated_at
        FROM memory_sync_envelopes
        WHERE user_id = ?
        ORDER BY updated_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;

    let mut envelopes = Vec::new();
    for row in rows {
        let updated_at: chrono::DateTime<Utc> = row.try_get("updated_at")?;
        envelopes.push(MemoryEnvelopeOut {
            id: row.try_get("envelope_id")?,
            ciphertext_b64: row.try_get("ciphertext_b64")?,
            nonce_b64: row.try_get("nonce_b64")?,
            content_hash: row.try_get("content_hash")?,
            version_vector_json: row.try_get("version_vector_json")?,
            deleted: false,
            updated_at: updated_at.to_rfc3339(),
        });
    }

    let tomb_rows = sqlx::query(
        "SELECT envelope_id FROM memory_sync_tombstones WHERE user_id = ? ORDER BY deleted_at DESC LIMIT 500",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;
    let tombstones = tomb_rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("envelope_id").ok())
        .collect();

    Ok(PullResult {
        envelopes,
        tombstones,
    })
}

#[cfg(test)]
mod tests {
    use super::dominates;
    use std::collections::HashMap;

    fn vv(pairs: &[(&str, u64)]) -> HashMap<String, u64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn dominates_requires_all_gte_and_one_gt() {
        assert!(dominates(
            &vv(&[("a", 2), ("b", 1)]),
            &vv(&[("a", 1), ("b", 1)])
        ));
        assert!(!dominates(&vv(&[("a", 1)]), &vv(&[("a", 1)]))); // equal
        assert!(!dominates(&vv(&[("a", 1)]), &vv(&[("a", 2)]))); // older
    }

    #[test]
    fn dominates_false_when_missing_history_or_empty() {
        assert!(!dominates(&vv(&[("a", 5)]), &vv(&[("b", 1)]))); // b-only key
        assert!(!dominates(&vv(&[]), &vv(&[]))); // empty never dominates
        assert!(dominates(&vv(&[("a", 1)]), &vv(&[]))); // newer than nothing
    }
}
