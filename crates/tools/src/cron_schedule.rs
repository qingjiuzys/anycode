//! Cron 表达式：调度器按 **UTC** 求值；`CronCreate` 默认把用户/模型的「本地墙钟」转为 UTC 存储。
//! `schedule_timezone` 支持 `local`、`utc`/`utc0`，以及 IANA 名称（如 `Asia/Shanghai`）。

use chrono::{Datelike, Local, TimeZone, Timelike, Utc};
use std::str::FromStr;

/// `CronCreate` 的 `schedule_timezone` 语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleTimezone {
    /// 本机 `chrono::Local` 墙钟。
    Local,
    /// 表达式字段已是 UTC（`utc` / `utc0` 同义）。
    Utc,
    /// 指定 IANA 时区的墙钟。
    Iana(chrono_tz::Tz),
}

/// 解析 `schedule_timezone`：`local`（空串同义）、`utc`/`utc0`，或 IANA 名称（大小写敏感）。
pub fn resolve_schedule_timezone(raw: &str) -> Result<ScheduleTimezone, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(ScheduleTimezone::Local);
    }
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "local" => Ok(ScheduleTimezone::Local),
        "utc" | "utc0" | "gmt" | "zulu" => Ok(ScheduleTimezone::Utc),
        _ => chrono_tz::Tz::from_str(t)
            .map(ScheduleTimezone::Iana)
            .map_err(|_| {
                format!(
                    "unsupported schedule_timezone {t:?}; use local, utc, or an IANA name like Asia/Shanghai"
                )
            }),
    }
}

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

/// 将「墙钟」6 段 cron（全为具体数字，非 `*`/`*/n`）按本机 `Local` 转为 UTC 存储表达式。
pub fn wall_clock_cron_to_utc_storage(expr: &str) -> Option<String> {
    wall_clock_cron_to_utc_storage_in_tz(expr, &Local, Local::now())
}

/// 将墙钟 cron 按指定 IANA 时区转为 UTC 存储表达式（与 [`wall_clock_cron_to_utc_storage`] 规则相同）。
pub fn wall_clock_cron_to_utc_storage_in_iana(expr: &str, tz: chrono_tz::Tz) -> Option<String> {
    let now = Utc::now().with_timezone(&tz);
    wall_clock_cron_to_utc_storage_in_tz(expr, &tz, now)
}

fn wall_clock_cron_to_utc_storage_in_tz<Tz: TimeZone>(
    expr: &str,
    tz: &Tz,
    now: chrono::DateTime<Tz>,
) -> Option<String> {
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

    let mut year = now.year();
    let mut dt = tz
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()?;
    if dt <= now {
        year += 1;
        dt = tz
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

/// Convert wall-clock cron using resolved `schedule_timezone`.
pub fn wall_clock_cron_to_utc_storage_for_timezone(
    expr: &str,
    tz: ScheduleTimezone,
) -> Option<String> {
    match tz {
        ScheduleTimezone::Local => wall_clock_cron_to_utc_storage(expr),
        ScheduleTimezone::Utc => Some(normalize_cron_schedule_expr(expr)),
        ScheduleTimezone::Iana(iana) => wall_clock_cron_to_utc_storage_in_iana(expr, iana),
    }
}

/// 校验 5/6 字段 cron 能否被内置调度器解析。
pub fn validate_cron_schedule_expr(expr: &str) -> Result<(), String> {
    use cron::Schedule;
    let normalized = normalize_cron_schedule_expr(expr);
    let field_count = normalized.split_whitespace().count();
    if field_count < 5 {
        return Err(format!(
            "schedule must have 5 or 6 fields (sec min hour day month weekday); got {field_count} in {:?}",
            expr.trim()
        ));
    }
    Schedule::from_str(&normalized).map_err(|e| {
        format!(
            "invalid cron expression {:?} (normalized {normalized:?}): {e}",
            expr.trim()
        )
    })?;
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

/// Heuristic natural-language → wall-clock 6-field cron (sec min hour day month weekday).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NaturalCronResult {
    pub schedule: String,
    pub summary: String,
}

/// Parse short Chinese/English schedule hints (no LLM). Returns `None` when unrecognized.
pub fn parse_natural_cron_hint(input: &str) -> Option<NaturalCronResult> {
    let raw = input.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.split_whitespace().count() >= 5 && validate_cron_schedule_expr(raw).is_ok() {
        return Some(NaturalCronResult {
            schedule: normalize_cron_schedule_expr(raw),
            summary: "Cron expression".into(),
        });
    }

    let lower = raw.to_ascii_lowercase();
    if lower.contains("every hour") || raw.contains("每小时") {
        return Some(NaturalCronResult {
            schedule: "0 0 * * * *".into(),
            summary: "Every hour".into(),
        });
    }

    let (hour, min) = extract_time(raw)?;
    let (hour, min) = adjust_hour_for_meridiem(hour, min, raw, &lower);

    if lower.contains("weekday") || raw.contains("工作日") {
        return Some(NaturalCronResult {
            schedule: format!("0 {min} {hour} * * 1-5"),
            summary: format!("Weekdays at {hour:02}:{min:02}"),
        });
    }

    if let Some(dow) = weekday_token(raw).or_else(|| weekday_token(&lower)) {
        return Some(NaturalCronResult {
            schedule: format!("0 {min} {hour} * * {dow}"),
            summary: format!("Weekly at {hour:02}:{min:02} (dow {dow})"),
        });
    }

    if lower.contains("daily")
        || lower.contains("every day")
        || raw.contains("每天")
        || raw.contains("每日")
    {
        return Some(NaturalCronResult {
            schedule: format!("0 {min} {hour} * * *"),
            summary: format!("Daily at {hour:02}:{min:02}"),
        });
    }

    None
}

fn extract_time(text: &str) -> Option<(u32, u32)> {
    let compact: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == '：' { ':' } else { c })
        .collect();

    if let Some(pos) = compact.find(':') {
        let hour = parse_trailing_digits(&compact[..pos])?;
        let rest = &compact[pos + 1..];
        let min = parse_leading_digits(rest).unwrap_or(0);
        if hour <= 23 && min <= 59 {
            return Some((hour, min));
        }
        return None;
    }

    if let Some(pos) = compact.find('点') {
        let hour = parse_trailing_digits(&compact[..pos])?;
        let rest = compact.get(pos + '点'.len_utf8()..)?;
        let min = parse_leading_digits(rest).unwrap_or(0);
        if hour <= 23 && min <= 59 {
            return Some((hour, min));
        }
        return None;
    }

    for token in compact.split(|c: char| !c.is_ascii_digit()) {
        if token.is_empty() {
            continue;
        }
        if let Ok(hour) = token.parse::<u32>() {
            if hour <= 23 {
                return Some((hour, 0));
            }
        }
    }
    None
}

fn parse_trailing_digits(s: &str) -> Option<u32> {
    let digits: String = s
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn parse_leading_digits(s: &str) -> Option<u32> {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn adjust_hour_for_meridiem(hour: u32, min: u32, raw: &str, lower: &str) -> (u32, u32) {
    let mut h = hour.min(23);
    let m = min.min(59);
    let pm = lower.contains("pm")
        || lower.contains("p.m")
        || raw.contains("下午")
        || raw.contains("晚上")
        || raw.contains("傍晚");
    let am = lower.contains("am")
        || lower.contains("a.m")
        || raw.contains("早上")
        || raw.contains("上午")
        || raw.contains("清晨");
    if raw.contains("中午") && h <= 11 {
        h = 12;
    } else if pm && h < 12 {
        h += 12;
    } else if am && h == 12 {
        h = 0;
    }
    (h, m)
}

fn weekday_token(text: &str) -> Option<&'static str> {
    const PAIRS: &[(&str, &str)] = &[
        ("sunday", "0"),
        ("sun", "0"),
        ("周日", "0"),
        ("星期天", "0"),
        ("星期日", "0"),
        ("monday", "1"),
        ("mon", "1"),
        ("周一", "1"),
        ("星期一", "1"),
        ("tuesday", "2"),
        ("tue", "2"),
        ("周二", "2"),
        ("星期二", "2"),
        ("wednesday", "3"),
        ("wed", "3"),
        ("周三", "3"),
        ("星期三", "3"),
        ("thursday", "4"),
        ("thu", "4"),
        ("周四", "4"),
        ("星期四", "4"),
        ("friday", "5"),
        ("fri", "5"),
        ("周五", "5"),
        ("星期五", "5"),
        ("saturday", "6"),
        ("sat", "6"),
        ("周六", "6"),
        ("星期六", "6"),
    ];
    for (needle, dow) in PAIRS {
        if text.contains(needle) {
            return Some(dow);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_clock_preserves_question_mark_weekday() {
        let Some(utc_expr) = wall_clock_cron_to_utc_storage("0 0 9 1 1 ?") else {
            panic!("expected conversion");
        };
        let parts: Vec<&str> = utc_expr.split_whitespace().collect();
        assert_eq!(
            parts[5], "?",
            "one-shot ? weekday should be preserved; got {utc_expr}"
        );
    }

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
    fn natural_cron_daily_chinese() {
        let r = parse_natural_cron_hint("每天上午8点").unwrap();
        assert_eq!(r.schedule, "0 0 8 * * *");
    }

    #[test]
    fn natural_cron_weekly_friday() {
        let r = parse_natural_cron_hint("每周五18:30").unwrap();
        assert_eq!(r.schedule, "0 30 18 * * 5");
    }

    #[test]
    fn natural_cron_weekdays() {
        let r = parse_natural_cron_hint("工作日 9:00").unwrap();
        assert_eq!(r.schedule, "0 0 9 * * 1-5");
    }

    #[test]
    fn natural_cron_english_daily() {
        let r = parse_natural_cron_hint("every day at 8am").unwrap();
        assert_eq!(r.schedule, "0 0 8 * * *");
    }

    #[test]
    fn validate_rejects_empty_schedule() {
        let err = validate_cron_schedule_expr("   ").unwrap_err();
        assert!(err.contains("5 or 6 fields"), "{err}");
    }

    #[test]
    fn validate_rejects_garbage_schedule() {
        let err = validate_cron_schedule_expr("not a cron").unwrap_err();
        assert!(err.contains("5 or 6 fields"), "{err}");
    }

    #[test]
    fn validate_error_includes_field_count() {
        let err = validate_cron_schedule_expr("0 0").unwrap_err();
        assert!(err.contains("got 2"), "{err}");
    }

    #[test]
    fn wall_clock_returns_none_for_wildcard_hour() {
        assert!(wall_clock_cron_to_utc_storage("0 0 * * * *").is_none());
    }

    #[test]
    fn validate_accepts_question_mark_weekday() {
        assert!(validate_cron_schedule_expr("0 0 9 * * ?").is_ok());
    }

    #[test]
    fn validate_accepts_six_field_schedule() {
        assert!(validate_cron_schedule_expr("0 0 9 * * *").is_ok());
    }

    #[test]
    fn validate_normalizes_five_field_schedule() {
        assert!(validate_cron_schedule_expr("0 9 * * *").is_ok());
    }

    #[test]
    fn next_fire_hourly_schedule_has_future_tick() {
        let next = next_fire_utc_from_stored_schedule("0 0 * * * *");
        assert!(next.is_some());
        assert!(next.unwrap() > Utc::now());
    }

    #[test]
    fn normalize_prepends_zero_seconds_for_five_field() {
        assert_eq!(normalize_cron_schedule_expr("15 10 * * 1"), "0 15 10 * * 1");
    }

    #[test]
    fn validate_rejects_four_field_schedule() {
        let err = validate_cron_schedule_expr("0 0 9 *").unwrap_err();
        assert!(err.contains("got 4"), "{err}");
    }

    #[test]
    fn validate_rejects_invalid_hour_field() {
        let err = validate_cron_schedule_expr("0 0 25 * * *").unwrap_err();
        assert!(err.contains("invalid cron"), "{err}");
    }

    #[test]
    fn validate_rejects_invalid_month_field() {
        let err = validate_cron_schedule_expr("0 0 9 1 13 *").unwrap_err();
        assert!(err.contains("invalid cron"), "{err}");
    }

    #[test]
    fn validate_rejects_invalid_day_of_month() {
        let err = validate_cron_schedule_expr("0 0 9 32 * *").unwrap_err();
        assert!(err.contains("invalid cron"), "{err}");
    }

    #[test]
    fn validate_rejects_seven_field_schedule() {
        let err = validate_cron_schedule_expr("0 0 9 * * * extra").unwrap_err();
        assert!(err.contains("invalid cron"), "{err}");
    }

    #[test]
    fn resolve_schedule_timezone_trims_iana_name() {
        assert_eq!(
            resolve_schedule_timezone("  Asia/Shanghai  ").unwrap(),
            ScheduleTimezone::Iana(chrono_tz::Asia::Shanghai)
        );
    }

    #[test]
    fn format_next_fire_human_includes_utc_label() {
        let utc = Utc.with_ymd_and_hms(2026, 5, 20, 4, 0, 0).unwrap();
        let (utc_s, _local) = format_next_fire_human(utc);
        assert!(utc_s.contains("UTC"), "{utc_s}");
        assert!(utc_s.starts_with("2026-05-20"), "{utc_s}");
    }

    #[test]
    fn next_fire_returns_future_utc_for_daily_schedule() {
        let next = next_fire_utc_from_stored_schedule("0 0 9 * * *");
        let Some(next) = next else {
            panic!("expected next fire for daily 09:00 UTC");
        };
        assert!(next > Utc::now());
    }

    #[test]
    fn resolve_schedule_timezone_accepts_local_utc_and_iana() {
        assert_eq!(
            resolve_schedule_timezone("local").unwrap(),
            ScheduleTimezone::Local
        );
        assert_eq!(
            resolve_schedule_timezone("utc").unwrap(),
            ScheduleTimezone::Utc
        );
        assert_eq!(
            resolve_schedule_timezone("utc0").unwrap(),
            ScheduleTimezone::Utc
        );
        assert_eq!(
            resolve_schedule_timezone("GMT").unwrap(),
            ScheduleTimezone::Utc
        );
        assert_eq!(
            resolve_schedule_timezone("Zulu").unwrap(),
            ScheduleTimezone::Utc
        );
        assert_eq!(
            resolve_schedule_timezone("Asia/Shanghai").unwrap(),
            ScheduleTimezone::Iana(chrono_tz::Asia::Shanghai)
        );
    }

    #[test]
    fn resolve_schedule_timezone_rejects_unknown_iana() {
        let err = resolve_schedule_timezone("Not/AZone").unwrap_err();
        assert!(err.contains("unsupported schedule_timezone"), "{err}");
    }

    #[test]
    fn wall_clock_iana_london_maps_noon_to_utc_eleven_in_may() {
        let Some(utc_expr) =
            wall_clock_cron_to_utc_storage_in_iana("0 0 12 19 5 *", chrono_tz::Europe::London)
        else {
            panic!("expected conversion");
        };
        let parts: Vec<&str> = utc_expr.split_whitespace().collect();
        assert_eq!(
            parts[2], "11",
            "12:00 Europe/London in May is 11:00 UTC; got {utc_expr}"
        );
    }

    #[test]
    fn wall_clock_iana_shanghai_maps_noon_to_utc_four() {
        let Some(utc_expr) =
            wall_clock_cron_to_utc_storage_in_iana("0 0 12 19 5 *", chrono_tz::Asia::Shanghai)
        else {
            panic!("expected conversion");
        };
        let parts: Vec<&str> = utc_expr.split_whitespace().collect();
        assert_eq!(parts.len(), 6);
        assert_eq!(
            parts[2], "4",
            "12:00 Asia/Shanghai is 04:00 UTC; got {utc_expr}"
        );
    }

    #[test]
    fn wall_clock_converts_using_local_timezone_offset() {
        use chrono::{Local, TimeZone, Timelike};
        let Some(utc_expr) = wall_clock_cron_to_utc_storage("0 15 12 19 5 *") else {
            panic!("expected conversion");
        };
        let parts: Vec<&str> = utc_expr.split_whitespace().collect();
        assert_eq!(parts.len(), 6);
        let utc_h: u32 = parts[2].parse().unwrap();
        let year = Local::now().year();
        let local_dt = Local
            .with_ymd_and_hms(year, 5, 19, 12, 15, 0)
            .single()
            .expect("local datetime");
        let expected_h = local_dt.with_timezone(&Utc).hour();
        assert_eq!(
            utc_h, expected_h,
            "wall clock 12:15 local should map to UTC hour {expected_h}, got {utc_expr}"
        );
    }
}
