//! 响应式压缩（与 Claude Code 2.1.218 `services/compact` reactive 路径对齐）。
//!
//! 二进制提取语义：
//! - `gap_guided`：按组间隙（gap）引导步进；PTL 后扩大 gap、缩小 step。
//! - `too_few_groups`：组数 < 2 时不压缩（「fewer than 2 groups, nothing to compact」）。
//! - `prompt_too_long` / `media_too_large` / `media_unstrippable`：三种重试/中止原因。
//! - `Reactive compact: attempt <n> hit prompt-too-long (gap=… step …), next preserves …`。
//! - `tengu_compact_credits_clamp_rescue`：credits 余额低时 clamp 输出 token 上限。
//! - `messagesToKeep`：步进后保留的尾部消息。

use anycode_core::prelude::*;

/// 与 Claude `COMPACT_MAX_OUTPUT_TOKENS` 同量级。
pub const REACTIVE_MAX_OUTPUT_TOKENS: u32 = 20_000;
/// 与 Claude `MAX_COMPACT_PTL_RETRIES` 对齐。
pub const REACTIVE_MAX_PTL_RETRIES: usize = 3;
/// 少于该组数不压缩（Claude 消息「fewer than 2 groups, nothing to compact」）。
pub const REACTIVE_MIN_GROUPS: usize = 2;

/// 压缩/中止/失败的原因分类（对齐 Claude telemetry 的 gap_guided 等标签）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactiveAbortReason {
    /// `too_few_groups`：组数不足。
    TooFewGroups,
    /// `media_unstrippable`：媒体过大且无法剥离。
    MediaUnstrippable,
    /// `exhausted`：重试预算耗尽。
    Exhausted,
}

/// 响应式压缩计划：把 `to_summarize` 组折叠为摘要，保留 `preserve_from` 起的尾部组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactivePlan {
    /// 需要折叠进摘要的组索引（从旧到新）。
    pub to_summarize: Vec<usize>,
    /// 保留的组起始索引（即 `messages_to_keep` 边界）。
    pub preserve_from: usize,
    /// 最后使用的 gap 值（PTL 重试后扩大）。
    pub gap: u32,
    /// 成功压缩的组数；`0` 表示放弃。
    pub summarized_groups: usize,
}

/// 一组会话消息：`user`（若无则 `None`）+ `assistant` 及其后续 `tool` 结果。
#[derive(Debug, Clone)]
pub struct TurnGroup {
    pub user: Option<Message>,
    pub assistant: Message,
    pub tools: Vec<Message>,
}

impl TurnGroup {
    /// 粗略 token 估计：字符数 / 4（对齐 Claude 的估算意图，非精确计数）。
    pub fn estimated_tokens(&self) -> u32 {
        let mut chars = 0usize;
        let mut count = |m: &Message| match &m.content {
            MessageContent::Text(t) => chars += t.len(),
            MessageContent::ToolResult { content, .. } => chars += content.len(),
            _ => {}
        };
        if let Some(u) = &self.user {
            count(u);
        }
        count(&self.assistant);
        for t in &self.tools {
            count(t);
        }
        (chars / 4).max(1) as u32
    }

    pub fn messages(&self) -> Vec<Message> {
        let mut out = Vec::new();
        if let Some(u) = &self.user {
            out.push(u.clone());
        }
        out.push(self.assistant.clone());
        out.extend(self.tools.iter().cloned());
        out
    }
}

/// 按「user+assistant+tool」轮次分组：user 开启新组；assistant 补入当前组；
/// tool 归属当前 assistant；连续 user 追加到组内 user 列表（保持顺序）。
pub fn group_turns(session: &[Message]) -> Vec<TurnGroup> {
    let mut groups: Vec<TurnGroup> = Vec::new();
    for m in session {
        match m.role {
            MessageRole::System => {}
            MessageRole::User => {
                if let Some(last) = groups.last_mut() {
                    if !assistant_has_content(&last.assistant) && last.tools.is_empty() {
                        last.user = Some(m.clone());
                        continue;
                    }
                }
                groups.push(TurnGroup {
                    user: Some(m.clone()),
                    assistant: empty_assistant(m.timestamp),
                    tools: Vec::new(),
                });
            }
            MessageRole::Assistant => {
                if let Some(last) = groups.last_mut() {
                    if !assistant_has_content(&last.assistant) && last.tools.is_empty() {
                        last.assistant = m.clone();
                        continue;
                    }
                }
                groups.push(TurnGroup {
                    user: None,
                    assistant: m.clone(),
                    tools: Vec::new(),
                });
            }
            MessageRole::Tool => {
                if let Some(last) = groups.last_mut() {
                    last.tools.push(m.clone());
                }
            }
        }
    }
    groups
}

fn empty_assistant(timestamp: chrono::DateTime<chrono::Utc>) -> Message {
    Message {
        id: uuid::Uuid::new_v4(),
        role: MessageRole::Assistant,
        content: MessageContent::Text(String::new()),
        timestamp,
        metadata: std::collections::HashMap::new(),
    }
}

/// 响应式压缩选项。
#[derive(Debug, Clone, Copy)]
pub struct ReactiveCompactOptions {
    /// 最大输出 token（受 credits clamp rescue 影响）。
    pub max_output_tokens: u32,
    /// PTL 重试预算。
    pub max_ptl_retries: usize,
    /// 起始 gap（步进宽度）。
    pub initial_gap: u32,
    /// PTL 后 gap 放大倍数。
    pub gap_multiplier: u32,
}

impl Default for ReactiveCompactOptions {
    fn default() -> Self {
        Self {
            max_output_tokens: REACTIVE_MAX_OUTPUT_TOKENS,
            max_ptl_retries: REACTIVE_MAX_PTL_RETRIES,
            initial_gap: 1,
            gap_multiplier: 2,
        }
    }
}

/// credits clamp rescue：余额低时把输出 token 上限压到余额（Claude `tengu_compact_credits_clamp_rescue`）。
/// `None` 表示无余额信息（不 clamp）。
pub fn clamp_output_tokens_to_credits(
    max_output_tokens: u32,
    credits_remaining: Option<u32>,
) -> u32 {
    match credits_remaining {
        Some(credits) if credits > 0 => max_output_tokens.min(credits),
        Some(_) => 1,
        None => max_output_tokens,
    }
}

/// gap-guided 步进决策：从最旧组开始折叠；`gap` 决定每次跳过/保留的间隔。
/// 返回建议的 `preserve_from`（保留起始组索引），保证至少保留 1 组且不全部折叠。
pub fn gap_guided_preserve_from(group_count: usize, gap: u32) -> usize {
    if group_count == 0 {
        return 0;
    }
    let gap = gap.max(1) as usize;
    let preserve = group_count.saturating_sub(gap).max(1);
    preserve.min(group_count)
}

/// 主计划函数：给定分组与选项，输出响应式压缩计划。
/// 若组数不足或计划无效返回 `None`（对应 Claude `too_few_groups` / bail）。
pub fn plan_reactive_compact(
    groups: &[TurnGroup],
    opts: &ReactiveCompactOptions,
) -> Result<ReactivePlan, ReactiveAbortReason> {
    if groups.len() < REACTIVE_MIN_GROUPS {
        return Err(ReactiveAbortReason::TooFewGroups);
    }
    // 必须至少有一个 assistant 消息可摘要；空 assistant 组（孤立 user）不算。
    if !groups.iter().any(|g| assistant_has_content(&g.assistant)) {
        return Err(ReactiveAbortReason::TooFewGroups);
    }
    let mut gap = opts.initial_gap.max(1);
    let mut preserve_from = gap_guided_preserve_from(groups.len(), gap);
    let mut retries = 0usize;
    loop {
        let to_summarize: Vec<usize> = (0..preserve_from).collect();
        let tokens = to_summarize
            .iter()
            .map(|&i| groups[i].estimated_tokens())
            .sum::<u32>();
        if tokens == 0 || to_summarize.is_empty() {
            return Err(ReactiveAbortReason::TooFewGroups);
        }
        // 折叠后的摘要输出接近输出上限 → 视为一次 PTL 类失败：预算内扩大 gap、保留更多尾部组。
        if tokens >= opts.max_output_tokens.saturating_mul(4) {
            if retries >= opts.max_ptl_retries {
                return Err(ReactiveAbortReason::Exhausted);
            }
            retries += 1;
            gap = gap.saturating_mul(opts.gap_multiplier.max(1));
            let next = gap_guided_preserve_from(groups.len(), gap);
            if next == preserve_from {
                // 已无更小步进可退（gap 已大到 preserve 恒为 1），继续也是浪费预算。
                return Err(ReactiveAbortReason::Exhausted);
            }
            preserve_from = next;
            continue;
        }
        let summarized_groups = to_summarize.len();
        return Ok(ReactivePlan {
            to_summarize,
            preserve_from,
            gap,
            summarized_groups,
        });
    }
}

/// PTL 重试步进：按 gap-guided 计划保留更多尾部组，重建摘要输入消息（对齐 Claude
/// `Reactive compact: attempt <n> hit prompt-too-long (gap=… step …), next preserves …`）。
/// 返回 `false` 表示无法继续缩小（组数不足或已无更小步进）。
pub fn apply_ptl_step(messages: &mut Vec<Message>, opts: &ReactiveCompactOptions) -> bool {
    let groups = group_turns(messages);
    let plan = match plan_reactive_compact(&groups, opts) {
        Ok(p) => p,
        Err(_) => return false,
    };
    if plan.preserve_from >= groups.len() {
        return false;
    }
    let mut out: Vec<Message> = Vec::new();
    if let Some(first) = messages.first() {
        if first.role == MessageRole::System {
            out.push(first.clone());
        }
    }
    for g in &groups[plan.preserve_from..] {
        out.extend(g.messages());
    }
    if out.len() >= messages.len() {
        return false;
    }
    *messages = out;
    true
}

/// 从会话与计划构建「摘要输入」：待折叠组的全部消息（供响应式部分摘要的 LLM 调用）。
pub fn build_reactive_summarize_set(session: &[Message], plan: &ReactivePlan) -> Vec<Message> {
    let groups = group_turns(session);
    let mut out = Vec::new();
    for &i in &plan.to_summarize {
        if let Some(g) = groups.get(i) {
            out.extend(g.messages());
        }
    }
    out
}

/// 消息是否「内容为空」的辅助判断（独立函数，便于测试）。
pub fn assistant_has_content(m: &Message) -> bool {
    match &m.content {
        MessageContent::Text(t) => !t.trim().is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: MessageRole, text: &str) -> Message {
        Message {
            id: uuid::Uuid::new_v4(),
            role,
            content: MessageContent::Text(text.into()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    fn session_of(n_assistant: usize) -> Vec<Message> {
        let mut out = vec![msg(MessageRole::System, "sys")];
        for i in 0..n_assistant {
            out.push(msg(MessageRole::User, &format!("user {i}")));
            out.push(msg(MessageRole::Assistant, &format!("assistant {i}")));
        }
        out
    }

    #[test]
    fn group_turns_merges_user_assistant_and_tool() {
        let mut s = session_of(2);
        let tool = Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Tool,
            content: MessageContent::ToolResult {
                tool_use_id: "t1".into(),
                content: "result".into(),
                is_error: false,
            },
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };
        // 末尾追加 tool 结果：属于最后一个 assistant 轮次（Claude 消息序中 tool 紧随 assistant）。
        s.push(tool.clone());
        let groups = group_turns(&s);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[1].tools.len(), 1);
        let user_text = match &groups[1].user.as_ref().unwrap().content {
            MessageContent::Text(t) => t.clone(),
            _ => String::new(),
        };
        assert_eq!(user_text, "user 1");
    }

    #[test]
    fn too_few_groups_is_rejected() {
        let s = vec![
            msg(MessageRole::System, "sys"),
            msg(MessageRole::User, "hi"),
        ];
        let groups = group_turns(&s);
        let plan = plan_reactive_compact(&groups, &ReactiveCompactOptions::default());
        assert_eq!(plan, Err(ReactiveAbortReason::TooFewGroups));
    }

    #[test]
    fn gap_guided_step_preserves_at_least_one_group() {
        let groups = group_turns(&session_of(5));
        assert_eq!(groups.len(), 5);
        let p = plan_reactive_compact(&groups, &ReactiveCompactOptions::default()).expect("plan");
        assert!(p.preserve_from >= 1);
        assert!(p.preserve_from <= 5);
        assert_eq!(p.summarized_groups, p.preserve_from);
    }

    #[test]
    fn pti_like_growth_expands_gap_and_keeps_more() {
        // 大体积组：初始 gap=1 时会把 3 组都折叠，token 超限 → 重试后 gap 翻倍、保留更多。
        let mut s = vec![msg(MessageRole::System, "sys")];
        for i in 0..3 {
            s.push(msg(
                MessageRole::User,
                &format!("user {i} {}", "x".repeat(400)),
            ));
            s.push(msg(
                MessageRole::Assistant,
                &format!("assistant {i} {}", "y".repeat(400)),
            ));
        }
        let groups = group_turns(&s);
        let big_opts = ReactiveCompactOptions {
            max_output_tokens: 100,
            max_ptl_retries: 3,
            initial_gap: 1,
            gap_multiplier: 2,
        };
        let p = plan_reactive_compact(&groups, &big_opts).expect("plan");
        assert!(p.gap >= 2);
        assert!(p.preserve_from >= 1);
    }

    #[test]
    fn credits_clamp_rescue_caps_output_tokens() {
        assert_eq!(clamp_output_tokens_to_credits(20_000, Some(5_000)), 5_000);
        assert_eq!(clamp_output_tokens_to_credits(20_000, Some(0)), 1);
        assert_eq!(clamp_output_tokens_to_credits(20_000, None), 20_000);
        assert_eq!(clamp_output_tokens_to_credits(20_000, Some(50_000)), 20_000);
    }

    #[test]
    fn assistant_has_content_detects_empty_text() {
        assert!(assistant_has_content(&msg(MessageRole::Assistant, "x")));
        assert!(!assistant_has_content(&msg(MessageRole::Assistant, "  ")));
    }
}
