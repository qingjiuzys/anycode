//! Cron 表达式：调度器按 **UTC** 求值；`CronCreate` 默认把用户/模型的「本地墙钟」转为 UTC 存储。

use chrono::{Datelike, Local, TimeZone, Timelike, Utc};
use std::str::FromStr;

/// 6 字段：`sec min hour day month weekday`（与 CLI scheduler 一致）。
pub fn normalize_cron_schedule_expr(expr: &str) -> String {
    let t = expr.trim();
    let fields: Vec<&str> = t.split_whitespace().collect();
    match fields.len() {
        5 => format!("0 {}", t),
        _ => t.to_string(),
    }
}

fn parse_u32_field(s: &str) -> Option<u32> {
    s.parse().ok()
}

/// 将「墙钟」6 段 cron（全为具体数字，非 `*`/`*/n`）转为下一次触发的 UTC 6 段表达式。
/// 用于 IM 里「3 分钟后 12:15 提醒」类登记；复杂周期表达式请显式传 `schedule_timezone: "utc"`。
pub fn wall_clock_cron_to_utc_storage(expr: &str) -> Option<String> {
    let normalized = normalize_cron_schedule_expr(expr);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }
    let sec = parse_u32_field(parts[0])?;
    let min = parse_u32_field(parts[1])?;
    let hour = parse_u32_field(parts[2])?;
    let day = parse_u32_field(parts[3])?;
    let month = parse_u32_field(parts[4])?;
    // weekday 保留原样（一次性任务常为 `*`）
    let dow = parts[5];

    let now = Local::now();
    let mut year = now.year();
    let mut dt = Local
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()?;
    if dt <= now {
        year += 1;
        dt = Local
            .with_ymd_and_hms(year, month, day, hour, min, sec)
            .single()?;
    }
    let utc = dt.with_timezone(&Utc);
    // One-shot wall-clock jobs must not pin weekday: models often pass today's DOW number
    // (e.g. `2` for Tuesday in ISO) while the `cron` crate uses Sun=1..Sat=7, which makes
    // `19 5 2` mean "19 May on a Monday" and never fire on the intended calendar day.
    let dow_stored = if dow == "*" || dow == "?" { dow } else { "*" };
    Some(format!(
        "{} {} {} {} {} {}",
        utc.second(),
        utc.minute(),
        utc.hour(),
        utc.day(),
        utc.month(),
        dow_stored
    ))
}

/// 校验 5/6 字段 cron 能否被内置调度器解析。
pub fn validate_cron_schedule_expr(expr: &str) -> Result<(), String> {
    use cron::Schedule;
    let normalized = normalize_cron_schedule_expr(expr);
    if normalized.split_whitespace().count() < 5 {
        return Err("schedule must have 5 or 6 fields (sec min hour day month weekday)".into());
    }
    Schedule::from_str(&normalized).map_err(|e| e.to_string())?;
    Ok(())
}

/// 下一次触发时间（按 **UTC** 解释已存储的表达式）。
pub fn next_fire_utc_from_stored_schedule(expr: &str) -> Option<chrono::DateTime<Utc>> {
    use cron::Schedule;
    let normalized = normalize_cron_schedule_expr(expr);
    let schedule = Schedule::from_str(&normalized).ok()?;
    let now = Utc::now();
    schedule.after(&now).next()
}

pub fn format_next_fire_human(utc: chrono::DateTime<Utc>) -> (String, String) {
    let local = utc.with_timezone(&Local);
    (
        utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        local.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_clock_stores_wildcard_weekday_for_one_shot() {
        let Some(utc_expr) = wall_clock_cron_to_utc_storage("0 33 12 19 5 2") else {
            panic!("expected conversion");
        };
        let parts: Vec<&str> = utc_expr.split_whitespace().collect();
        assert_eq!(parts.len(), 6);
        assert_eq!(
            parts[5], "*",
            "weekday must be * so job fires on calendar day; got {utc_expr}"
        );
    }

    #[test]
    fn validate_rejects_garbage_schedule() {
        assert!(validate_cron_schedule_expr("not a cron").is_err());
    }

    #[test]
    fn validate_accepts_six_field_schedule() {
        assert!(validate_cron_schedule_expr("0 0 9 * * *").is_ok());
    }

    #[test]
    fn wall_clock_converts_hour_for_positive_offset() {
        // 若本地为 UTC+8，12:15 local → 04:15 UTC（仅当测试机时区为东八区时稳定）
        let Some(utc_expr) = wall_clock_cron_to_utc_storage("0 15 12 19 5 *") else {
            return;
        };
        let parts: Vec<&str> = utc_expr.split_whitespace().collect();
        assert_eq!(parts.len(), 6);
        let h: u32 = parts[2].parse().unwrap();
        let local_h: u32 = 12;
        let offset_h = (local_h as i32 - h as i32).rem_euclid(24);
        // 允许 UTC+7..+9
        assert!(
            (7..=9).contains(&offset_h),
            "expected ~8h offset, got hour {h} from {utc_expr}"
        );
    }
}
