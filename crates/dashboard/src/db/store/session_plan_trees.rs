use super::*;
use anycode_core::PlanTree;

impl DashboardDb {
    pub async fn get_session_plan_tree(
        &self,
        session_id: &str,
    ) -> Result<Option<(PlanTree, String)>> {
        let row = sqlx::query(
            r#"
            SELECT tree_json, updated_at FROM session_plan_trees WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let tree_json: String = row.try_get("tree_json")?;
        let updated_at: String = row.try_get("updated_at")?;
        let tree: PlanTree = serde_json::from_str(&tree_json).unwrap_or_default();
        Ok(Some((tree, updated_at)))
    }

    pub async fn upsert_session_plan_tree(&self, session_id: &str, tree: &PlanTree) -> Result<()> {
        let tree_json = serde_json::to_string(tree)?;
        sqlx::query(
            r#"
            INSERT INTO session_plan_trees (session_id, tree_json, updated_at)
            VALUES (?, ?, datetime('now'))
            ON CONFLICT(session_id) DO UPDATE SET
                tree_json = excluded.tree_json,
                updated_at = datetime('now')
            "#,
        )
        .bind(session_id)
        .bind(&tree_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_session_plan_tree(&self, session_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM session_plan_trees WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_session_todos(
        &self,
        session_id: &str,
    ) -> Result<Option<(Vec<serde_json::Value>, String)>> {
        let row = sqlx::query(
            r#"
            SELECT todos_json, updated_at FROM session_todos WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let todos_json: String = row.try_get("todos_json")?;
        let updated_at: String = row.try_get("updated_at")?;
        let todos: Vec<serde_json::Value> = serde_json::from_str(&todos_json).unwrap_or_default();
        Ok(Some((todos, updated_at)))
    }

    pub async fn upsert_session_todos(
        &self,
        session_id: &str,
        todos: &[serde_json::Value],
    ) -> Result<()> {
        let todos_json = serde_json::to_string(todos)?;
        sqlx::query(
            r#"
            INSERT INTO session_todos (session_id, todos_json, updated_at)
            VALUES (?, ?, datetime('now'))
            ON CONFLICT(session_id) DO UPDATE SET
                todos_json = excluded.todos_json,
                updated_at = datetime('now')
            "#,
        )
        .bind(session_id)
        .bind(&todos_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_session_todos(&self, session_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM session_todos WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
