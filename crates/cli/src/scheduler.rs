//! 内置 cron 调度器：读取 `~/.anycode/tasks/orchestration.json` 中 `CronCreate` 持久化的任务并按计划执行。
//! `command` 作为单次 agent 提示（与 `anycode run` 单次任务一致）。

use crate::app_config::Config;
use crate::bootstrap;
use crate::tasks::{run_single_task_with_tail, ReplSink};
use crate::workspace;
use anycode_tools::{read_cron_jobs_from_orchestration_file, CronJob};
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

const MAX_CATCHUP_PER_WAKE: usize = 5;

struct ParsedJob {
    job: CronJob,
    schedule: Schedule,
}

fn orchestration_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/tasks/orchestration.json"))
}

/// 6 字段：`sec min hour day month weekday`（与 `cron` crate 一致）。若为传统 5 字段（分 时 日 月 周），自动补 `0` 秒。
pub(crate) fn normalize_cron_schedule_expr(expr: &str) -> String {
    let t = expr.trim();
    let fields: Vec<&str> = t.split_whitespace().collect();
    match fields.len() {
        5 => format!("0 {}", t),
        _ => t.to_string(),
    }
}

fn parse_jobs(jobs: Vec<CronJob>) -> Vec<ParsedJob> {
    let mut out = Vec::new();
    for job in jobs {
        let normalized = normalize_cron_schedule_expr(&job.schedule);
        match Schedule::from_str(&normalized) {
            Ok(schedule) => out.push(ParsedJob { job, schedule }),
            Err(e) => warn!(
                target: "anycode_scheduler",
                "skip cron job {}: invalid schedule {:?}: {}",
                job.id,
                job.schedule,
                e
            ),
        }
    }
    out
}

fn load_parsed_jobs(path: &Path) -> anyhow::Result<Vec<ParsedJob>> {
    let v = read_cron_jobs_from_orchestration_file(path)?;
    Ok(parse_jobs(v))
}

fn duration_until_next_tick(
    parsed: &[ParsedJob],
    last_fire: &HashMap<String, DateTime<Utc>>,
    now: DateTime<Utc>,
    reload_cap: Duration,
) -> Duration {
    let epoch = Utc.timestamp_opt(0, 0).single().expect("epoch");
    let mut soonest: Option<DateTime<Utc>> = None;
    for pj in parsed {
        let last = last_fire.get(&pj.job.id).cloned().unwrap_or(epoch);
        if let Some(n) = pj.schedule.after(&last).next() {
            if n <= now {
                return Duration::from_secs(0);
            }
            soonest = Some(match soonest {
                None => n,
                Some(s) if n < s => n,
                Some(s) => s,
            });
        }
    }
    let wait = match soonest {
        None => reload_cap,
        Some(t) => {
            let d = (t - now).to_std().unwrap_or(Duration::from_secs(0));
            d.min(reload_cap)
        }
    };
    wait.max(Duration::from_millis(50))
}

pub(crate) async fn run_builtin_scheduler(
    mut config: Config,
    working_dir: PathBuf,
    reload_interval: Duration,
) -> anyhow::Result<()> {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    workspace::apply_project_overlays(&mut config, &working_dir);
    let orch_path =
        orchestration_path().ok_or_else(|| anyhow::anyhow!("could not resolve home directory"))?;

    info!(
        target: "anycode_scheduler",
        "starting built-in scheduler; orchestration={}; workdir={}",
        orch_path.display(),
        working_dir.display()
    );

    let runtime: Arc<anycode_agent::AgentRuntime> =
        bootstrap::initialize_runtime(&config, None).await?;
    let disk = anycode_core::DiskTaskOutput::new_default()?;

    let default_agent = config
        .runtime
        .default_mode
        .default_agent()
        .as_str()
        .to_string();

    let mut last_fire: HashMap<String, DateTime<Utc>> = HashMap::new();

    loop {
        let now = Utc::now();
        let parsed = match load_parsed_jobs(&orch_path) {
            Ok(p) => p,
            Err(e) => {
                warn!(target: "anycode_scheduler", "read orchestration: {e}");
                tokio::time::sleep(reload_interval).await;
                continue;
            }
        };

        let epoch = Utc.timestamp_opt(0, 0).single().expect("epoch");

        for pj in &parsed {
            let mut last = last_fire.get(&pj.job.id).cloned().unwrap_or(epoch);
            let mut catchup = 0usize;
            loop {
                let next = match pj.schedule.after(&last).next() {
                    None => break,
                    Some(t) => t,
                };
                if next > now {
                    break;
                }
                catchup += 1;
                if catchup > MAX_CATCHUP_PER_WAKE {
                    warn!(
                        target: "anycode_scheduler",
                        "job {} exceeded catch-up limit ({}); advancing to {}",
                        pj.job.id,
                        MAX_CATCHUP_PER_WAKE,
                        next
                    );
                    last = next;
                    break;
                }
                info!(
                    target: "anycode_scheduler",
                    "cron fire job={} schedule={:?} at {}",
                    pj.job.id,
                    pj.job.schedule,
                    next
                );
                let mut sink = ReplSink::Stdio;
                if let Err(e) = run_single_task_with_tail(
                    runtime.as_ref(),
                    &disk,
                    default_agent.clone(),
                    pj.job.command.clone(),
                    working_dir.clone(),
                    &mut sink,
                )
                .await
                {
                    warn!(
                        target: "anycode_scheduler",
                        "cron job {} execution error: {e}",
                        pj.job.id
                    );
                }
                runtime.sync_memory_durability();
                last = next;
            }
            last_fire.insert(pj.job.id.clone(), last);
        }

        let sleep_d = duration_until_next_tick(&parsed, &last_fire, Utc::now(), reload_interval);
        tokio::time::sleep(sleep_d).await;
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_cron_schedule_expr;
    use cron::Schedule;
    use std::str::FromStr;

    #[test]
    fn normalize_inserts_seconds_for_five_field() {
        assert_eq!(
            normalize_cron_schedule_expr("*/15 * * * *"),
            "0 */15 * * * *"
        );
    }

    #[test]
    fn six_field_passthrough_trimmed() {
        let s = normalize_cron_schedule_expr("  0 0 12 * * *  ");
        assert_eq!(s, "0 0 12 * * *");
        assert!(Schedule::from_str(&s).is_ok());
    }
}
