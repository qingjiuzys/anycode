//! Localized strings for Markdown / UI report rendering.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn parse(raw: &str) -> Self {
        if raw.trim().to_ascii_lowercase().starts_with("zh") {
            Lang::Zh
        } else {
            Lang::En
        }
    }
}

pub struct ReportStrings {
    pub doc_title: &'static str,
    pub scope_project: &'static str,
    pub scope_session: &'static str,
    pub label_project: &'static str,
    pub label_session: &'static str,
    pub label_root: &'static str,
    pub label_generated: &'static str,
    pub section_summary: &'static str,
    pub section_trust: &'static str,
    pub section_sessions: &'static str,
    pub section_gates: &'static str,
    pub section_failures: &'static str,
    pub section_artifacts: &'static str,
    pub section_reproduce: &'static str,
    pub section_prompt: &'static str,
    pub section_summary_text: &'static str,
    pub section_events: &'static str,
    pub summary_sessions: &'static str,
    pub summary_events_sampled: &'static str,
    pub summary_failed_gates: &'static str,
    pub summary_artifacts: &'static str,
    pub summary_trusted: &'static str,
    pub no_sessions: &'static str,
    pub no_gates: &'static str,
    pub no_failures: &'static str,
    pub no_artifacts: &'static str,
    pub no_events: &'static str,
    pub imported_collapsed: &'static str,
    pub reproduce_hint: &'static str,
    pub gate_required: &'static str,
    pub gate_optional: &'static str,
}

pub fn strings(lang: Lang) -> ReportStrings {
    match lang {
        Lang::Zh => ReportStrings {
            doc_title: "anycode 数字工作台报告",
            scope_project: "项目",
            scope_session: "会话",
            label_project: "项目",
            label_session: "会话",
            label_root: "根目录",
            label_generated: "生成时间",
            section_summary: "概览",
            section_trust: "信任与交付",
            section_sessions: "近期会话",
            section_gates: "验收门禁",
            section_failures: "近期失败与告警",
            section_artifacts: "资产",
            section_reproduce: "本地复现",
            section_prompt: "任务摘要",
            section_summary_text: "会话总结",
            section_events: "近期事件",
            summary_sessions: "会话总数",
            summary_events_sampled: "本报告采样事件数（上限）",
            summary_failed_gates: "失败门禁",
            summary_artifacts: "资产条目",
            summary_trusted: "会话信任状态",
            no_sessions: "暂无会话记录。",
            no_gates: "暂无门禁记录。",
            no_failures: "近期无失败或告警事件。",
            no_artifacts: "暂无登记资产。",
            no_events: "暂无事件。",
            imported_collapsed: "另有 {n} 条导入的历史会话（已从主列表折叠）",
            reproduce_hint: "在本地打开工作台查看详情",
            gate_required: "必需",
            gate_optional: "可选",
        },
        Lang::En => ReportStrings {
            doc_title: "anycode Digital Workbench Report",
            scope_project: "project",
            scope_session: "session",
            label_project: "Project",
            label_session: "Session",
            label_root: "Root",
            label_generated: "Generated",
            section_summary: "Summary",
            section_trust: "Trust & delivery",
            section_sessions: "Recent sessions",
            section_gates: "Acceptance gates",
            section_failures: "Recent failures & warnings",
            section_artifacts: "Artifacts",
            section_reproduce: "Reproduce locally",
            section_prompt: "Prompt preview",
            section_summary_text: "Session summary",
            section_events: "Recent events",
            summary_sessions: "Total sessions",
            summary_events_sampled: "Events sampled in this report (cap)",
            summary_failed_gates: "Failed gates",
            summary_artifacts: "Artifacts",
            summary_trusted: "Session trust",
            no_sessions: "No sessions recorded.",
            no_gates: "No gate records.",
            no_failures: "No recent failures or warnings.",
            no_artifacts: "No artifacts tracked.",
            no_events: "No events.",
            imported_collapsed: "{n} imported historical session(s) collapsed from the main list",
            reproduce_hint: "Open the workbench locally for details",
            gate_required: "required",
            gate_optional: "optional",
        },
    }
}

pub fn label_trust_status(lang: Lang, status: &str) -> String {
    match (lang, status) {
        (Lang::Zh, "verified") => "已验证".into(),
        (Lang::Zh, "unverified") => "未验证".into(),
        (Lang::Zh, "blocked") => "已阻断".into(),
        (Lang::Zh, "pending") => "待确认".into(),
        (Lang::En, s) => s.to_string(),
        (Lang::Zh, s) => s.to_string(),
    }
}

pub fn label_session_kind(lang: Lang, kind: &str) -> String {
    match (lang, kind) {
        (Lang::Zh, "repl") => "REPL".into(),
        (Lang::Zh, "run") => "运行".into(),
        (Lang::Zh, "goal") => "目标".into(),
        (Lang::Zh, "workflow") => "工作流".into(),
        (Lang::Zh, "cron") => "定时".into(),
        (Lang::En, s) => s.to_string(),
        (Lang::Zh, s) => s.to_string(),
    }
}

pub fn label_session_status(lang: Lang, status: &str) -> String {
    match (lang, status) {
        (Lang::Zh, "completed") => "已完成".into(),
        (Lang::Zh, "running") => "运行中".into(),
        (Lang::Zh, "failed") => "失败".into(),
        (Lang::Zh, "cancelled") => "已取消".into(),
        (Lang::En, s) => s.to_string(),
        (Lang::Zh, s) => s.to_string(),
    }
}

pub fn format_verdict(
    lang: Lang,
    trust_verified: i64,
    trust_unverified: i64,
    trust_blocked: i64,
    failed_gates: i64,
) -> String {
    if failed_gates > 0 || trust_blocked > 0 {
        return match lang {
            Lang::Zh => format!("必需门禁失败 {failed_gates}；阻断会话 {trust_blocked}。"),
            Lang::En => format!(
                "Required gate failures: {failed_gates}; blocked sessions: {trust_blocked}."
            ),
        };
    }
    if trust_unverified == 0 {
        return match lang {
            Lang::Zh => {
                format!("已验证会话 {trust_verified}；未验证 0；失败门禁 0。")
            }
            Lang::En => format!(
                "Verified sessions: {trust_verified}; unverified: 0; failed required gates: 0."
            ),
        };
    }
    match lang {
        Lang::Zh => format!("已验证 {trust_verified}；未验证 {trust_unverified}；失败门禁 0。"),
        Lang::En => format!(
            "Verified: {trust_verified}; unverified: {trust_unverified}; failed required gates: 0."
        ),
    }
}
