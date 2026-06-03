//! WorkBuddy-parity doctor checks: skills starter, knowledge index, WeChat bridge, cron scheduler.

use crate::db::DashboardDb;
use crate::schema::DoctorCheck;
use crate::skill_suggestions::STARTER_SKILL_IDS;
use sqlx::Row;

pub async fn workbench_doctor_checks(db: &DashboardDb) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    checks.push(skills_starter_check(db).await);
    checks.push(knowledge_index_check(db).await);
    checks.extend(wechat_bridge_checks());
    checks.push(cron_scheduler_check());
    checks
}

async fn skills_starter_check(db: &DashboardDb) -> DoctorCheck {
    let skills = db.list_skills(200).await.unwrap_or_default();
    let installed: std::collections::HashSet<String> =
        skills.iter().map(|s| s.id.clone()).collect();
    let missing: Vec<&str> = STARTER_SKILL_IDS
        .iter()
        .copied()
        .filter(|id| !installed.contains(*id))
        .collect();
    if missing.is_empty() {
        DoctorCheck {
            id: "skills_starter_pack".into(),
            status: "ok".into(),
            message: format!("All {} starter skills installed", STARTER_SKILL_IDS.len()),
        }
    } else {
        DoctorCheck {
            id: "skills_starter_pack".into(),
            status: "warn".into(),
            message: format!(
                "Missing {} starter skill(s): {} — run `anycode skills install-starter` or Dashboard install",
                missing.len(),
                missing.join(", ")
            ),
        }
    }
}

async fn knowledge_index_check(db: &DashboardDb) -> DoctorCheck {
    let row = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM project_knowledge_paths) AS path_rows,
          (SELECT COUNT(*) FROM project_knowledge_chunks) AS chunk_rows,
          (SELECT COUNT(DISTINCT p.project_id)
             FROM project_knowledge_paths p
            WHERE NOT EXISTS (
              SELECT 1 FROM project_knowledge_chunks c WHERE c.project_id = p.project_id
            )) AS projects_needing_reindex
        "#,
    )
    .fetch_one(db.pool())
    .await;

    match row {
        Ok(r) => {
            let path_rows: i64 = r.try_get("path_rows").unwrap_or(0);
            let chunk_rows: i64 = r.try_get("chunk_rows").unwrap_or(0);
            let stale: i64 = r.try_get("projects_needing_reindex").unwrap_or(0);
            if path_rows == 0 {
                DoctorCheck {
                    id: "knowledge_index".into(),
                    status: "ok".into(),
                    message: "No project knowledge paths configured".into(),
                }
            } else if stale > 0 || chunk_rows == 0 {
                DoctorCheck {
                    id: "knowledge_index".into(),
                    status: "warn".into(),
                    message: format!(
                        "{path_rows} knowledge path(s), {chunk_rows} chunk(s); {stale} project(s) need reindex in Dashboard"
                    ),
                }
            } else {
                DoctorCheck {
                    id: "knowledge_index".into(),
                    status: "ok".into(),
                    message: format!(
                        "Knowledge index OK ({path_rows} path(s), {chunk_rows} chunk(s))"
                    ),
                }
            }
        }
        Err(e) => DoctorCheck {
            id: "knowledge_index".into(),
            status: "warn".into(),
            message: format!("Could not inspect knowledge tables: {e}"),
        },
    }
}

fn wechat_bridge_checks() -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let home = dirs::home_dir();
    let wechat_dir = home.as_ref().map(|h| h.join(".anycode/wechat"));
    let data_ok = wechat_dir.as_ref().is_some_and(|p| p.is_dir());
    checks.push(DoctorCheck {
        id: "wechat.data_dir".into(),
        status: if data_ok { "ok" } else { "warn" }.into(),
        message: if data_ok {
            format!("WeChat bridge data at {}", wechat_dir.unwrap().display())
        } else {
            "WeChat bridge not initialized (~/.anycode/wechat missing)".into()
        },
    });

    let cron_target = home
        .as_ref()
        .map(|h| h.join(".anycode/wechat/cron_notify_target.json"));
    let target_ok = cron_target.as_ref().is_some_and(|p| p.is_file());
    checks.push(DoctorCheck {
        id: "wechat.cron_notify".into(),
        status: if target_ok { "ok" } else { "warn" }.into(),
        message: if target_ok {
            "WeChat cron notify target configured".into()
        } else {
            "WeChat cron notify target missing — cron results won't push to WeChat".into()
        },
    });
    checks
}

fn cron_scheduler_check() -> DoctorCheck {
    let path = crate::cron_ledger::orchestration_path();
    let Some(path) = path else {
        return DoctorCheck {
            id: "cron_scheduler".into(),
            status: "warn".into(),
            message: "Could not resolve orchestration.json path".into(),
        };
    };
    if !path.is_file() {
        return DoctorCheck {
            id: "cron_scheduler".into(),
            status: "warn".into(),
            message: format!(
                "No orchestration file at {} — create a cron job or run scheduler once",
                path.display()
            ),
        };
    }
    match anycode_tools::read_cron_jobs_from_orchestration_file(&path) {
        Ok(jobs) => DoctorCheck {
            id: "cron_scheduler".into(),
            status: if jobs.is_empty() { "warn" } else { "ok" }.into(),
            message: if jobs.is_empty() {
                format!(
                    "Orchestration file exists but has no cron jobs ({})",
                    path.display()
                )
            } else {
                format!("{} cron job(s) in {}", jobs.len(), path.display())
            },
        },
        Err(e) => DoctorCheck {
            id: "cron_scheduler".into(),
            status: "error".into(),
            message: format!("Invalid orchestration JSON at {}: {e}", path.display()),
        },
    }
}
