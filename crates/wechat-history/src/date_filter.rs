use crate::model::{MessageDirection, WechatChatMessage, WechatHistoryQuery};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

pub fn parse_local_date(date: &str) -> crate::Result<(NaiveDate, Tz)> {
    let date = date.trim();
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| crate::WechatHistoryError::InvalidQuery(format!("invalid date {date:?}: {e}")))
        .map(|d| (d, Tz::Asia__Shanghai))
}

pub fn resolve_timezone(query: &WechatHistoryQuery, default_tz: &str) -> crate::Result<Tz> {
    let tz_name = query
        .timezone
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(default_tz);
    tz_name.parse::<Tz>().map_err(|_| {
        crate::WechatHistoryError::InvalidQuery(format!("unknown timezone {tz_name:?}"))
    })
}

pub fn day_bounds_ms(date: NaiveDate, tz: Tz) -> crate::Result<(i64, i64)> {
    let start_local = tz
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight"))
        .single()
        .ok_or_else(|| {
            crate::WechatHistoryError::InvalidQuery(format!("ambiguous local start for {date}"))
        })?;
    let end_local = tz
        .from_local_datetime(
            &date
                .succ_opt()
                .expect("date increment")
                .and_hms_opt(0, 0, 0)
                .expect("midnight"),
        )
        .single()
        .ok_or_else(|| {
            crate::WechatHistoryError::InvalidQuery(format!("ambiguous local end for {date}"))
        })?;
    Ok((start_local.timestamp_millis(), end_local.timestamp_millis()))
}

pub fn validate_query(query: &WechatHistoryQuery) -> crate::Result<(NaiveDate, Tz, i64, i64)> {
    let (date, _) = parse_local_date(&query.date)?;
    let tz = resolve_timezone(query, "Asia/Shanghai")?;
    let (start_ms, end_ms) = day_bounds_ms(date, tz)?;
    Ok((date, tz, start_ms, end_ms))
}

pub fn filter_messages(
    mut messages: Vec<WechatChatMessage>,
    query: &WechatHistoryQuery,
    start_ms: i64,
    end_ms: i64,
    limit: usize,
) -> (Vec<WechatChatMessage>, bool) {
    messages.retain(|m| m.timestamp_ms >= start_ms && m.timestamp_ms < end_ms);

    if let Some(kw) = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let kw_lower = kw.to_lowercase();
        messages.retain(|m| {
            m.content.to_lowercase().contains(&kw_lower)
                || m.sender
                    .as_deref()
                    .is_some_and(|s| s.to_lowercase().contains(&kw_lower))
                || m.conversation_name
                    .as_deref()
                    .is_some_and(|s| s.to_lowercase().contains(&kw_lower))
                || m.attachments.iter().any(|a| {
                    a.file_name
                        .as_deref()
                        .is_some_and(|n| n.to_lowercase().contains(&kw_lower))
                })
        });
    }

    if let Some(conv) = query
        .conversation
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let conv_lower = conv.to_lowercase();
        messages.retain(|m| {
            m.conversation_id.to_lowercase().contains(&conv_lower)
                || m.conversation_name
                    .as_deref()
                    .is_some_and(|n| n.to_lowercase().contains(&conv_lower))
        });
    }

    messages.sort_by_key(|m| m.timestamp_ms);
    let truncated = messages.len() > limit;
    if truncated {
        messages.truncate(limit);
    }
    (messages, truncated)
}

pub fn format_timestamp_ms(ms: i64, tz: Tz) -> String {
    let dt: DateTime<Utc> = Utc
        .timestamp_millis_opt(ms)
        .single()
        .unwrap_or_else(Utc::now);
    dt.with_timezone(&tz)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn direction_from_is_sender(is_sender: bool) -> MessageDirection {
    if is_sender {
        MessageDirection::Outbound
    } else {
        MessageDirection::Inbound
    }
}

pub fn summarize_content(content: &str, max_chars: usize) -> String {
    let flat = content.replace('\n', " ").trim().to_string();
    if flat.chars().count() <= max_chars {
        flat
    } else {
        let mut out = String::new();
        for (i, ch) in flat.chars().enumerate() {
            if i >= max_chars.saturating_sub(1) {
                out.push('…');
                break;
            }
            out.push(ch);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WechatHistoryQuery;

    #[test]
    fn day_bounds_respect_timezone() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let tz: Tz = "Asia/Shanghai".parse().unwrap();
        let (start, end) = day_bounds_ms(date, tz).unwrap();
        assert_eq!(end - start, 86_400_000);
    }

    #[test]
    fn filter_keyword_and_limit() {
        let query = WechatHistoryQuery {
            date: "2026-06-14".into(),
            conversation: None,
            keyword: Some("invoice".into()),
            timezone: None,
            limit: Some(1),
            include_group_sender: false,
            ..Default::default()
        };
        let (_, _, start, end) = validate_query(&query).unwrap();
        let msgs = vec![
            WechatChatMessage {
                id: None,
                timestamp_ms: start + 1000,
                conversation_id: "wxid_a".into(),
                conversation_name: Some("Alice".into()),
                sender: None,
                sender_id: None,
                direction: MessageDirection::Inbound,
                msg_type: "text".into(),
                content: "send invoice please".into(),
                media_path: None,
                attachments: Vec::new(),
            },
            WechatChatMessage {
                id: None,
                timestamp_ms: start + 2000,
                conversation_id: "wxid_a".into(),
                conversation_name: Some("Alice".into()),
                sender: None,
                sender_id: None,
                direction: MessageDirection::Inbound,
                msg_type: "text".into(),
                content: "invoice attached".into(),
                media_path: None,
                attachments: Vec::new(),
            },
            WechatChatMessage {
                id: None,
                timestamp_ms: start + 3000,
                conversation_id: "wxid_b".into(),
                conversation_name: Some("Bob".into()),
                sender: None,
                sender_id: None,
                direction: MessageDirection::Inbound,
                msg_type: "text".into(),
                content: "hello".into(),
                media_path: None,
                attachments: Vec::new(),
            },
        ];
        let (filtered, truncated) = filter_messages(msgs, &query, start, end, 1);
        assert_eq!(filtered.len(), 1);
        assert!(truncated);
        assert!(filtered[0].content.contains("invoice"));
    }
}
