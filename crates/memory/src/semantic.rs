//! 语义层：LLM 提炼记忆条目（name/description）+ 语义检索默认启用。
//!
//! 二进制提取语义（`# auto memory` 模板）：
//! - 记忆 frontmatter：`name: {{short-kebab-case-slug}}`、`description: {{one-line summary …}}`、
//!   `metadata:\n  type: {{user|feedback|project|reference}}`。
//! - 指导语：`Keep the name, description, and type fields in memory files up-to-date with the content`；
//!   `Organize memory semantically by topic, not chronologically`。
//! - slug 规则：`/^[a-z0-9_-]+$/` 合法则原样，否则小写 + 非字母数字连续段折叠为 `-` 并去首尾 `-`。
//! - 引用：回复中引用记忆时用 `<memory filenames="{slug}.md">…</memory>`。
//! - telemetry：`tengu_sdk_memory_summary`；语义检索开关 `embeddingDataDeliveryEnabled`（默认开启）。

use anycode_core::prelude::*;
use serde::{Deserialize, Serialize};

/// 记忆条目提炼结果（对齐 frontmatter `name`/`description`/`metadata.type`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistilledMemoryEntry {
    /// kebab-case slug，用作文件名与 `[[name]]` 链接。
    pub name: String,
    /// 一行摘要：未来会话靠它判断相关性，因此要求具体。
    pub description: String,
    pub mem_type: MemoryType,
}

impl DistilledMemoryEntry {
    /// 从现有记忆启发式提炼（无 LLM 时的回退路径）。
    pub fn from_memory(memory: &Memory) -> Self {
        let name = slugify_name(memory.title.trim());
        let description = first_line(&memory.content, 200);
        Self {
            name,
            description,
            mem_type: memory.mem_type,
        }
    }
}

/// 把任意标题/文本转为合法 slug（合法 kebab-case 则原样，否则小写折叠非字母数字）。
pub fn slugify_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "untitled".to_string();
    }
    let legal = trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if legal {
        // 纯分隔符组成的 slug 无意义（slug 至少含一个词）。
        if !trimmed.chars().any(|c| c.is_ascii_alphanumeric()) {
            return "untitled".to_string();
        }
        return trimmed.to_string();
    }
    let mut out = String::new();
    let mut prev_dash = false;
    for c in trimmed.to_ascii_lowercase().chars() {
        let keep = c.is_ascii_lowercase() || c.is_ascii_digit();
        if keep {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// 取首行并截断到指定字符数（保留一句话，对齐 `tZc` 的一行摘要语义）。
pub fn first_line(text: &str, max_chars: usize) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    let mut s = line.chars().take(max_chars).collect::<String>();
    if line.chars().count() > max_chars {
        s.push('…');
    }
    s
}

/// 构建 LLM 提炼 prompt：给定记忆原始内容，输出 name/description/type（对齐 auto memory 指导语）。
pub fn build_distill_prompt(content: &str, mem_type: MemoryType) -> String {
    format!(
        "You are maintaining a persistent memory system.\n\
    Write a memory entry for the following content.\n\
    Keep the name, description, and type fields in memory files up-to-date with the content.\n\
    Organize memory semantically by topic, not chronologically.\n\
    Rules:\n\
    - `name`: a short kebab-case slug (lowercase letters, digits, hyphens) identifying this one fact.\n\
    - `description`: one specific line used to decide relevance in future conversations.\n\
    - `type`: one of user|feedback|project|reference (current memory type: {type}).\n\
    Output YAML only:\n\
    ```markdown\n\
    name: <slug>\n\
    description: <one-line summary>\n\
    metadata:\n\
    type: <type>\n\
    ```\n\
    Content:\n{content}",
        type = mem_type.as_storage_str(),
        content = content,
    )
}

/// 解析模型提炼输出（frontmatter 风格 YAML 或 JSON 对象）。
pub fn parse_distill_response(text: &str) -> Option<DistilledMemoryEntry> {
    let cleaned = strip_code_fence(text).trim().to_string();
    // JSON 优先（模型可能直接返回对象）。
    if cleaned.trim_start().starts_with('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&cleaned) {
            if let Some(e) = entry_from_json(&v) {
                return Some(e);
            }
        }
    }
    // YAML frontmatter：剥掉首尾 `---` 后按 yaml 解析。
    let yaml = cleaned
        .strip_prefix("---")
        .map(|s| s.strip_suffix("---").unwrap_or(s))
        .unwrap_or(&cleaned);
    let value: serde_json::Value = serde_yaml::from_str(yaml).ok()?;
    entry_from_json(&value)
}

fn entry_from_json(v: &serde_json::Value) -> Option<DistilledMemoryEntry> {
    let name = v.get("name")?.as_str()?.trim();
    let description = v
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("")
        .trim();
    let type_raw = v
        .get("metadata")
        .and_then(|m| m.get("type"))
        .and_then(|t| t.as_str())
        .or_else(|| v.get("type").and_then(|t| t.as_str()))
        .unwrap_or("");
    let mem_type = MemoryType::from_storage_str(type_raw).unwrap_or(MemoryType::Project);
    Some(DistilledMemoryEntry {
        name: slugify_name(name),
        description: description.to_string(),
        mem_type,
    })
}

fn strip_code_fence(text: &str) -> String {
    let t = text.trim();
    t.strip_prefix("```markdown")
        .or_else(|| t.strip_prefix("```yaml"))
        .or_else(|| t.strip_prefix("```"))
        .map(|s| s.strip_suffix("```").unwrap_or(s).to_string())
        .unwrap_or_else(|| t.to_string())
}

/// 在 markdown frontmatter 中写入/更新 `name`/`description`/`metadata.type`。
/// frontmatter 以 `---` 开头结尾；找不到时原样返回。
pub fn write_distilled_frontmatter(content: &str, entry: &DistilledMemoryEntry) -> String {
    let Some((head, rest)) = split_frontmatter(content) else {
        return content.to_string();
    };
    // 去掉已有的 name/description/metadata.type 行（含 metadata 顶层键若它只含 type）。
    let lines: Vec<String> = head
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("name:")
                || t.starts_with("description:")
                || t == "metadata:"
                || t.starts_with("type:"))
        })
        .map(|l| l.to_string())
        .collect();
    // 按模板顺序重建：name → description → metadata.type，其它字段（id 等）随后。
    let mut front: Vec<String> = Vec::with_capacity(4);
    front.push(format!("name: {}", entry.name));
    front.push(format!("description: {}", entry.description));
    front.push("metadata:".to_string());
    front.push(format!("  type: {}", entry.mem_type.as_storage_str()));
    front.extend(lines);
    format!("---\n{}\n---{}", front.join("\n"), rest)
}

/// 解析 frontmatter 中的 name/description/metadata.type。
pub fn read_distilled_frontmatter(content: &str) -> Option<DistilledMemoryEntry> {
    let (head, _) = split_frontmatter(content)?;
    let value: serde_json::Value = serde_yaml::from_str(head).ok()?;
    entry_from_json(&value)
}

/// 拆分 frontmatter：返回（frontmatter 不含首尾 `---`，body 含后续文本）。
fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix("---")?;
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let end_idx = rest.find("\n---\n")?;
    Some((&rest[..end_idx], &rest[end_idx + 4..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(title: &str, content: &str) -> Memory {
        Memory {
            id: "m1".into(),
            mem_type: MemoryType::Project,
            title: title.into(),
            content: content.into(),
            tags: vec![],
            scope: MemoryScope::Project,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            meta: None,
        }
    }

    #[test]
    fn slugifies_names_to_kebab_case() {
        assert_eq!(slugify_name("already-valid_1"), "already-valid_1");
        assert_eq!(slugify_name("Hello World!"), "hello-world");
        assert_eq!(slugify_name("  Rust / Tokio  ",), "rust-tokio");
        assert_eq!(slugify_name("---"), "untitled");
        assert_eq!(slugify_name(""), "untitled");
    }

    #[test]
    fn heuristic_distill_uses_title_and_first_line() {
        let m = memory(
            "Deploy flow",
            "Run `bun test` before shipping.\nThen push to main.",
        );
        let e = DistilledMemoryEntry::from_memory(&m);
        assert_eq!(e.name, "deploy-flow");
        assert!(e.description.contains("bun test"));
        assert_eq!(e.mem_type, MemoryType::Project);
    }

    #[test]
    fn parses_yaml_and_json_output() {
        let yaml = "```markdown\n---\nname: Testing Scripts\ndescription: Run bun test before shipping\nmetadata:\n  type: project\n---\n```";
        let e = parse_distill_response(yaml).expect("yaml");
        assert_eq!(e.name, "testing-scripts");
        assert!(e.description.contains("bun test"));
        assert_eq!(e.mem_type, MemoryType::Project);

        let json = r#"{"name":"Testing Scripts","description":"Run bun test","metadata":{"type":"reference"}}"#;
        let e = parse_distill_response(json).expect("json");
        assert_eq!(e.name, "testing-scripts");
        assert_eq!(e.mem_type, MemoryType::Reference);
    }

    #[test]
    fn distill_prompt_contains_guidance() {
        let p = build_distill_prompt("content here", MemoryType::User);
        assert!(p.contains("Keep the name, description, and type fields"));
        assert!(p.contains("Organize memory semantically by topic"));
        assert!(p.contains("type: user"));
        assert!(p.contains("content here"));
    }

    #[test]
    fn frontmatter_write_and_read_roundtrip() {
        let src = "---\nid: m1\n---\n\nBody text";
        let e = DistilledMemoryEntry {
            name: "deploy-flow".into(),
            description: "Run bun test".into(),
            mem_type: MemoryType::Project,
        };
        let out = write_distilled_frontmatter(src, &e);
        assert!(out.starts_with(
            "---\nname: deploy-flow\ndescription: Run bun test\nmetadata:\n  type: project\n"
        ));
        assert!(out.contains("\n---\n\nBody text"));
        let back = read_distilled_frontmatter(&out).expect("read back");
        assert_eq!(back, e);
    }

    #[test]
    fn frontmatter_replace_updates_existing_fields() {
        let src = "---\nname: old-name\ndescription: old\ntitle: t\nmetadata:\n  type: user\n---\n";
        let e = DistilledMemoryEntry {
            name: "new-name".into(),
            description: "new desc".into(),
            mem_type: MemoryType::Feedback,
        };
        let out = write_distilled_frontmatter(src, &e);
        assert!(out.contains("name: new-name"));
        assert!(out.contains("description: new desc"));
        assert!(out.contains("type: feedback"));
        // 每个字段只出现一次
        assert_eq!(out.matches("name: new-name").count(), 1);
        assert_eq!(out.matches("type: feedback").count(), 1);
    }
}
