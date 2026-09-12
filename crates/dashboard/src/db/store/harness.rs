use super::DashboardDb;
use anyhow::Result;

impl DashboardDb {
    pub async fn grant_harness_accounts_member(
        &self,
        project_id: &str,
        accounts_sub: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO harness_accounts_members (project_id, accounts_sub)
             VALUES (?, ?)",
        )
        .bind(project_id)
        .bind(accounts_sub)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_harness_accounts_member(
        &self,
        project_id: &str,
        accounts_sub: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM harness_accounts_members
             WHERE project_id = ? AND accounts_sub = ?",
        )
        .bind(project_id)
        .bind(accounts_sub)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn harness_accounts_member(
        &self,
        project_id: &str,
        accounts_sub: &str,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM harness_accounts_members
             WHERE project_id = ? AND accounts_sub = ?",
        )
        .bind(project_id)
        .bind(accounts_sub)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn insert_harness_device_challenge(
        &self,
        id: &str,
        accounts_sub: &str,
        label: &str,
        challenge_hash: &str,
        expires_at: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO harness_devices
             (id, accounts_sub, label, challenge_hash, status, expires_at)
             VALUES (?, ?, ?, ?, 'pending', ?)",
        )
        .bind(id)
        .bind(accounts_sub)
        .bind(label)
        .bind(challenge_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn confirm_harness_device(
        &self,
        challenge_hash: &str,
        token_hash: &str,
        now: &str,
    ) -> Result<Option<(String, String)>> {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT id, accounts_sub FROM harness_devices
             WHERE challenge_hash = ? AND status = 'pending' AND expires_at > ?",
        )
        .bind(challenge_hash)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;
        let Some((id, sub)) = row else {
            return Ok(None);
        };
        let updated = sqlx::query(
            "UPDATE harness_devices
             SET token_hash = ?, challenge_hash = NULL, status = 'active'
             WHERE id = ? AND status = 'pending'",
        )
        .bind(token_hash)
        .bind(&id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            return Ok(None);
        }
        Ok(Some((id, sub)))
    }

    pub async fn harness_device_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<(String, String, String)>> {
        let row = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, accounts_sub, status FROM harness_devices WHERE token_hash = ?",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn revoke_harness_device(
        &self,
        id: &str,
        accounts_sub: &str,
        now: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE harness_devices
             SET status = 'revoked', revoked_at = ?, token_hash = NULL
             WHERE id = ? AND accounts_sub = ? AND status = 'active'",
        )
        .bind(now)
        .bind(id)
        .bind(accounts_sub)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::UpsertProjectRequest;
    use tempfile::tempdir;

    #[tokio::test]
    async fn accounts_membership_is_explicit() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("members.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/harness-member".into(),
                name: Some("M".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let sub = "11111111-1111-1111-1111-111111111111";
        assert!(!db.harness_accounts_member(&project.id, sub).await.unwrap());
        db.grant_harness_accounts_member(&project.id, sub)
            .await
            .unwrap();
        assert!(db.harness_accounts_member(&project.id, sub).await.unwrap());
        assert!(db
            .revoke_harness_accounts_member(&project.id, sub)
            .await
            .unwrap());
        assert!(!db.harness_accounts_member(&project.id, sub).await.unwrap());
    }
}
