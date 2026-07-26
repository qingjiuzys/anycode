//! TaskCompiler: utterance → base intent (capabilities/gates) + optional Experience.

use anycode_core::{
    AgentPromptPack, ClarifyingQuestion, ExpectedArtifact, ExperiencePack, GatePlan, GatePolicy,
    Memory, MemoryType, TaskFamily, TaskSpec,
};

/// Independent recall budgets so types are not mashed into one unattributed blob.
#[derive(Debug, Clone)]
pub struct MemoryRecallBudgets {
    pub user: usize,
    pub feedback: usize,
    pub project: usize,
    pub reference: usize,
}

impl Default for MemoryRecallBudgets {
    fn default() -> Self {
        Self {
            user: 6,
            feedback: 4,
            project: 8,
            reference: 4,
        }
    }
}

/// Pure task intent derived without reading Experience.
#[derive(Debug, Clone)]
pub struct BaseTaskIntent {
    pub family: TaskFamily,
    pub goal: String,
    pub constraints: Vec<String>,
    pub expected_artifacts: Vec<ExpectedArtifact>,
    pub required_capabilities: Vec<String>,
    pub deliverables: Vec<String>,
}

/// Full compile output including optional Experience/Skill segments.
#[derive(Debug, Clone, Default)]
pub struct CompiledPromptParts {
    pub task_spec: TaskSpec,
    pub gate_plan: Option<GatePlan>,
    pub preferences_segment: String,
    pub experience_segment: String,
    pub skill_segment: String,
    pub selected_skill_ids: Vec<String>,
    /// Skill IDs denied for this arm (e.g. production skills off) — applied as tool denies.
    pub denied_skill_ids: Vec<String>,
    pub memories_by_type: Vec<(MemoryType, Vec<Memory>)>,
}

/// Eval / runtime arm switches (only two experimental factors).
#[derive(Debug, Clone, Copy, Default)]
pub struct CompileArmFlags {
    pub experience_enabled: bool,
    pub production_skills_enabled: bool,
    /// True under the eval harness (`ANYCODE_EVAL_MODE=1`) — no human in the
    /// loop, so clarifying questions are never injected.
    pub eval_mode: bool,
}

impl CompileArmFlags {
    pub fn production() -> Self {
        Self {
            experience_enabled: true,
            production_skills_enabled: true,
            eval_mode: false,
        }
    }

    pub fn from_eval_env() -> Self {
        if std::env::var("ANYCODE_EVAL_MODE").ok().as_deref() != Some("1") {
            return Self::production();
        }
        let experience_enabled = std::env::var("ANYCODE_EVAL_EXPERIENCE")
            .ok()
            .map(|v| v != "0" && v != "false")
            .unwrap_or(true);
        let production_skills_enabled = std::env::var("ANYCODE_EVAL_SKILLS")
            .ok()
            .map(|v| v != "0" && v != "false")
            .unwrap_or(true);
        Self {
            experience_enabled,
            production_skills_enabled,
            eval_mode: true,
        }
    }
}

pub struct TaskCompiler<'a> {
    pub experience: &'a ExperiencePack,
    pub arm: CompileArmFlags,
}

impl<'a> TaskCompiler<'a> {
    pub fn new(experience: &'a ExperiencePack) -> Self {
        Self {
            experience,
            arm: CompileArmFlags::production(),
        }
    }

    pub fn with_arm(mut self, arm: CompileArmFlags) -> Self {
        self.arm = arm;
        self
    }

    /// ASCII word-boundary match for short tokens (plain `contains("ui")` would
    /// hit "guide", "quick", "equipment", "build"...).
    fn contains_word(haystack: &str, needle: &str) -> bool {
        let bytes = haystack.as_bytes();
        for (i, _) in haystack.match_indices(needle) {
            let start_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let end = i + needle.len();
            let end_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
            if start_ok && end_ok {
                return true;
            }
        }
        false
    }

    pub fn infer_family(prompt: &str) -> TaskFamily {
        let p = prompt.to_ascii_lowercase();
        if [
            "webpage", "landing", "html", "css", "frontend", "网页", "页面", "视觉",
        ]
        .iter()
        .any(|k| p.contains(k))
            || Self::contains_word(&p, "ui")
        {
            return TaskFamily::WebDesign;
        }
        if [
            "sql",
            "select ",
            "create table",
            "database",
            "schema",
            "ddl",
            "postgresql",
            "数据库",
            "查询",
            "表结构",
        ]
        .iter()
        .any(|k| p.contains(k))
        {
            return TaskFamily::DatabaseSql;
        }
        if [
            "ppt",
            "slides",
            "powerpoint",
            "docx",
            "xlsx",
            "xls",
            "spreadsheet",
            "excel",
            "工作簿",
            "表格文件",
            "word文档",
            "办公文档",
            "演示稿",
            "幻灯片",
            "weekly report",
            "postmortem",
        ]
        .iter()
        .any(|k| p.contains(k))
        {
            return TaskFamily::OfficeDelivery;
        }
        if ["refactor", "rename", "cleanup", "重构"]
            .iter()
            .any(|k| p.contains(k))
        {
            return TaskFamily::Refactor;
        }
        if ["research", "investigate", "总结", "调研"]
            .iter()
            .any(|k| p.contains(k))
        {
            return TaskFamily::Research;
        }
        if [
            "rust",
            "cargo",
            "clippy",
            "crate",
            "代码",
            "function",
            "helper",
            "implement",
        ]
        .iter()
        .any(|k| p.contains(k))
        {
            return TaskFamily::CrossFileCoding;
        }
        TaskFamily::General
    }

    /// Capability / artifact derivation — must not read Experience.
    pub fn compile_base_intent(prompt: &str) -> BaseTaskIntent {
        let family = Self::infer_family(prompt);
        let goal = prompt.trim().to_string();
        let mut expected_artifacts = Vec::new();
        let mut required_capabilities = Vec::new();
        let mut deliverables = Vec::new();
        let mut constraints = Vec::new();

        match family {
            TaskFamily::WebDesign => {
                required_capabilities.extend(["web.implement".into(), "web.preview".into()]);
                expected_artifacts.push(ExpectedArtifact {
                    id: "landing_html".into(),
                    kind: "html".into(),
                    required: true,
                    path_globs: vec!["**/*.html".into(), "index.html".into()],
                });
                deliverables.push("self-contained HTML landing page".into());
                constraints.push("no purple/violet AI-slop gradients".into());
            }
            TaskFamily::OfficeDelivery => {
                let p = prompt.to_ascii_lowercase();
                if p.contains("xlsx")
                    || p.contains(".xls")
                    || p.contains("spreadsheet")
                    || p.contains("excel")
                    || p.contains("工作簿")
                    || p.contains("表格")
                    || p.contains("表格文件")
                {
                    required_capabilities.extend([
                        "spreadsheet.author".into(),
                        "spreadsheet.export.xlsx".into(),
                    ]);
                    expected_artifacts.push(ExpectedArtifact {
                        id: "workbook_xlsx".into(),
                        kind: "xlsx".into(),
                        required: true,
                        path_globs: vec!["**/*.xlsx".into()],
                    });
                    deliverables.push("Excel .xlsx workbook (anycode-xlsx)".into());
                } else if p.contains("pptx")
                    || p.contains("ppt")
                    || p.contains("slides")
                    || p.contains("deck")
                    || p.contains("演示")
                    || p.contains("幻灯片")
                {
                    let scenario = infer_office_scenario(prompt);
                    let html_slides = scenario.as_deref() == Some("anycode-ppt")
                        || p.contains("anycode-ppt")
                        || p.contains("anycode ppt")
                        || p.contains("html ppt")
                        || p.contains("html 幻灯片");
                    if html_slides {
                        required_capabilities.push("presentation.author".into());
                        expected_artifacts.push(ExpectedArtifact {
                            id: "deck_html_slides".into(),
                            kind: "html".into(),
                            required: true,
                            path_globs: vec![
                                "slides/*.html".into(),
                                "**/slides/*.html".into(),
                                "index.html".into(),
                            ],
                        });
                        deliverables
                            .push("HTML slide deck (1920×1080 pages + index.html viewer)".into());
                    } else {
                        required_capabilities.extend([
                            "presentation.author".into(),
                            "presentation.export.pptx".into(),
                        ]);
                        expected_artifacts.push(ExpectedArtifact {
                            id: "deck_pptx".into(),
                            kind: "pptx".into(),
                            required: true,
                            path_globs: vec!["**/*.pptx".into()],
                        });
                        deliverables.push("PowerPoint .pptx deck".into());
                    }
                } else {
                    required_capabilities
                        .extend(["document.author".into(), "document.export.docx".into()]);
                    expected_artifacts.push(ExpectedArtifact {
                        id: "report_docx".into(),
                        kind: "docx".into(),
                        required: true,
                        path_globs: vec!["**/*.docx".into()],
                    });
                    expected_artifacts.push(ExpectedArtifact {
                        id: "report_preview".into(),
                        kind: "html".into(),
                        required: false,
                        path_globs: vec!["report.preview.html".into(), "**/*.preview.html".into()],
                    });
                    deliverables.push(
                        "Word .docx + HTML preview (anycode-docx: MD → preview → docx)".into(),
                    );
                }
                let brand = infer_office_brand_kit(prompt);
                constraints.push(format!(
                    "apply brand-kit `{brand}` — no empty default OOXML shells"
                ));
                if infer_office_scenario(prompt).is_some() {
                    constraints.push(
                        "follow scenario pack outline under scenarios/ when templates exist".into(),
                    );
                }
                let p_lower = prompt.to_ascii_lowercase();
                let html_slides = infer_office_scenario(prompt).as_deref() == Some("anycode-ppt")
                    || p_lower.contains("anycode-ppt")
                    || p_lower.contains("anycode ppt")
                    || p_lower.contains("html ppt")
                    || p_lower.contains("html 幻灯片");
                let pptish = p_lower.contains("pptx")
                    || p_lower.contains("ppt")
                    || p_lower.contains("slides")
                    || p_lower.contains("deck")
                    || p_lower.contains("演示")
                    || p_lower.contains("幻灯片");
                if pptish {
                    constraints.push(
                        "PPT: invoke Skill anycode-ppt — COPY templates/*.html (fde-editorial); NEVER lingqi #1B3A5C/#00B050 or invented CSS".into(),
                    );
                    if html_slides {
                        constraints.push(
                            "HTML slides: anycode-ppt run → validate anycode-ppt → deliver slides/*.html + index.html; NO pptx".into(),
                        );
                        constraints.push(
                            "formal artifact is paginated HTML slide files; evidence PNG is preview only".into(),
                        );
                    } else {
                        constraints.push(
                            "PPT export: presentation-design → presentation-commercial-delivery for native OOXML".into(),
                        );
                        constraints.push(
                            "formal artifact must be native editable OOXML; evidence PNG alone is not deliverable".into(),
                        );
                    }
                    constraints
                        .push("render evidence/slide-*.png after HTML design when possible".into());
                }
                let xlsxish = p_lower.contains("xlsx")
                    || p_lower.contains("xls")
                    || p_lower.contains("spreadsheet")
                    || p_lower.contains("excel")
                    || p_lower.contains("工作簿")
                    || p_lower.contains("表格");
                if xlsxish {
                    constraints.push(
                        "XLSX: invoke Skill anycode-xlsx — COPY templates/workbook-*.json → run → .xlsx".into(),
                    );
                    constraints.push(
                        "workbook.json is source; formal artifact must be native .xlsx with fde-editorial theme".into(),
                    );
                }
                let docxish = p_lower.contains("docx")
                    || p_lower.contains("word")
                    || p_lower.contains("document")
                    || p_lower.contains("文档")
                    || p_lower.contains("报告")
                    || p_lower.contains("汇报");
                if docxish {
                    constraints.push(
                        "DOCX: invoke Skill anycode-docx — COPY templates/*.md → validate → preview.html → .docx".into(),
                    );
                    constraints.push(
                        "HTML preview for Workbench review; formal artifact must be editable .docx"
                            .into(),
                    );
                }
            }
            TaskFamily::CrossFileCoding => {
                deliverables.push("code change with tests".into());
            }
            TaskFamily::DatabaseSql => {
                deliverables.push("SQL DDL or query".into());
            }
            _ => {}
        }

        BaseTaskIntent {
            family,
            goal,
            constraints,
            expected_artifacts,
            required_capabilities,
            deliverables,
        }
    }

    pub fn intent_hash(intent: &BaseTaskIntent) -> String {
        let mut parts = vec![intent.family.as_str().to_string(), intent.goal.clone()];
        parts.extend(intent.required_capabilities.iter().cloned());
        for a in &intent.expected_artifacts {
            parts.push(format!("{}:{}", a.id, a.kind));
        }
        // Stable non-cryptographic fingerprint for gate plan identity.
        format!("{:x}", simple_hash(&parts.join("|")))
    }

    pub fn compile(
        &self,
        prompt: &str,
        recalled: &[(MemoryType, Vec<Memory>)],
    ) -> CompiledPromptParts {
        let intent = Self::compile_base_intent(prompt);
        let hash = Self::intent_hash(&intent);
        let brand_kit = infer_office_brand_kit(prompt);
        let scenario = infer_office_scenario(prompt);
        let gate_extras = office_gate_extras(&brand_kit, scenario.as_deref());
        let gate_plan = if !intent.expected_artifacts.is_empty() {
            Some(GatePolicy::plan(
                Some(intent.family),
                &intent.expected_artifacts,
                &hash,
                Some(&gate_extras),
            ))
        } else {
            None
        };

        let cards = if self.arm.experience_enabled {
            self.experience.retrieve(prompt, 3)
        } else {
            Vec::new()
        };

        let mut preference_hits = Vec::new();
        let mut missing = Vec::new();
        let mut clarifying = Vec::new();
        let eval_mode = self.arm.eval_mode;

        let user_mems: Vec<&Memory> = recalled
            .iter()
            .filter(|(t, _)| *t == MemoryType::User)
            .flat_map(|(_, v)| v.iter())
            .collect();
        for m in &user_mems {
            preference_hits.push(format!("{}: {}", m.title, truncate(&m.content, 160)));
        }

        // The prompt itself counts as an answer — never ask about what the
        // user already stated.
        let prompt_lower = intent.goal.to_ascii_lowercase();

        // Eval harness has no human in the loop — never inject clarifying questions.
        if !eval_mode && intent.family == TaskFamily::WebDesign {
            let joined = format!("{}\n{}", preference_hits.join("\n"), prompt_lower);
            if !joined.contains("color")
                && !joined.contains("theme")
                && !joined.contains("色")
                && !joined.contains("主题")
                && !joined.contains('#')
            // explicit hex color in prompt
            {
                missing.push("visual_theme".into());
                clarifying.push(ClarifyingQuestion {
                    id: "visual_theme".into(),
                    prompt: "Preferred visual theme / colors / density? (ask at most once)".into(),
                    options: vec![
                        "FDE editorial（浅底 #f2f5f0 + 电蓝 #1400ff，默认）".into(),
                        "dark tech + emerald".into(),
                        "reuse last preference".into(),
                    ],
                });
            }
        }

        if !eval_mode && intent.family == TaskFamily::OfficeDelivery {
            let joined = format!("{}\n{}", preference_hits.join("\n"), prompt_lower);
            if !joined.contains("audience") && !joined.contains("受众") && !joined.contains("读者")
            {
                missing.push("audience".into());
                clarifying.push(ClarifyingQuestion {
                    id: "audience".into(),
                    prompt: "Who is the primary audience for this document/deck?".into(),
                    options: vec![
                        "executives".into(),
                        "engineering".into(),
                        "customers".into(),
                    ],
                });
            }
        }

        if !eval_mode && intent.family == TaskFamily::DatabaseSql {
            let joined = format!("{}\n{}", preference_hits.join("\n"), prompt_lower);
            if !joined.contains("postgres")
                && !joined.contains("mysql")
                && !joined.contains("sqlite")
                && !joined.contains("数据库")
            {
                missing.push("sql_dialect".into());
                clarifying.push(ClarifyingQuestion {
                    id: "sql_dialect".into(),
                    prompt: "Which SQL dialect should we target?".into(),
                    options: vec!["PostgreSQL".into(), "MySQL".into(), "SQLite".into()],
                });
            }
        }

        // Lead pack only — do NOT attach visual_verifier completion authority.
        let agent_packs = vec![AgentPromptPack {
            agent_id: "general-purpose".into(),
            role: "lead".into(),
            objective: intent.goal.clone(),
            constraints: {
                let mut c = vec![
                    "Respect recalled preferences; do not re-ask known stable prefs.".into(),
                    "Keep tool use minimal and verify deliverables.".into(),
                ];
                c.extend(intent.constraints.clone());
                c
            },
            allowed_tools: Vec::new(),
            done_when: vec!["independent verification gates pass".into()],
            experience_examples: cards.iter().map(|c| c.to_prompt_excerpt(1)).collect(),
            recovery_hints: cards.iter().flat_map(|c| c.recovery.clone()).collect(),
        }];

        let mut acceptance = Vec::new();
        // Soft hints only — Experience must not rewrite required gates.
        if self.arm.experience_enabled {
            if let Some(card) = cards.first() {
                acceptance.extend(card.key_checks.iter().cloned().take(4));
            }
        }

        let mut extras = std::collections::HashMap::new();
        extras.insert("intent_hash".into(), hash.clone());
        extras.insert(
            "experience_enabled".into(),
            self.arm.experience_enabled.to_string(),
        );
        extras.insert(
            "production_skills_enabled".into(),
            self.arm.production_skills_enabled.to_string(),
        );
        if intent.family == TaskFamily::OfficeDelivery {
            extras.insert("brand_kit".into(), brand_kit);
            if let Some(sc) = scenario {
                extras.insert("scenario".into(), sc);
            }
        }

        let spec = TaskSpec {
            goal: intent.goal.clone(),
            family: Some(intent.family),
            constraints: intent.constraints.clone(),
            deliverables: intent.deliverables.clone(),
            required_capabilities: intent.required_capabilities.clone(),
            expected_artifacts: intent.expected_artifacts.clone(),
            acceptance,
            risks: Vec::new(),
            missing_preferences: missing,
            clarifying_questions: clarifying.into_iter().take(2).collect(),
            preference_hits: preference_hits.clone(),
            experience_card_ids: cards.iter().map(|c| c.id.clone()).collect(),
            agent_packs,
            workflow: None,
            extras,
        };

        let preferences_segment = if preference_hits.is_empty() {
            String::new()
        } else {
            let mut lines = vec!["## Preferences".to_string()];
            for (mt, mems) in recalled {
                if *mt != MemoryType::User && *mt != MemoryType::Feedback {
                    continue;
                }
                let label = mt.as_storage_str();
                for m in mems {
                    lines.push(format!("### [{label}] {}", m.title));
                    lines.push(m.content.clone());
                    if let Some(meta) = &m.meta {
                        lines.push(format!(
                            "<!-- evidence:{} kind:{} -->",
                            meta.evidence_hash,
                            meta.kind.as_str()
                        ));
                    }
                    lines.push(String::new());
                }
            }
            lines.join("\n")
        };

        let experience_segment = if self.arm.experience_enabled {
            self.experience.to_prompt_segment(prompt, 3)
        } else {
            String::new()
        };

        CompiledPromptParts {
            task_spec: spec,
            gate_plan,
            preferences_segment,
            experience_segment,
            skill_segment: String::new(),
            selected_skill_ids: Vec::new(),
            denied_skill_ids: Vec::new(),
            memories_by_type: recalled.to_vec(),
        }
    }
}

fn infer_office_brand_kit(prompt: &str) -> String {
    let p = prompt.to_ascii_lowercase();
    let gov = [
        "政府",
        "政务",
        "公文",
        "红头",
        "gov ",
        "government",
        "密级",
        "机关",
    ];
    let edu = [
        "教育",
        "教学",
        "课纲",
        "教案",
        "lesson plan",
        "school",
        "course",
        "课件",
    ];
    if gov.iter().any(|k| p.contains(k)) {
        return "gov-formal".into();
    }
    if edu.iter().any(|k| p.contains(k)) {
        return "edu-clean".into();
    }
    "fde-editorial".into()
}

fn infer_office_scenario(prompt: &str) -> Option<String> {
    let p = prompt.to_ascii_lowercase();
    let rules: &[(&str, &[&str])] = &[
        (
            "anycode-ppt",
            &[
                "anycode-ppt",
                "anycode ppt",
                "html ppt",
                "slides",
                "deck",
                "幻灯片",
                "演示文稿",
                "pitch deck",
                "presentation",
                "调研",
                "高密度 ppt",
                "dense deck",
                "editorial ppt",
            ],
        ),
        (
            "anycode-xlsx",
            &[
                "anycode-xlsx",
                "anycode xlsx",
                "xlsx",
                "excel",
                "spreadsheet",
                "工作簿",
                "表格",
            ],
        ),
        // Specific scenario packs BEFORE the generic anycode-docx family —
        // otherwise 述职/工作汇报/周报 would be shadowed by the docx default.
        (
            "performance-review",
            &["述职", "performance review", "okr review", "年度考核"],
        ),
        (
            "education-lesson-plan",
            &["课纲", "教案", "lesson plan", "教学设计", "教学课件"],
        ),
        (
            "gov-briefing",
            &["政府", "政务", "gov briefing", "公文", "汇报材料"],
        ),
        (
            "finance-quarterly-review",
            &[
                "财务",
                "预算",
                "quarterly review",
                "损益",
                "p&l",
                "cashflow",
            ],
        ),
        (
            "med-aesthetic-proposal",
            &["医美", "med aesthetic", "美容", "诊所方案"],
        ),
        (
            "product-launch",
            &["产品发布", "product launch", "launch deck"],
        ),
        (
            "work-report",
            &["工作汇报", "weekly report", "周报", "月报", "ops report"],
        ),
        (
            "anycode-docx",
            &[
                "anycode-docx",
                "anycode docx",
                "docx",
                "word",
                "文档",
                "报告",
                "briefing",
            ],
        ),
    ];
    for (id, keys) in rules {
        if keys.iter().any(|k| p.contains(k)) {
            return Some((*id).into());
        }
    }
    None
}

fn office_gate_extras(
    brand_kit: &str,
    scenario: Option<&str>,
) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    m.insert("brand_kit".into(), brand_kit.into());
    if let Some(s) = scenario {
        m.insert("scenario".into(), s.into());
    }
    m
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Format multi-type memories as separate attributed sections (not one blob).
pub fn attributed_memories_sections(recalled: &[(MemoryType, Vec<Memory>)]) -> Vec<String> {
    let mut out = Vec::new();
    for (mt, mems) in recalled {
        if mems.is_empty() {
            continue;
        }
        let mut lines = vec![
            format!("## Memories ({})", mt.as_storage_str()),
            String::new(),
        ];
        for m in mems {
            lines.push(format!("### {}", m.title));
            lines.push(m.content.clone());
            lines.push(String::new());
        }
        out.push(lines.join("\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::{builtin_web_and_rust_pack, MemoryScope};

    fn mem(title: &str, content: &str, mt: MemoryType) -> Memory {
        let now = chrono::Utc::now();
        Memory {
            id: title.into(),
            mem_type: mt,
            title: title.into(),
            content: content.into(),
            tags: vec![],
            scope: MemoryScope::Private,
            created_at: now,
            updated_at: now,
            meta: None,
        }
    }

    #[test]
    fn base_intent_does_not_need_experience() {
        let intent = TaskCompiler::compile_base_intent("Build a landing webpage");
        assert_eq!(intent.family, TaskFamily::WebDesign);
        assert!(intent
            .required_capabilities
            .contains(&"web.implement".into()));
        assert!(!intent.expected_artifacts.is_empty());
    }

    #[test]
    fn report_results_phrase_is_not_office_delivery() {
        assert_ne!(
            TaskCompiler::infer_family("请读取 Cargo.toml 并报告结果"),
            TaskFamily::OfficeDelivery
        );
        assert_eq!(
            TaskCompiler::infer_family("导出 weekly report 为 .docx"),
            TaskFamily::OfficeDelivery
        );
    }

    #[test]
    fn experience_disabled_keeps_capabilities() {
        let pack = builtin_web_and_rust_pack();
        let compiler = TaskCompiler::new(&pack).with_arm(CompileArmFlags {
            experience_enabled: false,
            production_skills_enabled: false,
            eval_mode: true,
        });
        let parts = compiler.compile("Build a landing webpage", &[]);
        assert!(parts.experience_segment.is_empty());
        assert_eq!(
            parts.task_spec.required_capabilities,
            TaskCompiler::compile_base_intent("Build a landing webpage").required_capabilities
        );
        assert!(parts.gate_plan.as_ref().is_some_and(|g| !g.is_empty()));
        assert!(parts.task_spec.workflow.is_none());
        assert!(!parts
            .task_spec
            .agent_packs
            .iter()
            .any(|p| p.role == "visual_verifier"));
    }

    #[test]
    fn compiles_web_task_with_prefs_and_experience() {
        let pack = builtin_web_and_rust_pack();
        let compiler = TaskCompiler::new(&pack);
        let recalled = vec![(
            MemoryType::User,
            vec![mem(
                "theme",
                "prefer dark tech theme with emerald accent",
                MemoryType::User,
            )],
        )];
        let parts = compiler.compile("Build a landing webpage", &recalled);
        assert_eq!(parts.task_spec.family, Some(TaskFamily::WebDesign));
        assert!(parts.task_spec.clarifying_questions.is_empty());
        assert!(!parts.experience_segment.is_empty());
        assert!(parts.preferences_segment.contains("dark tech"));
        assert!(parts.experience_segment.contains("avoid:"));
    }

    #[test]
    fn infers_office_and_sql_families() {
        let pack = builtin_web_and_rust_pack();
        let compiler = TaskCompiler::new(&pack);
        let ppt = compiler.compile("写一份 pptx 产品 briefing", &[]);
        assert_eq!(ppt.task_spec.family, Some(TaskFamily::OfficeDelivery));
        assert!(ppt
            .task_spec
            .required_capabilities
            .iter()
            .any(|c| c.starts_with("presentation.")));
        let xlsx = compiler.compile("导出 June sales 为 .xlsx 工作簿", &[]);
        assert_eq!(xlsx.task_spec.family, Some(TaskFamily::OfficeDelivery));
        assert!(xlsx
            .task_spec
            .required_capabilities
            .iter()
            .any(|c| c.starts_with("spreadsheet.")));
        assert!(xlsx
            .task_spec
            .expected_artifacts
            .iter()
            .any(|a| a.kind == "xlsx"));
        let sql = compiler.compile(
            "Write PostgreSQL SELECT with JOIN for unpaid invoices",
            &[(
                MemoryType::User,
                vec![mem("dialect", "prefer PostgreSQL", MemoryType::User)],
            )],
        );
        assert_eq!(sql.task_spec.family, Some(TaskFamily::DatabaseSql));
        assert!(sql.task_spec.clarifying_questions.is_empty());
    }

    #[test]
    fn infers_anycode_html_slides_over_pptx() {
        let pack = builtin_web_and_rust_pack();
        let compiler = TaskCompiler::new(&pack);
        let html = compiler.compile("用 anycode ppt 做产品发布幻灯片", &[]);
        assert!(html
            .task_spec
            .expected_artifacts
            .iter()
            .any(|a| a.kind == "html" && a.id == "deck_html_slides"));
        assert!(!html
            .task_spec
            .required_capabilities
            .iter()
            .any(|c| c == "presentation.export.pptx"));
        assert!(html
            .task_spec
            .constraints
            .iter()
            .any(|c| c.contains("NO pptx")));
    }

    #[test]
    fn infers_gov_and_edu_brand_kits() {
        assert_eq!(infer_office_brand_kit("政府公文汇报材料"), "gov-formal");
        assert_eq!(infer_office_brand_kit("高中物理课纲教案"), "edu-clean");
        assert_eq!(
            infer_office_brand_kit("enterprise pitch deck"),
            "fde-editorial"
        );
        assert_eq!(
            infer_office_scenario("anycode ppt 产品发布幻灯片"),
            Some("anycode-ppt".into())
        );
    }

    #[test]
    fn infers_office_scenarios() {
        assert_eq!(
            infer_office_scenario("年度述职 OKR review"),
            Some("performance-review".into())
        );
        assert_eq!(
            infer_office_scenario("教学设计 lesson plan"),
            Some("education-lesson-plan".into())
        );
        assert_eq!(
            infer_office_scenario("generic docx report"),
            Some("anycode-docx".into())
        );
    }

    #[test]
    fn office_compile_sets_brand_and_scenario_extras() {
        let pack = builtin_web_and_rust_pack();
        let compiler = TaskCompiler::new(&pack);
        let parts = compiler.compile("政府政务公文 docx 汇报材料", &[]);
        assert_eq!(
            parts.task_spec.extras.get("brand_kit").map(String::as_str),
            Some("gov-formal")
        );
        assert_eq!(
            parts.task_spec.extras.get("scenario").map(String::as_str),
            Some("gov-briefing")
        );
        let plan = parts.gate_plan.expect("gate plan");
        assert!(plan
            .requirements
            .iter()
            .any(|r| r.validator_id == "office.docx_classification"));
    }
}
