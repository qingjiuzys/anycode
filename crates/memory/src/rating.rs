//! 记忆评分回写（`memorySurveyRating` / `tengu_memory_rating_writeback`）。
//!
//! 二进制提取语义：
//! - 评分档位：`bad` / `fine` / `good`（对应 `bad_feedback_survey` / `fine_feedback_survey` /
//!   `good_feedback_survey`），映射 1 / 2 / 3。
//! - 落盘 frontmatter：
//!   ```yaml
//!   metadata:
//!     surveyRating:
//!       count: 3
//!       mean: 2.3333335
//!       total: 7
//!   ```
//! - telemetry 事件：`tengu_memory_rating_writeback`；失败日志：`memorySurveyRating: write-back failed:`。

use anycode_core::prelude::*;

/// 将用户评分档位映射为 1..=3 的数值（bad/fine/good）。
pub fn rating_to_score(rating: &str) -> Option<u8> {
    match rating.trim().to_ascii_lowercase().as_str() {
        "bad" => Some(1),
        "fine" | "ok" | "okay" => Some(2),
        "good" | "great" | "excellent" => Some(3),
        _ => None,
    }
}

/// 把一次评分写入 meta：首次评分创建 `SurveyRating`，之后累加并更新均值。
/// 返回更新后的评分统计（便于调用方写回/记录 telemetry）。
pub fn apply_survey_rating(meta: &mut Option<MemoryMetaV2>, rating: u8) -> SurveyRating {
    let meta = meta.get_or_insert_with(MemoryMetaV2::default);
    let rating = rating.clamp(1, 3);
    let survey = meta.survey_rating.get_or_insert_with(SurveyRating::default);
    survey.record(rating);
    survey.clone()
}

/// 在 markdown frontmatter 中写入/更新 `metadata:\n  surveyRating:\n    count/mean/total`。
/// frontmatter 以 `---` 开头结尾；找不到时原样返回。
pub fn write_survey_rating_frontmatter(content: &str, survey: &SurveyRating) -> String {
    let Some((head, rest)) = split_frontmatter(content) else {
        return content.to_string();
    };
    let yaml_block = format!(
        "metadata:\n  surveyRating:\n    count: {}\n    mean: {}\n    total: {}\n",
        survey.count, survey.mean, survey.total
    );
    // 若已存在 `surveyRating:` 块，替换之；否则追加到 frontmatter 末尾。
    let updated = if let Some(start) = find_survey_rating_start(head) {
        let mut out = head[..start].to_string();
        // 保留块之前的换行格式：若前一段以 `\n` 结尾则直接接上。
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&yaml_block);
        out
    } else {
        let mut out = head.to_string();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&yaml_block);
        out
    };
    format!("---{updated}---{rest}")
}

/// 解析 frontmatter 中的 `surveyRating`（`metadata.surveyRating`）。
pub fn read_survey_rating_frontmatter(content: &str) -> Option<SurveyRating> {
    let (head, _) = split_frontmatter(content)?;
    let meta: serde_json::Value = serde_yaml::from_str(head).ok()?;
    let survey = meta.get("metadata")?.get("surveyRating")?;
    serde_json::from_value(survey.clone()).ok()
}

/// 拆分 frontmatter：返回（frontmatter 不含首尾 `---`，body 含后续文本）。
fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix("---")?;
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let end_idx = rest.find("\n---\n")?;
    Some((&rest[..end_idx], &rest[end_idx + 4..]))
}

/// 定位 `surveyRating:` 起始位置（仅当位于 frontmatter 顶层 `metadata:` 之下时）。
fn find_survey_rating_start(head: &str) -> Option<usize> {
    let mut in_metadata = false;
    for (i, line) in head.lines().enumerate() {
        let trimmed = line.trim_end();
        let indent = trimmed.len() - trimmed.trim_start().len();
        if indent == 0 {
            if trimmed == "metadata:" {
                in_metadata = true;
                continue;
            }
            in_metadata = false;
        }
        if in_metadata && indent == 2 && trimmed.trim_start().starts_with("surveyRating:") {
            // 计算该行在 head 中的绝对偏移
            let mut offset = 0usize;
            for (j, l) in head.lines().enumerate() {
                if j == i {
                    return Some(offset);
                }
                offset += l.len() + 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_mapping_bad_fine_good() {
        assert_eq!(rating_to_score("bad"), Some(1));
        assert_eq!(rating_to_score("fine"), Some(2));
        assert_eq!(rating_to_score("good"), Some(3));
        assert_eq!(rating_to_score("GOOD"), Some(3));
        assert_eq!(rating_to_score("meh"), None);
    }

    #[test]
    fn apply_rating_accumulates_count_mean_total() {
        let mut meta = None;
        let s1 = apply_survey_rating(&mut meta, 1);
        assert_eq!((s1.count, s1.total), (1, 1));
        assert!((s1.mean - 1.0).abs() < 1e-6);
        let s2 = apply_survey_rating(&mut meta, 3);
        assert_eq!((s2.count, s2.total), (2, 4));
        assert!((s2.mean - 2.0).abs() < 1e-6);
        let s3 = apply_survey_rating(&mut meta, 2);
        assert_eq!((s3.count, s3.total), (3, 6));
        assert!((s3.mean - 2.0).abs() < 1e-6);
    }

    #[test]
    fn write_frontmatter_adds_block_when_absent() {
        let md = "---\nid: abc\ntitle: T\n---\n\nbody";
        let survey = SurveyRating {
            count: 2,
            mean: 2.0,
            total: 4,
        };
        let out = write_survey_rating_frontmatter(md, &survey);
        assert!(out.contains("metadata:\n  surveyRating:\n    count: 2\n    mean: 2\n    total: 4"));
        assert!(out.ends_with("body"));
        // 往返读取
        let read = read_survey_rating_frontmatter(&out).unwrap();
        assert_eq!(read.count, 2);
        assert_eq!(read.total, 4);
    }

    #[test]
    fn write_frontmatter_replaces_existing_block() {
        let md = "---\nid: abc\nmetadata:\n  surveyRating:\n    count: 1\n    mean: 1\n    total: 1\n---\n\nbody";
        let survey = SurveyRating {
            count: 3,
            mean: 2.3333335,
            total: 7,
        };
        let out = write_survey_rating_frontmatter(md, &survey);
        assert!(!out.contains("count: 1\n    mean: 1\n    total: 1"));
        assert!(out.contains("count: 3"));
        assert_eq!(out.matches("surveyRating:").count(), 1);
    }

    #[test]
    fn missing_frontmatter_returns_unchanged() {
        let plain = "no frontmatter here";
        assert_eq!(
            write_survey_rating_frontmatter(plain, &SurveyRating::default()),
            plain
        );
        assert_eq!(read_survey_rating_frontmatter(plain), None);
    }
}
