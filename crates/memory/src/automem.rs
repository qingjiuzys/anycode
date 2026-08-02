//! Auto-memory 领域层：持久记忆（MEMORY.md 索引 + 受限子代理）。
//!
//! 本模块是与 `crates/memory` 现有本地 consolidate/dedup/promote 引擎并列的 **LLM 驱动**路径，
//! 实现 auto-memory 机制：
//! - 目录布局：`{base}/projects/{sanitized-cwd}/memory/`，入口 `MEMORY.md`。
//! - 受限工具面：只读 Bash + FileRead/Grep/Glob 无限制 + 仅 memory-dir 内的 Edit/Write。
//! - 四阶段巩固 prompt：orient → gather → consolidate → prune/index。
//! - 索引截断：`MEMORY.md` ≤ `MAX_ENTRYPOINT_LINES` 行且 ≤ `MAX_ENTRYPOINT_BYTES` 字节，超限附 WARNING。
//! - 门控：autoDream 时间门（默认 24h）+ 会话数门（默认 5）+ 锁。
//! - 互斥：`lastMemoryMessageUuid` cursor + `has_memory_writes_since`（主 agent 直写则跳过 fork）。
//!
//! 全部为纯函数/确定性逻辑，不依赖 IO 与 LLM 客户端，便于单测。
//! 调用方（agent/编排层）负责把本模块的判定接入真实的工具面与 LLM 客户端。

use std::path::{Path, PathBuf};

/// memory 子目录名（规范常量 `AUTO_MEM_DIRNAME = 'memory'`）。
pub const AUTO_MEM_DIRNAME: &str = "memory";
/// 索引入口文件名（规范常量 `AUTO_MEM_ENTRYPOINT_NAME = 'MEMORY.md'`）。
pub const AUTO_MEM_ENTRYPOINT_NAME: &str = "MEMORY.md";
/// 索引入口最大行数（规范常量 `MAX_ENTRYPOINT_LINES = 200`）。
pub const MAX_ENTRYPOINT_LINES: usize = 200;
/// 索引入口最大字节数（规范常量 `MAX_ENTRYPOINT_BYTES = 25_000`）。
pub const MAX_ENTRYPOINT_BYTES: usize = 25_000;

/// autoDream 时间门（小时），默认规范常量 `minHours = 24`。
pub const DEFAULT_DREAM_MIN_HOURS: f64 = 24.0;
/// autoDream 会话数门，默认规范常量 `minSessions = 5`。
pub const DEFAULT_DREAM_MIN_SESSIONS: usize = 5;

/// 工具名常量（与 `crates/tools` 保持一致，避免引入 crate 依赖造成循环）。
pub const TOOL_FILE_READ: &str = "FileRead";
pub const TOOL_FILE_WRITE: &str = "FileWrite";
pub const TOOL_BASH: &str = "Bash";
pub const TOOL_GLOB: &str = "Glob";
pub const TOOL_GREP: &str = "Grep";
pub const TOOL_EDIT: &str = "Edit";

// ============================================================================
// 路径解析（对齐 getAutoMemPath / isAutoMemPath）
// ============================================================================

/// 把任意项目根（cwd / git root）清洗为稳定的目录名片段。
/// 同 `sanitizePath`：空/`/` → `~`；剥离首尾 `/`、把 `/` 折叠为 `-`，
/// 去除可能导致目录穿越的 `..` 段。
pub fn sanitize_project_key(project_root: &str) -> String {
    let trimmed = project_root.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "~".to_string();
    }
    let segments = trimmed
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return "~".to_string();
    }
    segments.join("-")
}

/// 返回 auto-memory 目录：`{base}/projects/{sanitized-cwd}/memory/`。
/// 同 `getAutoMemPath()` 的默认分支。
pub fn auto_mem_path(base: impl AsRef<Path>, project_root: &str) -> PathBuf {
    base.as_ref()
        .join("projects")
        .join(sanitize_project_key(project_root))
        .join(AUTO_MEM_DIRNAME)
}

/// 返回索引入口路径：`{auto_mem_dir}/MEMORY.md`。
pub fn auto_mem_entrypoint(auto_mem_dir: impl AsRef<Path>) -> PathBuf {
    auto_mem_dir.as_ref().join(AUTO_MEM_ENTRYPOINT_NAME)
}

/// 判断绝对路径是否位于 auto-memory 目录内（同 `isAutoMemPath`）。
/// 安全：规范化后比较前缀，防止 `..` 穿越绕过。
pub fn is_auto_mem_path(absolute_path: &str, auto_mem_dir: impl AsRef<Path>) -> bool {
    let path = normalize_lexical_str(absolute_path);
    let dir = normalize_lexical_str(&auto_mem_dir.as_ref().to_string_lossy());
    // 组件级前缀比较（Path::starts_with），避免 `memory` 误匹配 `memory_x`。
    Path::new(&path).starts_with(Path::new(&dir))
}

/// 词法规范化（不访问文件系统）：解析 `.`/`..`，保留绝对性。
fn normalize_lexical_str(raw: &str) -> String {
    let mut out = PathBuf::new();
    let p = PathBuf::from(raw);
    for comp in p.components() {
        use std::path::Component::*;
        match comp {
            CurDir => {}
            ParentDir => {
                if !out.as_os_str().is_empty() {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out.to_string_lossy().into_owned()
}

// ============================================================================
// 受限工具面判定（对齐 createAutoMemCanUseTool）
// ============================================================================

/// 只读命令的判定结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BashReadOnly {
    ReadOnly,
    Write,
}

/// 判定一条 Bash 命令是否只读（`BashTool.isReadOnly` 的保守近似）。
///
/// 规则（提取/巩固子代理允许的命令类型 ls/find/grep/cat/stat/wc/head/tail）：
/// 1. 首段为命令；出现写重定向或写类命令 → Write。
/// 2. 仅允许显式白名单中的只读命令。保守近似：白名单外一律视为写（宁可多拒）。
pub fn bash_command_is_read_only(command: &str) -> BashReadOnly {
    let lower = command.to_ascii_lowercase();
    // 写重定向 / 管道写是硬性拒绝信号。
    for marker in [
        " >",
        ">>",
        ">|",
        "2>",
        "&>",
        "| tee",
        "|tee",
        "| xargs ",
        " rm ",
        " mv ",
        " cp ",
        " mkdir ",
        " touch ",
        " tee ",
        " curl ",
        " wget ",
        " dd ",
        " chmod ",
        " chown ",
        " git push",
        " git commit",
        " git add",
        " git reset",
        " git checkout",
        " npm install",
        " pip install",
        " cargo build",
        " cargo run",
        " make ",
    ] {
        if lower.contains(marker) {
            return BashReadOnly::Write;
        }
    }

    // 首段命令若为改变状态的 shell 关键字直接判写。
    let first = lower.split_whitespace().next().unwrap_or("");
    if matches!(
        first,
        "cd" | "export"
            | "unset"
            | "alias"
            | "source"
            | "kill"
            | "pkill"
            | "sleep"
            | "timeout"
            | "nohup"
            | "rm"
            | "mv"
            | "cp"
            | "mkdir"
            | "touch"
            | "tee"
            | "dd"
    ) {
        return BashReadOnly::Write;
    }

    // git：按其子命令区分（白名单里的只读子命令，其余一律视为写，含命令开头不带空格的情况）。
    if first == "git" {
        let sub = lower
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .trim_end_matches('-');
        let read_only_git = [
            "status",
            "log",
            "diff",
            "show",
            "grep",
            "ls",
            "ls-files",
            "blame",
            "rev-parse",
            "describe",
            "branch",
            "tag",
            "remote",
            "config",
            "rev-list",
            "whatchanged",
            "show-ref",
            "cat-file",
            "check-ignore",
            "check-attr",
        ];
        return if read_only_git.contains(&sub) {
            BashReadOnly::ReadOnly
        } else {
            BashReadOnly::Write
        };
    }

    // 只读白名单（保守子集，覆盖提取/巩固实际需要的命令）。
    let read_only_first = [
        "ls", "find", "grep", "cat", "stat", "wc", "head", "tail", "echo", "printf", "pwd",
        "which", "whoami", "basename", "dirname", "realpath", "file", "cut", "sort", "uniq", "tr",
        "sed", "awk", "date", "env", "git", "rg", "rgrep", "diff", "du", "df", "readlink",
    ];
    if read_only_first.contains(&first) {
        return BashReadOnly::ReadOnly;
    }
    BashReadOnly::Write
}

/// `createAutoMemCanUseTool` 等价的纯函数判定。
///
/// 返回 `Ok(())` 表示允许；`Err(理由)` 表示拒绝。规则：
/// - FileRead / Grep / Glob：无条件允许（天然只读）。
/// - Bash：仅 `bash_command_is_read_only` 为只读时允许。
/// - Edit / Write：仅当 `file_path` 位于 auto-memory 目录内时允许。
/// - 其余一律拒绝。
pub fn automem_can_use_tool(
    tool_name: &str,
    input: &serde_json::Value,
    auto_mem_dir: impl AsRef<Path>,
) -> Result<(), String> {
    match tool_name {
        TOOL_FILE_READ | TOOL_GREP | TOOL_GLOB => Ok(()),
        TOOL_BASH => {
            let command = input.get("command").and_then(|c| c.as_str()).unwrap_or("");
            if bash_command_is_read_only(command) == BashReadOnly::ReadOnly {
                Ok(())
            } else {
                Err("Only read-only shell commands are permitted in this context (ls, find, grep, cat, stat, wc, head, tail, and similar)".into())
            }
        }
        TOOL_EDIT | TOOL_FILE_WRITE => {
            match input.get("file_path").and_then(|f| f.as_str()) {
                Some(fp) if is_auto_mem_path(fp, auto_mem_dir.as_ref()) => Ok(()),
                Some(_) => Err(format!(
                    "only {TOOL_EDIT}/{TOOL_FILE_WRITE} within {} are allowed",
                    auto_mem_dir.as_ref().display()
                )),
                None => Err(format!("{tool_name} requires a `file_path` argument")),
            }
        }
        other => Err(format!(
            "only {TOOL_FILE_READ}, {TOOL_GREP}, {TOOL_GLOB}, read-only {TOOL_BASH}, and {TOOL_EDIT}/{TOOL_FILE_WRITE} within the memory directory are allowed (got `{other}`)"
        )),
    }
}

/// 受限工具面的 deny 名单（供 `prepare_tool_names_for_llm` 的 `extra_deny_names` 使用）。
/// 除 FileRead/Grep/Glob/Bash/Edit/Write 之外的工具整名剔除（Edit/Write 受目录白名单约束）。
pub fn automem_extra_deny_names(all_tools: &[&str]) -> Vec<String> {
    let allowed = [TOOL_FILE_READ, TOOL_GREP, TOOL_GLOB, TOOL_BASH];
    all_tools
        .iter()
        .filter(|t| !allowed.contains(t) && **t != TOOL_EDIT && **t != TOOL_FILE_WRITE)
        .map(|s| s.to_string())
        .collect()
}

// ============================================================================
// 四阶段巩固 prompt（对齐 consolidationPrompt.ts）
// ============================================================================

/// 目录已存在指导语（规范常量 `DIR_EXISTS_GUIDANCE`）——避免模型烧 turn 做 `ls`/`mkdir -p`。
pub const DIR_EXISTS_GUIDANCE: &str =
    "This directory already exists — write to it directly with the Write tool (do not run mkdir or check for its existence).";

/// autoDream 巩固 prompt：orient → gather → consolidate → prune/index 四阶段。
/// `extra` 为可选的附加上下文（如工具约束说明、本次待审会话列表）。
pub fn build_consolidation_prompt(memory_root: &str, transcript_dir: &str, extra: &str) -> String {
    format!(
        "# Dream: Memory Consolidation\n\n\
You are performing a dream — a reflective pass over your memory files. Synthesize what you've learned recently into durable, well-organized memories so that future sessions can orient quickly.\n\n\
Memory directory: `{memory_root}`\n\
{DIR_EXISTS_GUIDANCE}\n\n\
Session transcripts: `{transcript_dir}` (large JSONL files — grep narrowly, don't read whole files)\n\n\
**Tool constraints for this run:** only read-only shell commands (`ls`, `find`, `grep`, `cat`, `stat`, `wc`, `head`, `tail`, and similar), plus read/edit/write restricted to the memory directory. Plan your exploration with this in mind \u{2014} no need to probe.\n\n\
---\n\n\
## Phase 1 — Orient\n\n\
- `ls` the memory directory to see what already exists\n\
- Read `{entrypoint}` to understand the current index\n\
- Skim existing topic files so you improve them rather than creating duplicates\n\
- If `logs/` or `sessions/` subdirectories exist, review recent entries there\n\n\
## Phase 2 — Gather recent signal\n\n\
Look for new information worth persisting. Sources in rough priority order:\n\n\
1. **Daily logs** (`logs/YYYY/MM/YYYY-MM-DD.md`) if present — these are the append-only stream\n\
2. **Existing memories that drifted** — facts that contradict something you see in the codebase now\n\
3. **Transcript search** — if you need specific context, grep the JSONL transcripts for narrow terms:\n\
   `grep -rn \"<narrow term>\" {transcript_dir}/ --include=\"*.jsonl\" | tail -50`\n\n\
Don't exhaustively read transcripts. Look only for things you already suspect matter.\n\n\
## Phase 3 — Consolidate\n\n\
For each thing worth remembering, write or update a memory file at the top level of the memory directory. Use the memory file format and type conventions from your system prompt's auto-memory section — it's the source of truth for what to save, how to structure it, and what NOT to save.\n\n\
Focus on:\n\
- Merging new signal into existing topic files rather than creating near-duplicates\n\
- Converting relative dates (\"yesterday\", \"last week\") to absolute dates so they remain interpretable after time passes\n\
- Deleting contradicted facts — if today's investigation disproves an old memory, fix it at the source\n\n\
## Phase 4 — Prune and index\n\n\
Update `{entrypoint}` so it stays under {max_lines} lines AND under ~25KB. It's an **index**, not a dump — each entry should be one line under ~150 characters: `- [Title](file.md) — one-line hook`. Never write memory content directly into it.\n\n\
- Remove pointers to memories that are now stale, wrong, or superseded\n\
- Demote verbose entries: if an index line is over ~200 chars, it's carrying content that belongs in the topic file — shorten the line, move the detail\n\
- Add pointers to newly important memories\n\
- Resolve contradictions — if two files disagree, fix the wrong one\n\n\
---\n\n\
Return a brief summary of what you consolidated, updated, or pruned. If nothing changed (memories are already tight), say so.{extra_block}",
        memory_root = memory_root,
        transcript_dir = transcript_dir,
        entrypoint = AUTO_MEM_ENTRYPOINT_NAME,
        max_lines = MAX_ENTRYPOINT_LINES,
        extra_block = if extra.trim().is_empty() {
            String::new()
        } else {
            format!("\n\n## Additional context\n\n{extra}")
        },
    )
}

/// autoDream 工具约束说明（放在 `extra`，而非共享 prompt 体——手动 /dream 主循环用
/// 正常权限，此说明只对该受限 fork 有效，避免误导）。
pub fn dream_tool_constraints_extra(session_ids: &[String]) -> String {
    let sessions = if session_ids.is_empty() {
        String::new()
    } else {
        format!(
            "\nSessions since last consolidation ({}):\n{}",
            session_ids.len(),
            session_ids
                .iter()
                .map(|id| format!("- {id}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    format!(
        "\n\n**Tool constraints for this run:** Bash is restricted to read-only commands (`ls`, `find`, `grep`, `cat`, `stat`, `wc`, `head`, `tail`, and similar). Anything that writes, redirects to a file, or modifies state will be denied. Plan your exploration with this in mind — no need to probe.{sessions}"
    )
}

// ============================================================================
// MEMORY.md 索引截断（对齐 truncateEntrypointContent）
// ============================================================================

/// 索引入口截断结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrypointTruncation {
    pub content: String,
    pub line_count: usize,
    pub byte_count: usize,
    pub was_line_truncated: bool,
    pub was_byte_truncated: bool,
}

/// 截断 MEMORY.md 内容到行数与字节数双上限，超限附 WARNING。
/// 先按行截断（自然边界），再按字节在上限前最后一个换行处截断，避免切断行中。
pub fn truncate_entrypoint_content(raw: &str) -> EntrypointTruncation {
    let trimmed = raw.trim();
    let line_count = trimmed.lines().count();
    let byte_count = trimmed.len();

    let was_line_truncated = line_count > MAX_ENTRYPOINT_LINES;
    // 用原始字节数判断（长行正是字节上限针对的失效模式，行截断后再量会低估）。
    let was_byte_truncated = byte_count > MAX_ENTRYPOINT_BYTES;

    if !was_line_truncated && !was_byte_truncated {
        return EntrypointTruncation {
            content: trimmed.to_string(),
            line_count,
            byte_count,
            was_line_truncated,
            was_byte_truncated,
        };
    }

    let mut truncated = if was_line_truncated {
        trimmed
            .lines()
            .take(MAX_ENTRYPOINT_LINES)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        trimmed.to_string()
    };

    if truncated.len() > MAX_ENTRYPOINT_BYTES {
        let cut_at = truncated[..MAX_ENTRYPOINT_BYTES].rfind('\n');
        truncated = match cut_at {
            Some(pos) => truncated[..pos].to_string(),
            None => truncated[..MAX_ENTRYPOINT_BYTES].to_string(),
        };
    }

    let reason = match (was_byte_truncated, was_line_truncated) {
        (true, false) => format!(
            "{byte_count} bytes (limit: {MAX_ENTRYPOINT_BYTES}) — index entries are too long"
        ),
        (false, true) => format!("{line_count} lines (limit: {MAX_ENTRYPOINT_LINES})"),
        (true, true) => format!("{line_count} lines and {byte_count} bytes"),
        (false, false) => unreachable!(),
    };

    EntrypointTruncation {
        content: format!(
            "{truncated}\n\n> WARNING: {AUTO_MEM_ENTRYPOINT_NAME} is {reason}. Only part of it was loaded. Keep index entries to one line under ~200 chars; move detail into topic files."
        ),
        line_count,
        byte_count,
        was_line_truncated,
        was_byte_truncated,
    }
}

/// 构建一条索引入口（`- [Title](file.md) — one-line hook`）。
pub fn index_line(title: &str, file_name: &str, hook: &str) -> String {
    format!("- [{title}]({file_name}) — {hook}")
}

/// 索引入口构建：把一条条索引行拼成 MEMORY.md，并复用截断逻辑保证 ≤200 行 / ≤25KB。
/// 该函数是**幂等合并**（同 `mergeEntrypointContent` 的更新语义）：
/// 传入「应保留的索引行」（旧条目 + 新增条目去重后），重新生成，不追加重复。
/// `existing` 若为 `Some(旧内容)`，会保留其中未失效的索引行（由 `keep_lines` 过滤，
/// 调用方可按标题/文件名决定保留或剔除），避免重复条目。
#[derive(Clone, Default)]
pub struct EntrypointBuildInput<'a> {
    /// 已有的 MEMORY.md 原文（可空）。
    pub existing: Option<&'a str>,
    /// 需新增/更新的索引行（`index_line` 产物）。
    pub upsert_lines: &'a [String],
    /// 对既有内容中的索引行做保留判定（返回 false 的行被剔除）。空则全部保留。
    pub keep_line: Option<&'a dyn Fn(&str) -> bool>,
    /// 可选 frontmatter（`MEMORY_FRONTMATTER`，如 `projectId`/`updatedAt`）。
    pub frontmatter: Option<&'a str>,
}

/// 构建并截断 MEMORY.md 内容，返回最终内容与是否被截断。
pub fn build_entrypoint(input: EntrypointBuildInput) -> EntrypointTruncation {
    let mut lines: Vec<String> = Vec::new();

    if let Some(existing) = input.existing {
        for line in existing.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('#') || trimmed.starts_with("---") {
                continue; // 标题/frontmatter 交给下面统一重建
            }
            let keep = input.keep_line.map(|f| f(trimmed)).unwrap_or(true);
            if keep {
                lines.push(trimmed.to_string());
            }
        }
    }

    // 新增/更新行去重后追加（同一文件名不重复）。
    let mut seen: std::collections::HashSet<String> = lines.iter().cloned().collect();
    for line in input.upsert_lines {
        let t = line.trim().to_string();
        if !t.is_empty() && seen.insert(t.clone()) {
            lines.push(t);
        }
    }

    let fm = match input.frontmatter {
        Some(f) if !f.trim().is_empty() => format!("---\n{}\n---\n\n", f.trim()),
        _ => String::new(),
    };
    let body = if lines.is_empty() {
        fm.trim_end().to_string()
    } else {
        format!("{fm}{}", lines.join("\n"))
    };
    truncate_entrypoint_content(&body)
}

// ============================================================================
// transcript 组装（对齐 autoExtract 的 transcript 注入）
// ============================================================================

/// 默认 transcript 最大字符数（防止注入上下文超限）。
pub const DEFAULT_TRANSCRIPT_MAX_CHARS: usize = 48_000;

/// 把会话条目组装为紧凑 transcript 文本，供受限 agent 上下文注入。
/// `entries` 为 `(role, text)`，`role` 用 `user`/`assistant`/`tool`（由调用方序列化；
/// 工具文本可先用 `EpisodeEvent::to_structured_text` 压缩）。空 role/text 跳过；
/// 累计超过 `max_chars` 时截断（整条丢弃，不切行中）。
pub fn assemble_transcript(entries: &[(&str, &str)], max_chars: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut used = 0usize;
    for (role, text) in entries {
        let role = role.trim();
        let text = text.trim();
        if role.is_empty() || text.is_empty() {
            continue;
        }
        let line = format!("[{role}] {text}");
        if used + line.len() > max_chars {
            break;
        }
        used += line.len();
        lines.push(line);
    }
    lines.join("\n")
}

// ============================================================================
// autoExtract 编排决策（对齐 autoExtract.ts：互斥 + transcript → 注入上下文）
// ============================================================================

/// autoExtract 计划：本次 turn 是否应 fork 受限 agent 提取记忆，以及注入的上下文。
/// 纯判定：`has_memory_writes_since` 为真（主 agent 本段已直写 memory）时跳过 fork，
/// 并把 cursor 语义交给调用方（本模块不含可变 cursor 状态）。
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractPlan {
    /// 是否应 fork 受限 agent 提取记忆。
    pub should_extract: bool,
    /// 跳过原因（`should_extract == false` 时才有意义）。
    pub skip_reason: Option<String>,
    /// 组装后的 transcript 注入文本（作为受限 agent 的 context section）。
    pub injected_context: String,
}

/// 组装 autoExtract 计划。
/// - `entries`：本次 turn 的 `(role, text)` transcript 条目。
/// - `writes`：本段工具写入 `(tool_name, file_path)`，用于互斥检测。
/// - `max_chars`：transcript 注入上限（用 `DEFAULT_TRANSCRIPT_MAX_CHARS`）。
pub fn plan_extract(
    entries: &[(&str, &str)],
    writes: &[(&str, &str)],
    auto_mem_dir: impl AsRef<Path>,
    max_chars: usize,
) -> ExtractPlan {
    let injected_context = assemble_transcript(entries, max_chars);
    if has_memory_writes_since(writes, auto_mem_dir) {
        return ExtractPlan {
            should_extract: false,
            skip_reason: Some(
                "main agent already wrote to the memory directory this segment; \
                 skipping redundant fork extract"
                    .to_string(),
            ),
            injected_context,
        };
    }
    ExtractPlan {
        should_extract: !injected_context.is_empty(),
        skip_reason: None,
        injected_context,
    }
}

// ============================================================================
// 互斥：cursor + hasMemoryWritesSince
// ============================================================================

/// 判断某范围内的写入是否命中 auto-memory 路径。
/// `writes` 以 `(tool_name, file_path)` 传入，由调用方从 transcript 组装。
/// 同 `hasMemoryWritesSince`：主 agent 直写记忆时，fork 提取冗余——
/// 跳过该范围并把 cursor 推进到消息末尾，使主 agent 与后台 agent 每 turn 互斥。
pub fn has_memory_writes_since(writes: &[(&str, &str)], auto_mem_dir: impl AsRef<Path>) -> bool {
    writes.iter().any(|(tool, path)| {
        (tool == &TOOL_EDIT || tool == &TOOL_FILE_WRITE) && is_auto_mem_path(path, &auto_mem_dir)
    })
}

// ============================================================================
// autoDream 门控（对齐 autoDream.ts：时间门 → 会话数门 → 锁）
// ============================================================================

/// autoDream 门控结果。
#[derive(Debug, Clone, PartialEq)]
pub enum DreamGate {
    /// 时间门未过（距上次巩固不足 `min_hours`）。
    TimeNotElapsed { hours_since: f64, min_hours: f64 },
    /// 会话数门未过。
    NotEnoughSessions {
        session_count: usize,
        min_sessions: usize,
    },
    /// 已持锁（其它进程正在巩固），跳过。
    Locked,
    /// 全部通过，可触发巩固。
    Open {
        hours_since: f64,
        session_count: usize,
    },
}

/// 门控纯判定（不含锁的 IO，`locked` 由调用方读取锁文件后传入）。
pub fn dream_gate(
    hours_since_last: f64,
    session_count: usize,
    min_hours: f64,
    min_sessions: usize,
    locked: bool,
) -> DreamGate {
    if locked {
        return DreamGate::Locked;
    }
    if hours_since_last < min_hours {
        return DreamGate::TimeNotElapsed {
            hours_since: hours_since_last,
            min_hours,
        };
    }
    if session_count < min_sessions {
        return DreamGate::NotEnoughSessions {
            session_count,
            min_sessions,
        };
    }
    DreamGate::Open {
        hours_since: hours_since_last,
        session_count,
    }
}

// ============================================================================
// 引擎决策：LLM 驱动 vs 本地规则引擎降级
// ============================================================================

/// auto-memory 编排引擎选择结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomemEngine {
    /// LLM 驱动：受限后台 fork agent + 四阶段巩固 prompt 写 `MEMORY.md`。
    Llm,
    /// 回退本地规则引擎（`consolidate_episodes` / dedup / promote + 向量检索）。
    Local,
}

/// 依据配置与 LLM 可用性决定 auto-memory 走哪条引擎。
///
/// 仅当 `automem.enabled && automem.fork_agent` 且 LLM 可用（provider/key 就绪）时走 LLM 驱动；
/// 否则回退本地规则引擎，保证在无 LLM 或未开启时仍保留 dedup/promote/forget + 向量检索。
pub fn resolve_automem_engine(
    settings: &anycode_core::AutomemSettings,
    llm_available: bool,
) -> AutomemEngine {
    if settings.enabled && settings.fork_agent && llm_available {
        AutomemEngine::Llm
    } else {
        AutomemEngine::Local
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mem_dir() -> PathBuf {
        auto_mem_path("/home/u/.anycode", "/repo/src")
    }

    #[test]
    fn sanitize_project_key_folds_segments() {
        assert_eq!(sanitize_project_key("/repo/src"), "repo-src");
        assert_eq!(sanitize_project_key("/"), "~");
        assert_eq!(sanitize_project_key(""), "~");
        assert_eq!(sanitize_project_key("/a/b/../c"), "a-b-c");
    }

    #[test]
    fn auto_mem_path_layout_matches_spec() {
        let p = auto_mem_path("/home/u/.anycode", "/repo");
        assert_eq!(p, PathBuf::from("/home/u/.anycode/projects/repo/memory"));
        assert_eq!(
            auto_mem_entrypoint(&p),
            PathBuf::from("/home/u/.anycode/projects/repo/memory/MEMORY.md")
        );
    }

    #[test]
    fn is_auto_mem_path_matches_inside_and_rejects_traversal() {
        let dir = mem_dir();
        assert!(is_auto_mem_path(
            "/home/u/.anycode/projects/repo-src/memory/user_role.md",
            &dir
        ));
        assert!(is_auto_mem_path(
            "/home/u/.anycode/projects/repo-src/memory/sub/note.md",
            &dir
        ));
        assert!(!is_auto_mem_path(
            "/home/u/.anycode/projects/repo-src/memory/../../.ssh/keys",
            &dir
        ));
        assert!(!is_auto_mem_path(
            "/home/u/.anycode/projects/repo-src/memory_x/note.md",
            &dir
        ));
    }

    #[test]
    fn bash_read_only_classifies_safe_and_unsafe() {
        assert_eq!(
            bash_command_is_read_only("ls -la /repo"),
            BashReadOnly::ReadOnly
        );
        assert_eq!(
            bash_command_is_read_only("grep -rn error /repo --include='*.jsonl'"),
            BashReadOnly::ReadOnly
        );
        assert_eq!(
            bash_command_is_read_only("cat memory/MEMORY.md"),
            BashReadOnly::ReadOnly
        );
        assert_eq!(bash_command_is_read_only("rm -rf /"), BashReadOnly::Write);
        assert_eq!(
            bash_command_is_read_only("echo x > /tmp/out"),
            BashReadOnly::Write
        );
        assert_eq!(
            bash_command_is_read_only("git status"),
            BashReadOnly::ReadOnly
        );
        assert_eq!(
            bash_command_is_read_only("git commit -m x"),
            BashReadOnly::Write
        );
        assert_eq!(bash_command_is_read_only("cd /repo"), BashReadOnly::Write);
    }

    #[test]
    fn automem_can_use_tool_grants_limited_surface() {
        let dir = mem_dir();
        let in_dir = "/home/u/.anycode/projects/repo-src/memory/note.md";
        let out_dir = "/tmp/outside.md";

        assert!(automem_can_use_tool("FileRead", &serde_json::json!({}), &dir).is_ok());
        assert!(automem_can_use_tool("Grep", &serde_json::json!({}), &dir).is_ok());
        assert!(automem_can_use_tool("Glob", &serde_json::json!({}), &dir).is_ok());

        assert!(automem_can_use_tool("Bash", &serde_json::json!({"command": "ls"}), &dir).is_ok());
        assert!(
            automem_can_use_tool("Bash", &serde_json::json!({"command": "rm -rf /"}), &dir)
                .is_err()
        );

        assert!(
            automem_can_use_tool("Edit", &serde_json::json!({"file_path": in_dir}), &dir).is_ok()
        );
        assert!(
            automem_can_use_tool("FileWrite", &serde_json::json!({"file_path": in_dir}), &dir)
                .is_ok()
        );
        assert!(
            automem_can_use_tool("Edit", &serde_json::json!({"file_path": out_dir}), &dir).is_err()
        );

        assert!(automem_can_use_tool("WebFetch", &serde_json::json!({}), &dir).is_err());
        assert!(automem_can_use_tool("Agent", &serde_json::json!({}), &dir).is_err());
    }

    #[test]
    fn automem_extra_deny_names_drops_unlisted() {
        let all = [
            "FileRead",
            "Grep",
            "Glob",
            "Bash",
            "Edit",
            "FileWrite",
            "WebFetch",
            "Agent",
            "Skill",
            "mcp__srv__tool",
        ];
        let denied = automem_extra_deny_names(&all);
        for d in ["WebFetch", "Agent", "Skill", "mcp__srv__tool"] {
            assert!(denied.contains(&d.to_string()));
        }
        for keep in ["FileRead", "Grep", "Glob", "Bash", "Edit", "FileWrite"] {
            assert!(!denied.contains(&keep.to_string()));
        }
    }

    #[test]
    fn consolidation_prompt_contains_four_phases_and_constraints() {
        let p = build_consolidation_prompt("/mem", "/transcripts", "## Additional context\n\nnote");
        for needle in [
            "Phase 1 — Orient",
            "Phase 2 — Gather recent signal",
            "Phase 3 — Consolidate",
            "Phase 4 — Prune and index",
            "MEMORY.md",
            "only read-only",
            "Additional context",
        ] {
            assert!(p.contains(needle), "missing {needle}");
        }
        assert!(p.contains(&MAX_ENTRYPOINT_LINES.to_string()));
    }

    #[test]
    fn dream_tool_constraints_extra_lists_sessions() {
        let extra = dream_tool_constraints_extra(&["s1".into(), "s2".into()]);
        assert!(extra.contains("- s1"));
        assert!(extra.contains("- s2"));
        assert!(extra.contains("read-only"));
        let empty = dream_tool_constraints_extra(&[]);
        assert!(!empty.contains("- s"));
    }

    #[test]
    fn truncate_entrypoint_content_passes_small() {
        let raw = "- [a](a.md) — hook\n- [b](b.md) — hook";
        let t = truncate_entrypoint_content(raw);
        assert!(!t.was_line_truncated && !t.was_byte_truncated);
        assert_eq!(t.content, raw);
    }

    #[test]
    fn truncate_entrypoint_content_line_truncates() {
        let raw = (0..210)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let t = truncate_entrypoint_content(&raw);
        assert!(t.was_line_truncated);
        assert_eq!(t.content.lines().count(), MAX_ENTRYPOINT_LINES + 2);
        assert!(t.content.contains("WARNING"));
    }

    #[test]
    fn truncate_entrypoint_content_byte_truncates() {
        let raw = "x".repeat(MAX_ENTRYPOINT_BYTES + 100);
        let t = truncate_entrypoint_content(&raw);
        assert!(t.was_byte_truncated);
        assert!(!t.was_line_truncated);
        assert!(t.content.len() <= MAX_ENTRYPOINT_BYTES + 300);
        assert!(t.content.contains("WARNING"));
    }

    #[test]
    fn build_entrypoint_merges_dedupes_and_frontmatter() {
        let old = "- [old](old.md) — hook\n- [dup](dup.md) — stale";
        let upsert = vec![
            "- [new](new.md) — fresh".to_string(),
            "- [dup](dup.md) — updated".to_string(),
        ];
        let t = build_entrypoint(EntrypointBuildInput {
            existing: Some(old),
            upsert_lines: &upsert,
            keep_line: Some(&|line: &str| !line.contains("stale")),
            frontmatter: Some("projectId: p\nupdatedAt: 2026-08-02"),
        });
        let content = t.content;
        assert!(content.starts_with("---\nprojectId: p"));
        assert!(content.contains("- [old](old.md)"));
        assert!(content.contains("- [dup](dup.md) — updated"));
        assert!(!content.contains("stale"));
        assert_eq!(content.matches("- [dup](dup.md)").count(), 1);
        assert!(!t.was_line_truncated);
    }

    #[test]
    fn build_entrypoint_empty_keeps_frontmatter_only() {
        let t = build_entrypoint(EntrypointBuildInput {
            existing: None,
            upsert_lines: &[],
            keep_line: None,
            frontmatter: Some("projectId: p"),
        });
        assert_eq!(t.content, "---\nprojectId: p\n---");
    }

    #[test]
    fn assemble_transcript_skips_empty_and_truncates() {
        let entries = [
            ("user", "  "),
            ("user", "build the parser"),
            ("tool", "Bash ok=true"),
            ("assistant", "done"),
        ];
        let t = assemble_transcript(&entries, usize::MAX);
        assert_eq!(
            t,
            "[user] build the parser\n[tool] Bash ok=true\n[assistant] done"
        );

        let small = assemble_transcript(&entries, 44);
        assert!(small.len() <= 44);
        assert_eq!(small, "[user] build the parser\n[tool] Bash ok=true");
    }

    #[test]
    fn plan_extract_skips_when_main_agent_wrote_memory() {
        let dir = mem_dir();
        let entries = [("user", "fix parser"), ("assistant", "did it")];
        let in_dir = "/home/u/.anycode/projects/repo-src/memory/note.md";
        let p = plan_extract(&entries, &[("Edit", in_dir)], &dir, 1000);
        assert!(!p.should_extract);
        assert!(p.skip_reason.is_some());
        assert!(p.injected_context.contains("[user] fix parser"));
    }

    #[test]
    fn plan_extract_extracts_without_memory_writes() {
        let dir = mem_dir();
        let entries = [("user", "fix parser")];
        let p = plan_extract(&entries, &[("Edit", "/tmp/out.rs")], &dir, 1000);
        assert!(p.should_extract);
        assert_eq!(p.skip_reason, None);
        assert_eq!(p.injected_context, "[user] fix parser");
    }

    #[test]
    fn has_memory_writes_since_detects_automem_writes() {
        let dir = mem_dir();
        let in_dir = "/home/u/.anycode/projects/repo-src/memory/note.md";
        assert!(has_memory_writes_since(&[("Edit", in_dir)], &dir));
        assert!(has_memory_writes_since(&[("FileWrite", in_dir)], &dir));
        assert!(!has_memory_writes_since(&[("Edit", "/tmp/out.md")], &dir));
        assert!(!has_memory_writes_since(&[("Read", in_dir)], &dir));
    }

    #[test]
    fn resolve_engine_llm_only_when_enabled_fork_and_available() {
        let on = anycode_core::AutomemSettings {
            enabled: true,
            fork_agent: true,
            ..Default::default()
        };
        assert_eq!(resolve_automem_engine(&on, true), AutomemEngine::Llm);
        // LLM 不可用 → 回退本地。
        assert_eq!(resolve_automem_engine(&on, false), AutomemEngine::Local);
        // 开关未开 → 本地。
        let off = anycode_core::AutomemSettings {
            enabled: false,
            fork_agent: true,
            ..Default::default()
        };
        assert_eq!(resolve_automem_engine(&off, true), AutomemEngine::Local);
        // fork 关闭 → 本地。
        let no_fork = anycode_core::AutomemSettings {
            enabled: true,
            fork_agent: false,
            ..Default::default()
        };
        assert_eq!(resolve_automem_engine(&no_fork, true), AutomemEngine::Local);
    }

    #[test]
    fn dream_gate_orders_checks() {
        assert_eq!(
            dream_gate(1.0, 10, 24.0, 5, false),
            DreamGate::TimeNotElapsed {
                hours_since: 1.0,
                min_hours: 24.0
            }
        );
        assert_eq!(
            dream_gate(30.0, 2, 24.0, 5, false),
            DreamGate::NotEnoughSessions {
                session_count: 2,
                min_sessions: 5
            }
        );
        assert_eq!(dream_gate(30.0, 10, 24.0, 5, true), DreamGate::Locked);
        assert_eq!(
            dream_gate(30.0, 10, 24.0, 5, false),
            DreamGate::Open {
                hours_since: 30.0,
                session_count: 10
            }
        );
    }
}
