//! Composer "grill me / 拷问" mode — Socratic plan alignment before implementation.

pub const GRILL_COMPOSER_MODE: &str = "grill";

pub fn normalize_composer_mode(raw: Option<&str>) -> Option<&'static str> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(GRILL_COMPOSER_MODE) => Some(GRILL_COMPOSER_MODE),
        Some(other) if other.eq_ignore_ascii_case(GRILL_COMPOSER_MODE) => Some(GRILL_COMPOSER_MODE),
        _ => None,
    }
}

pub fn system_append_for_mode(mode: Option<&str>, reply_lang: &str) -> Option<&'static str> {
    if normalize_composer_mode(mode).is_some() {
        Some(grill_system_append(reply_lang))
    } else {
        None
    }
}

pub fn grill_system_append(reply_lang: &str) -> &'static str {
    if reply_lang.starts_with("zh") {
        GRILL_APPEND_ZH
    } else {
        GRILL_APPEND_EN
    }
}

const GRILL_APPEND_ZH: &str = r#"## 拷问模式（Grill Me）

用户已进入 **拷问模式**：在双方达成共同理解、且用户明确允许动手之前，**禁止写代码、改文件、跑破坏性命令**。

### 流程
1. **一次只问一个问题**。必须用 `AskUserQuestion` 工具提问（不要用纯 Markdown 列表一次性抛出多个问题）。
2. **每个问题都要给出推荐答案**：把最可能正确的选项放在第一项，标签含「（推荐）」；其余 2–4 个选项覆盖常见分歧。
3. **能自己查的就别问用户**：仓库结构、已有命令、配置位置、API 路由、技能/工具能力等——先用 Read/Grep/Glob 查代码库，不要把能在代码里找到答案的问题抛给用户。
4. **等用户答完再问下一题**。收到回答后简短确认，再进入下一维度。
5. **退出**：当用户说「可以动手了」「开始实现」等，或选项里明确「理解已对齐，开始实现」时——用 3–5 条 bullet **复述共识**（目标、范围、验收、不做项），然后停止追问，等待用户下一条实施指令。

### 拷问维度（按 relevance 选，不必全问）
- 目标与成功标准（做完怎么算对）
- 范围边界（做什么 / 不做什么）
- 用户角色与交付物形态
- 约束（时间、环境、不能动的部分）
- 风险与回滚

### 语气
直接、具体、无套话；不要 emoji。"#;

const GRILL_APPEND_EN: &str = r#"## Grill Me mode

The user enabled **Grill Me**: do **not** write code, edit files, or run destructive commands until you both align and the user explicitly allows implementation.

### Protocol
1. **One question at a time**. Always use the `AskUserQuestion` tool (never dump multiple questions in Markdown).
2. **Every question includes a recommended answer**: put the best guess first with "(Recommended)" in the label; offer 2–4 other plausible options.
3. **Answer from the repo yourself**: layout, commands, config paths, APIs, skills/tools — use Read/Grep/Glob before asking the user anything you could infer from code.
4. **Wait for the user's reply** before the next question. Briefly acknowledge each answer.
5. **Exit**: When the user says "go ahead", "start implementing", or picks an option that means alignment — summarize consensus in 3–5 bullets (goal, scope, acceptance, out-of-scope), then stop grilling.

### Dimensions (pick what matters; don't exhaust a checklist)
- Goal and definition of done
- Scope in / out
- Audience and deliverable shape
- Constraints (env, time, must-not-touch)
- Risks and rollback

### Tone
Direct, specific, no filler; no emoji."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_grill_mode() {
        assert_eq!(
            normalize_composer_mode(Some("grill")),
            Some(GRILL_COMPOSER_MODE)
        );
        assert_eq!(
            normalize_composer_mode(Some(" GRILL ")),
            Some(GRILL_COMPOSER_MODE)
        );
        assert_eq!(normalize_composer_mode(Some("plan")), None);
        assert_eq!(normalize_composer_mode(None), None);
    }

    #[test]
    fn append_only_for_grill() {
        assert!(system_append_for_mode(Some("grill"), "zh").is_some());
        assert!(system_append_for_mode(Some("other"), "en").is_none());
    }
}
