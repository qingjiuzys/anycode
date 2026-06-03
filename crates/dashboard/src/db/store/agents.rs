use super::*;
use anycode_agent::BUILTIN_AGENT_SEED;
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentProfileRecord {
    pub id: String,
    pub scope: String,
    pub project_id: Option<String>,
    pub extends: String,
    pub description: String,
    pub tools_json: String,
    pub skills_json: String,
    pub routing_json: String,
    pub prompt_overlay: String,
    pub version: i64,
    pub builtin: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpsertAgentProfileRequest {
    pub extends: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tools_json: Option<Value>,
    #[serde(default)]
    pub skills_json: Option<Value>,
    #[serde(default)]
    pub routing_json: Option<Value>,
    #[serde(default)]
    pub prompt_overlay: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
}

impl DashboardDb {
    pub async fn list_agent_profiles(&self) -> Result<Vec<AgentProfileRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, scope, project_id, extends, description, tools_json, skills_json,
                   routing_json, prompt_overlay, version, builtin, updated_at
            FROM agent_profiles
            ORDER BY builtin DESC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| AgentProfileRecord {
                id: r.get("id"),
                scope: r.get("scope"),
                project_id: r.get("project_id"),
                extends: r.get("extends"),
                description: r.get("description"),
                tools_json: r.get("tools_json"),
                skills_json: r.get("skills_json"),
                routing_json: r.get("routing_json"),
                prompt_overlay: r.get("prompt_overlay"),
                version: r.get("version"),
                builtin: r.get::<i64, _>("builtin") != 0,
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    pub async fn get_agent_profile(&self, id: &str) -> Result<Option<AgentProfileRecord>> {
        let row = sqlx::query(
            r#"
            SELECT id, scope, project_id, extends, description, tools_json, skills_json,
                   routing_json, prompt_overlay, version, builtin, updated_at
            FROM agent_profiles WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| AgentProfileRecord {
            id: r.get("id"),
            scope: r.get("scope"),
            project_id: r.get("project_id"),
            extends: r.get("extends"),
            description: r.get("description"),
            tools_json: r.get("tools_json"),
            skills_json: r.get("skills_json"),
            routing_json: r.get("routing_json"),
            prompt_overlay: r.get("prompt_overlay"),
            version: r.get("version"),
            builtin: r.get::<i64, _>("builtin") != 0,
            updated_at: r.get("updated_at"),
        }))
    }

    pub async fn upsert_agent_profile(
        &self,
        id: &str,
        req: &UpsertAgentProfileRequest,
        builtin: bool,
    ) -> Result<AgentProfileRecord> {
        let scope = req.scope.as_deref().unwrap_or("global").to_string();
        let tools_json = req
            .tools_json
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".to_string());
        let skills_json = req
            .skills_json
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".to_string());
        let routing_json = req
            .routing_json
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".to_string());
        let description = req.description.clone().unwrap_or_default();
        let prompt_overlay = req.prompt_overlay.clone().unwrap_or_default();
        sqlx::query(
            r#"
            INSERT INTO agent_profiles
              (id, scope, project_id, extends, description, tools_json, skills_json,
               routing_json, prompt_overlay, version, builtin, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
              scope = excluded.scope,
              project_id = excluded.project_id,
              extends = excluded.extends,
              description = excluded.description,
              tools_json = excluded.tools_json,
              skills_json = excluded.skills_json,
              routing_json = excluded.routing_json,
              prompt_overlay = excluded.prompt_overlay,
              version = agent_profiles.version + 1,
              builtin = excluded.builtin,
              updated_at = datetime('now')
            "#,
        )
        .bind(id)
        .bind(&scope)
        .bind(&req.project_id)
        .bind(&req.extends)
        .bind(&description)
        .bind(&tools_json)
        .bind(&skills_json)
        .bind(&routing_json)
        .bind(&prompt_overlay)
        .bind(if builtin { 1 } else { 0 })
        .execute(&self.pool)
        .await?;
        self.get_agent_profile(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("profile missing after upsert"))
    }

    pub async fn delete_agent_profile(&self, id: &str) -> Result<bool> {
        let builtin: i64 = sqlx::query_scalar(
            "SELECT COALESCE((SELECT builtin FROM agent_profiles WHERE id = ?), 0)",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        if builtin != 0 {
            anyhow::bail!("cannot delete builtin agent profile");
        }
        let r = sqlx::query("DELETE FROM agent_profiles WHERE id = ? AND builtin = 0")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    pub async fn seed_builtin_agent_profiles(&self) -> Result<usize> {
        let mut n = 0usize;
        for seed in BUILTIN_AGENT_SEED {
            self.upsert_agent_profile(
                seed.id,
                &UpsertAgentProfileRequest {
                    extends: seed.extends.to_string(),
                    description: Some(seed.description.to_string()),
                    tools_json: None,
                    skills_json: None,
                    routing_json: None,
                    prompt_overlay: None,
                    scope: Some("global".into()),
                    project_id: None,
                },
                true,
            )
            .await?;
            n += 1;
        }
        Ok(n)
    }
}
