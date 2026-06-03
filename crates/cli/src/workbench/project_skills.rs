//! Load project-scoped enabled skills from the dashboard DB (caller-side DI for bootstrap).

use std::collections::HashSet;
use std::path::Path;

/// Enabled skill ids for `cwd` when the dashboard SQLite DB exists.
pub async fn load_project_enabled_skills(cwd: &Path) -> Option<HashSet<String>> {
    let db = anycode_dashboard::project_skills::open_default_db_if_exists()
        .await
        .ok()??;
    anycode_dashboard::project_skills::enabled_skill_ids_for_path(&db, cwd)
        .await
        .ok()
        .flatten()
}
