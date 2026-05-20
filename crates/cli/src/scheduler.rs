//! 内置 cron 调度器：读取 `~/.anycode/tasks/orchestration.json` 中 `CronCreate` 持久化的任务并按计划执行。
//! `command` 作为单次 agent 提示（与 `anycode run` 单次任务一致）。

use crate::app_config::Config;
use crate::bootstrap;
use crate::tasks::{run_single_task_with_tail, ReplSink, RunTaskOptions};
use crate::workspace;
use crate::wx::cron_notify::deliver_cron_to_wechat;
use crate::wx::WxSender;
use anycode_tools::{read_cron_jobs_from_orchestration_file, CronJob};
use chrono::{DateTime, Utc};
use cron::Schedule;
use fs4::fs_std::FileExt;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

const MAX_CATCHUP_PER_WAKE: usize = 5;

fn cron_runs_log_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/logs/cron-runs.jsonl"))
}

fn append_cron_run_log_with_session(
    job_id: &str,
    session_id: Option<&str>,
    fired_at: DateTime<Utc>,
    status: &str,
    detail: Option<&str>,
) {
    let Some(path) = cron_runs_log_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = serde_json::json!({
        "job_id": job_id,
        "session_id": session_id.unwrap_or(""),
        "fired_at": fired_at.to_rfc3339(),
        "status": status,
        "detail": detail.unwrap_or(""),
    });
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(f, "{line}");
    }
}

/// 微信桥内嵌调度器：到点后把任务结果推回最近会话。
#[derive(Clone)]
pub(crate) struct SchedulerWechatHooks {
    pub data_root: PathBuf,
    pub sender: Arc<WxSender>,
}

struct ParsedJob {
    job: CronJob,
    schedule: Schedule,
}

fn orchestration_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/tasks/orchestration.json"))
}

/// 单实例互斥：与内嵌于微信桥的调度器或另一 `anycode scheduler` 进程共享，避免重复触发 cron。
pub(crate) fn scheduler_lock_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/tasks/scheduler.lock"))
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
    let mut soonest: Option<DateTime<Utc>> = None;
    for pj in parsed {
        // Anchor never-seen jobs at `now`, not Unix epoch: epoch makes every expression's
        // first tick far in the past → zero sleep + unbounded historical catch-up attempts.
        let last = last_fire.get(&pj.job.id).cloned().unwrap_or(now);
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

/// When `shared_runtime` is set (IM bridge), reuse that runtime instead of a second
/// `initialize_runtime`. Standalone scheduler uses `MemoryAttachMode::Shared` (file on same path).
pub(crate) async fn run_builtin_scheduler(
    mut config: Config,
    working_dir: PathBuf,
    reload_interval: Duration,
    shared_runtime: Option<Arc<RwLock<Arc<anycode_agent::AgentRuntime>>>>,
    delivery: CronDelivery,
) -> anyhow::Result<()> {
    let wechat_hooks = delivery.wechat_hooks();
    let lock_path =
        scheduler_lock_path().ok_or_else(|| anyhow::anyhow!("could not resolve home directory"))?;
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(anyhow::Error::from)?;

    match lock_file.try_lock_exclusive() {
        Ok(()) => {}
        Err(e) => {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                info!(
                    target: "anycode_scheduler",
                    "scheduler lock busy (another process holds {}); exiting scheduler",
                    lock_path.display()
                );
                return Ok(());
            }
            return Err(anyhow::Error::new(e));
        }
    }

    let _scheduler_lock_holder = lock_file;

    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    workspace::apply_project_overlays(&mut config, &working_dir);
    let orch_path =
        orchestration_path().ok_or_else(|| anyhow::anyhow!("could not resolve home directory"))?;

    info!(
        target: "anycode_scheduler",
        "starting built-in scheduler; lock={}; orchestration={}; workdir={}",
        lock_path.display(),
        orch_path.display(),
        working_dir.display()
    );

    let owned_runtime = if shared_runtime.is_none() {
        Some(
            bootstrap::initialize_runtime(&config, None, None, bootstrap::MemoryAttachMode::Shared)
                .await?,
        )
    } else {
        None
    };
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

        for pj in &parsed {
            let mut last = last_fire.get(&pj.job.id).cloned().unwrap_or(now);
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
                append_cron_run_log_with_session(
                    &pj.job.id,
                    pj.job.session_id.as_deref(),
                    next,
                    "started",
                    None,
                );
                if let Some(hooks) = &wechat_hooks {
                    deliver_cron_to_wechat(
                        &hooks.data_root,
                        &hooks.sender,
                        pj.job.command.as_str(),
                        "",
                    )
                    .await;
                }
                let runtime = match &shared_runtime {
                    Some(sr) => sr.read().await.clone(),
                    None => owned_runtime
                        .as_ref()
                        .expect("owned_runtime when not embedded")
                        .clone(),
                };
                let mut sink = ReplSink::Stdio;
                let mut captured = String::new();
                let cron_prompt = format!(
                    "{}\n\n\
                     [Scheduled cron — deliver to the user]\n\
                     Execute the reminder above in concise Chinese. \
                     Your final answer will be pushed to the user's WeChat chat automatically.",
                    pj.job.command
                );
                let cron_session_id = pj
                    .job
                    .session_id
                    .as_deref()
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let run_options = RunTaskOptions {
                    session_id: cron_session_id,
                    tool_profile: pj.job.tool_profile.clone(),
                    tool_allowlist: pj.job.tool_allowlist.clone(),
                };
                if let Err(e) = run_single_task_with_tail(
                    runtime.as_ref(),
                    &disk,
                    default_agent.clone(),
                    cron_prompt,
                    working_dir.clone(),
                    &mut sink,
                    if wechat_hooks.is_some() {
                        Some(&mut captured)
                    } else {
                        None
                    },
                    run_options,
                    Some(&config),
                )
                .await
                {
                    let msg = e.to_string();
                    warn!(
                        target: "anycode_scheduler",
                        "cron job {} execution error: {msg}",
                        pj.job.id
                    );
                    append_cron_run_log_with_session(
                        &pj.job.id,
                        pj.job.session_id.as_deref(),
                        next,
                        "error",
                        Some(&msg),
                    );
                    let failure_detail = crate::cron_failure::sanitize_failure_detail(&msg);
                    match pj.job.failure_destination.as_deref() {
                        Some("same_channel") => {
                            delivery
                                .route_same_channel_failure(&pj.job, &failure_detail)
                                .await;
                        }
                        Some("shell") => {
                            crate::cron_failure::route_cron_failure_shell(&pj.job, &failure_detail)
                                .await;
                        }
                        Some("http") => {
                            crate::cron_failure::route_cron_failure_http(&pj.job, &failure_detail)
                                .await;
                        }
                        _ => {}
                    }
                } else {
                    append_cron_run_log_with_session(
                        &pj.job.id,
                        pj.job.session_id.as_deref(),
                        next,
                        "ok",
                        captured.trim().get(..200),
                    );
                }
                if let Some(hooks) = &wechat_hooks {
                    let body = captured.trim();
                    if body.len() > 80 {
                        deliver_cron_to_wechat(
                            &hooks.data_root,
                            &hooks.sender,
                            pj.job.command.as_str(),
                            body,
                        )
                        .await;
                    }
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

/// How cron jobs deliver success/failure notifications.
#[derive(Clone)]
pub(crate) enum CronDelivery {
    None,
    Wechat(SchedulerWechatHooks),
}

impl CronDelivery {
    pub(crate) fn wechat_hooks(&self) -> Option<SchedulerWechatHooks> {
        match self {
            CronDelivery::None => None,
            CronDelivery::Wechat(h) => Some(h.clone()),
        }
    }

    /// `failure_destination == "same_channel"` routes through the active delivery adapter.
    pub(crate) async fn route_same_channel_failure(&self, job: &CronJob, detail: &str) {
        match self {
            CronDelivery::Wechat(hooks) => {
                deliver_cron_to_wechat(
                    &hooks.data_root,
                    &hooks.sender,
                    job.command.as_str(),
                    &format!("❌ 定时任务失败\n\n{detail}"),
                )
                .await;
            }
            CronDelivery::None => {}
        }
    }
}

/// Embed the built-in scheduler beside a long-running channel bridge (single lock per machine).
pub(crate) fn spawn_embedded_scheduler(
    config: Config,
    working_dir: PathBuf,
    shared_runtime: Arc<RwLock<Arc<anycode_agent::AgentRuntime>>>,
    delivery: CronDelivery,
    reload_secs: u64,
) {
    tracing::info!(
        target: "anycode_scheduler",
        cwd = %working_dir.display(),
        delivery = ?matches!(delivery, CronDelivery::Wechat(_)),
        "embedding built-in scheduler beside channel bridge (or exit if lock held)"
    );
    tokio::spawn(async move {
        if let Err(e) = run_builtin_scheduler(
            config,
            working_dir,
            Duration::from_secs(reload_secs),
            Some(shared_runtime),
            delivery,
        )
        .await
        {
            tracing::error!(
                target: "anycode_scheduler",
                "built-in scheduler exited: {e:#}"
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::duration_until_next_tick;
    use super::normalize_cron_schedule_expr;
    use super::ParsedJob;
    use anycode_tools::CronJob;
    use chrono::{TimeZone, Utc};
    use cron::Schedule;
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::time::Duration;

    #[test]
    fn duration_skips_historical_ticks_for_never_seen_job() {
        let now = Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap();
        let pj = ParsedJob {
            job: CronJob {
                id: "j1".into(),
                schedule: "0 0 9 * * *".into(),
                command: "ping".into(),
                session_id: None,
                failure_destination: None,
                tool_profile: None,
                tool_allowlist: None,
            },
            schedule: Schedule::from_str("0 0 9 * * *").unwrap(),
        };
        let last_fire = HashMap::new();
        let d = duration_until_next_tick(&[pj], &last_fire, now, Duration::from_secs(30));
        assert_eq!(
            d,
            Duration::from_secs(30),
            "next 9:00 UTC is tomorrow; should sleep reload_cap, not 0 (epoch backfill)"
        );
    }

    #[test]
    fn duration_after_prior_day_fire_waits_until_next_daily_tick() {
        let pj = ParsedJob {
            job: CronJob {
                id: "j1".into(),
                schedule: "0 0 9 * * *".into(),
                command: "ping".into(),
                session_id: None,
                failure_destination: None,
                tool_profile: None,
                tool_allowlist: None,
            },
            schedule: Schedule::from_str("0 0 9 * * *").unwrap(),
        };
        let mut last_fire = HashMap::new();
        last_fire.insert(
            "j1".into(),
            Utc.with_ymd_and_hms(2026, 5, 17, 9, 0, 0).unwrap(),
        );
        let now = Utc.with_ymd_and_hms(2026, 5, 18, 8, 0, 0).unwrap();
        let d = duration_until_next_tick(&[pj], &last_fire, now, Duration::from_secs(7200));
        assert_eq!(
            d,
            Duration::from_secs(3600),
            "next 9:00 UTC same day; should sleep 1h (capped below reload_cap)"
        );
    }

    #[test]
    fn duration_zero_when_daily_tick_overdue() {
        let pj = ParsedJob {
            job: CronJob {
                id: "j1".into(),
                schedule: "0 0 9 * * *".into(),
                command: "ping".into(),
                session_id: None,
                failure_destination: None,
                tool_profile: None,
                tool_allowlist: None,
            },
            schedule: Schedule::from_str("0 0 9 * * *").unwrap(),
        };
        let mut last_fire = HashMap::new();
        last_fire.insert(
            "j1".into(),
            Utc.with_ymd_and_hms(2026, 5, 17, 9, 0, 0).unwrap(),
        );
        let now = Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap();
        let d = duration_until_next_tick(&[pj], &last_fire, now, Duration::from_secs(30));
        assert_eq!(d, Duration::from_secs(0));
    }

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

    #[test]
    fn duration_for_every_fifteen_minutes_never_fired() {
        let pj = ParsedJob {
            job: CronJob {
                id: "j15".into(),
                schedule: "0 */15 * * * *".into(),
                command: "tick".into(),
                session_id: None,
                failure_destination: None,
                tool_profile: None,
                tool_allowlist: None,
            },
            schedule: Schedule::from_str("0 */15 * * * *").unwrap(),
        };
        let now = Utc.with_ymd_and_hms(2026, 5, 19, 10, 7, 30).unwrap();
        let d = duration_until_next_tick(&[pj], &HashMap::new(), now, Duration::from_secs(30));
        assert!(
            d <= Duration::from_secs(8 * 60),
            "next */15 tick should be within ~8m, got {d:?}"
        );
        assert!(d >= Duration::from_secs(1));
    }
}
