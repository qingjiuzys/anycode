//! Validated, signed experience packs distilled offline from teacher trajectories.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::task_spec::TaskFamily;

/// Retrievable procedural experience card (not raw teacher chain-of-thought).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperienceCard {
    pub id: String,
    pub title: String,
    pub family: TaskFamily,
    #[serde(default)]
    pub applicable_when: Vec<String>,
    #[serde(default)]
    pub task_breakdown: Vec<String>,
    #[serde(default)]
    pub tool_order: Vec<String>,
    #[serde(default)]
    pub key_checks: Vec<String>,
    #[serde(default)]
    pub common_failures: Vec<String>,
    #[serde(default)]
    pub recovery: Vec<String>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub model_compat: Vec<String>,
    #[serde(default)]
    pub regression_score: f64,
    #[serde(default)]
    pub version: String,
}

impl ExperienceCard {
    pub fn to_prompt_excerpt(&self, max_examples: usize) -> String {
        let mut lines = vec![
            format!("### Experience: {}", self.title),
            format!("id: {}", self.id),
            format!("family: {}", self.family.as_str()),
        ];
        if !self.applicable_when.is_empty() {
            lines.push(format!("when: {}", self.applicable_when.join("; ")));
        }
        if !self.task_breakdown.is_empty() {
            lines.push(format!("breakdown: {}", self.task_breakdown.join(" → ")));
        }
        if !self.tool_order.is_empty() {
            lines.push(format!("tools: {}", self.tool_order.join(" → ")));
        }
        if !self.key_checks.is_empty() {
            lines.push(format!("checks: {}", self.key_checks.join("; ")));
        }
        if !self.common_failures.is_empty() {
            lines.push(format!("avoid: {}", self.common_failures.join("; ")));
        }
        if !self.recovery.is_empty() {
            lines.push(format!("recovery: {}", self.recovery.join("; ")));
        }
        for ex in self.examples.iter().take(max_examples) {
            lines.push(format!("example: {ex}"));
        }
        lines.join("\n")
    }

    /// Simple keyword score for retrieval (no embedding required).
    pub fn score_query(&self, query: &str) -> f64 {
        let q = query.to_ascii_lowercase();
        let mut score = 0.0;
        if q.contains(self.family.as_str()) {
            score += 2.0;
        }
        for token in self
            .applicable_when
            .iter()
            .chain(self.task_breakdown.iter())
            .chain(std::iter::once(&self.title))
        {
            let t = token.to_ascii_lowercase();
            if !t.is_empty() && q.contains(t.trim()) {
                score += 1.0;
            }
            for word in t.split_whitespace() {
                if word.len() > 3 && q.contains(word) {
                    score += 0.25;
                }
            }
        }
        score + self.regression_score.max(0.0) * 0.1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperiencePackMeta {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub model_compat: Vec<String>,
    #[serde(default)]
    pub regression_score: f64,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// Detached signature over canonical card payload (hex). Empty = unsigned local draft.
    #[serde(default)]
    pub signature_hex: String,
    #[serde(default)]
    pub signer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperiencePack {
    pub meta: ExperiencePackMeta,
    pub cards: Vec<ExperienceCard>,
}

impl ExperiencePack {
    pub fn retrieve(&self, query: &str, limit: usize) -> Vec<&ExperienceCard> {
        let mut scored: Vec<_> = self
            .cards
            .iter()
            .map(|c| (c.score_query(query), c))
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(limit).map(|(_, c)| c).collect()
    }

    pub fn to_prompt_segment(&self, query: &str, limit: usize) -> String {
        let hits = self.retrieve(query, limit);
        if hits.is_empty() {
            return String::new();
        }
        let mut lines = vec![
            "## Experience Pack".to_string(),
            format!("pack: {}@{}", self.meta.id, self.meta.version),
        ];
        for card in hits {
            lines.push(card.to_prompt_excerpt(1));
            lines.push(String::new());
        }
        lines.join("\n")
    }

    /// Canonical bytes used for signing / verification (stable field order via serde_json).
    pub fn signing_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        let body = serde_json::json!({
            "id": self.meta.id,
            "version": self.meta.version,
            "cards": self.cards,
        });
        serde_json::to_vec(&body)
    }
}

fn weak_compat() -> Vec<String> {
    vec!["weak_local".into(), "deepseek-v4-flash".into(), "*".into()]
}

/// Ship a built-in pack covering web / visual / office / code / database scenarios.
///
/// 0.3.0 focuses on common product paths (landing visuals, docs, decks) with
/// output contracts learned from v3 ablation.
pub fn builtin_web_and_rust_pack() -> ExperiencePack {
    ExperiencePack {
        meta: ExperiencePackMeta {
            id: "anycode-builtin".into(),
            version: "0.3.0".into(),
            model_compat: weak_compat(),
            regression_score: 0.86,
            created_at: None,
            signature_hex: String::new(),
            signer: "anycode-builtin".into(),
        },
        cards: vec![
            ExperienceCard {
                id: "web.design-implement-verify".into(),
                title: "Landing page: rich visual craft + verify".into(),
                family: TaskFamily::WebDesign,
                applicable_when: vec![
                    "webpage".into(),
                    "landing".into(),
                    "html".into(),
                    "css".into(),
                    "UI".into(),
                    "frontend".into(),
                    "visual".into(),
                    "网页".into(),
                    "页面".into(),
                    "视觉".into(),
                ],
                task_breakdown: vec![
                    "reuse known visual preferences".into(),
                    "pick distinctive type + dark tokens + emerald CTA".into(),
                    "compose rich hero with lede, primary CTA, secondary link".into(),
                    "add one visual anchor (terminal mock / product panel)".into(),
                    "write semantic HTML/CSS with contrast comments".into(),
                    "verify via Skill or browser: no purple, single H1, readable contrast".into(),
                ],
                tool_order: vec![
                    "SkillSearch".into(),
                    "Skill".into(),
                    "Write".into(),
                    "BrowserNavigate".into(),
                    "BrowserScreenshot".into(),
                    "Bash".into(),
                ],
                key_checks: vec![
                    "output pure HTML only — no markdown fences".into(),
                    "background ~#0B0F14; CTA #10B981; body text light on dark".into(),
                    "exactly one H1; primary CTA + secondary text link".into(),
                    "HTML comments state approximate contrast for body text and CTA".into(),
                    "keep visual hierarchy — do not flatten to a bare centered block".into(),
                    "prefer distinctive fonts over Inter/Roboto; no purple/violet".into(),
                ],
                common_failures: vec![
                    "checklist flattening that deletes brand/visual anchor".into(),
                    "purple-indigo gradient AI slop".into(),
                    "wrapping HTML in markdown fences".into(),
                    "external font CDN when self-contained was required".into(),
                    "re-asking colors already in preference memory".into(),
                ],
                recovery: vec![
                    "restore secondary visual panel, then re-check contrast comments".into(),
                    "strip fences; keep single HTML document".into(),
                ],
                examples: vec![
                    "Dark tech landing: serif/mono pairing, emerald CTA, terminal aside, contrast comments.".into(),
                ],
                model_compat: weak_compat(),
                regression_score: 0.9,
                version: "0.3.0".into(),
            },
            ExperienceCard {
                id: "code.cross-file-verify".into(),
                title: "Code change → assert consistency → test → fix".into(),
                family: TaskFamily::CrossFileCoding,
                applicable_when: vec![
                    "rust".into(),
                    "cargo".into(),
                    "clippy".into(),
                    "code".into(),
                    "slugify".into(),
                    "代码".into(),
                    "function".into(),
                ],
                task_breakdown: vec![
                    "locate symbols".into(),
                    "edit implementation".into(),
                    "write tests whose expected values match the stated rules".into(),
                    "run focused tests".into(),
                    "fix failures before claiming done".into(),
                ],
                tool_order: vec!["Grep".into(), "Read".into(), "Edit".into(), "Bash".into()],
                key_checks: vec![
                    "every assert expected value is reachable under the stated transform rules".into(),
                    "deleted chars must not invent separators".into(),
                    "output Rust only — no markdown fences".into(),
                ],
                common_failures: vec![
                    "tests that contradict the implementation rules".into(),
                    "edit without running tests".into(),
                    "markdown fences around code".into(),
                ],
                recovery: vec!["failing test → fix assert or impl, re-run until green".into()],
                examples: vec![
                    "slugify('--_junk_123--') → 'junk123' (underscores dropped, no extra dash).".into(),
                ],
                model_compat: weak_compat(),
                regression_score: 0.91,
                version: "0.3.0".into(),
            },
            ExperienceCard {
                id: "office.pptx-briefing".into(),
                title: "Executive deck: valid schema + narrative".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "pptx".into(),
                    "ppt".into(),
                    "slides".into(),
                    "deck".into(),
                    "briefing".into(),
                    "演示".into(),
                    "幻灯片".into(),
                ],
                task_breakdown: vec![
                    "presentation-design: dense HTML slides + slide_manifest.json + evidence PNGs".into(),
                    "presentation-commercial-delivery: fill_potx → editable native .pptx (not raster)".into(),
                    "cover: 3 chips; content: 5 bullets + sidebar; metrics: 6 cards; closing: 4 actions".into(),
                    "outline problem → metric → plan → risks → ask with numbers or owner/date".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "office.pptx_editable gate: deck has real a:t text, not full-slide images".into(),
                    "no sparse slides — two-column / metrics grid required".into(),
                    "evidence/slide-*.png from HTML for blind review only".into(),
                    "non-title slides: every bullet has a number or owner/date + source".into(),
                ],
                common_failures: vec![
                    "duplicate JSON keys".into(),
                    "walls of text; generic competitor names".into(),
                    "markdown fences around JSON".into(),
                ],
                recovery: vec![
                    "re-serialize from an in-memory structure so keys cannot duplicate".into(),
                ],
                examples: vec![
                    "Title, Problem, Metric, Plan, Risks, Ask — each Plan bullet has owner + ISO date.".into(),
                ],
                model_compat: weak_compat(),
                regression_score: 0.92,
                version: "0.4.0".into(),
            },
            ExperienceCard {
                id: "office.docx-report".into(),
                title: "Ops/doc report: hierarchy + Decision/Action".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "docx".into(),
                    "word".into(),
                    "report".into(),
                    "document".into(),
                    "文档".into(),
                    "报告".into(),
                    "周报".into(),
                ],
                task_breakdown: vec![
                    "Summary as level-1 heading first".into(),
                    "H2 sections for metrics/incidents/changes/next steps".into(),
                    "end every section with Decision: or Action: plus owner/date".into(),
                    "export via document-delivery Skill with fde-editorial brand header/footer".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "first section Summary at level 1; body sections level 2+".into(),
                    "page header + footer present (commercial docx gate)".into(),
                    "every section ends with Decision: or Action:".into(),
                    "Action/Decision lines include a person name or ISO date when possible".into(),
                    "no empty paragraphs; no markdown fences".into(),
                ],
                common_failures: vec![
                    "flat bullet dump without hierarchy".into(),
                    "Decision/Action missing or vague without owner/date".into(),
                    "markdown fences around JSON".into(),
                ],
                recovery: vec![
                    "rebuild outline first; append Decision/Action as last paragraph of each section".into(),
                ],
                examples: vec![
                    "Action: Priya Nair will publish variance analysis by 2026-07-21.".into(),
                ],
                model_compat: weak_compat(),
                regression_score: 0.92,
                version: "0.4.0".into(),
            },
            ExperienceCard {
                id: "office.xlsx-workbook".into(),
                title: "Spreadsheet: header + concrete numeric rows".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "xlsx".into(),
                    "xls".into(),
                    "spreadsheet".into(),
                    "excel".into(),
                    "workbook".into(),
                    "工作簿".into(),
                    "表格文件".into(),
                ],
                task_breakdown: vec![
                    "confirm sheet name, columns, and units".into(),
                    "write CSV or Markdown table with header + real rows".into(),
                    "export via spreadsheet-delivery Skill: ≥3 sheets (Summary + Detail + Pricing), branded header fill".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "≥3 worksheets; Summary + Detail + Pricing with brand header fill".into(),
                    "header row present; ≥6 data rows".into(),
                    "no TBD / lorem / placeholder cells".into(),
                    "real .xlsx OOXML — not CSV-only delivery".into(),
                ],
                common_failures: vec![
                    "CSV left as final artifact without .xlsx export".into(),
                    "empty sheet or header-only workbook".into(),
                    "placeholder cells".into(),
                ],
                recovery: vec![
                    "rebuild table source then re-run spreadsheet Skill export".into(),
                ],
                examples: vec![
                    "Region,Product,Units,Revenue — APAC,Pro,120,48000".into(),
                ],
                model_compat: weak_compat(),
                regression_score: 0.92,
                version: "0.4.0".into(),
            },
            ExperienceCard {
                id: "office.pptx-product-launch".into(),
                title: "Product launch deck: narrative + metrics".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "product launch".into(),
                    "产品发布".into(),
                    "launch deck".into(),
                    "go-to-market".into(),
                ],
                task_breakdown: vec![
                    "scenario product-launch: problem → solution → traction → roadmap → ask".into(),
                    "presentation-design + presentation-commercial-delivery with brand_kit from prompt".into(),
                    "metrics slide may include data-chart JSON → native or PNG chart".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "office.pptx_editable pass; ≥5 shapes/slide".into(),
                    "every non-title slide has number or owner/date".into(),
                ],
                common_failures: vec!["sparse launch deck".into(), "missing ask slide".into()],
                recovery: vec!["copy scenarios/product-launch outline before authoring HTML".into()],
                examples: vec!["Cover + Problem + Metric + Plan + Risks + Ask + Closing contact".into()],
                model_compat: weak_compat(),
                regression_score: 0.9,
                version: "0.5.0".into(),
            },
            ExperienceCard {
                id: "office.docx-work-report".into(),
                title: "Work report: weekly/monthly hierarchy".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "工作汇报".into(),
                    "weekly report".into(),
                    "月报".into(),
                    "work report".into(),
                ],
                task_breakdown: vec![
                    "scenario work-report: Summary + Progress + Metrics + Issues + Next Steps".into(),
                    "document-delivery with inferred brand_kit (fde-editorial default; gov-formal/edu-clean for their domains)".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "office.docx_commercial pass".into(),
                    "Decision/Action with owner/date per section".into(),
                ],
                common_failures: vec!["flat bullets without hierarchy".into()],
                recovery: vec!["rebuild H2 sections from work-report scenario manifest".into()],
                examples: vec!["Action: Lin Wei will close incident #442 by 2026-07-25.".into()],
                model_compat: weak_compat(),
                regression_score: 0.91,
                version: "0.5.0".into(),
            },
            ExperienceCard {
                id: "office.docx-performance-review".into(),
                title: "Performance review (述职): OKR recap".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "述职".into(),
                    "performance review".into(),
                    "okr review".into(),
                    "年度考核".into(),
                ],
                task_breakdown: vec![
                    "scenario performance-review: achievements → gaps → plan → support".into(),
                    "export docx with Heading chain + Decision/Action".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "sections for Highlights, Gaps, Next Quarter Plan".into(),
                    "concrete metrics — no TBD".into(),
                ],
                common_failures: vec!["vague accomplishments".into()],
                recovery: vec!["add numbered OKR outcomes before narrative".into()],
                examples: vec!["Decision: promote API reliability program to P0 for Q3.".into()],
                model_compat: weak_compat(),
                regression_score: 0.9,
                version: "0.5.0".into(),
            },
            ExperienceCard {
                id: "office.pptx-education-lesson".into(),
                title: "Education lesson deck".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "课纲".into(),
                    "教案".into(),
                    "lesson plan".into(),
                    "教学设计".into(),
                    "教学课件".into(),
                ],
                task_breakdown: vec![
                    "brand_kit edu-clean; scenario education-lesson-plan outline".into(),
                    "objectives → activities → assessment → homework slides".into(),
                    "presentation-design + commercial export".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "office.pptx_editable pass".into(),
                    "learning objectives measurable on content slides".into(),
                ],
                common_failures: vec!["slides too sparse for classroom use".into()],
                recovery: vec!["add assessment rubric bullets + activity timeline".into()],
                examples: vec!["Objective: students can explain 3 core concepts with examples.".into()],
                model_compat: weak_compat(),
                regression_score: 0.89,
                version: "0.5.0".into(),
            },
            ExperienceCard {
                id: "office.docx-gov-briefing".into(),
                title: "Government briefing: classification + structure".into(),
                family: TaskFamily::OfficeDelivery,
                applicable_when: vec![
                    "政府".into(),
                    "政务".into(),
                    "gov briefing".into(),
                    "公文".into(),
                    "汇报材料".into(),
                ],
                task_breakdown: vec![
                    "brand_kit gov-formal; scenario gov-briefing".into(),
                    "include 密级/classification line in header or opening paragraph".into(),
                    "document-delivery export with Heading styles".into(),
                ],
                tool_order: vec!["SkillSearch".into(), "Skill".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "office.docx_classification pass when gov scenario".into(),
                    "formal section order: background → analysis → recommendations".into(),
                ],
                common_failures: vec!["missing classification label".into()],
                recovery: vec!["add 密级：内部 line before Summary heading".into()],
                examples: vec!["密级：内部 · Summary · Background · Recommendations".into()],
                model_compat: weak_compat(),
                regression_score: 0.9,
                version: "0.5.0".into(),
            },
            ExperienceCard {
                id: "db.schema-first".into(),
                title: "Database schema with constraints + cents".into(),
                family: TaskFamily::DatabaseSql,
                applicable_when: vec![
                    "database".into(),
                    "schema".into(),
                    "ddl".into(),
                    "table".into(),
                    "数据库".into(),
                    "表结构".into(),
                ],
                task_breakdown: vec![
                    "list entities + relationships".into(),
                    "UUID PKs + FKs with ON DELETE".into(),
                    "money as integer cents + NOT NULL status".into(),
                    "COMMENT ON TABLE; useful indexes".into(),
                ],
                tool_order: vec!["Write".into(), "Bash".into()],
                key_checks: vec![
                    "every table PRIMARY KEY; FKs state ON DELETE".into(),
                    "amounts in integer cents; status NOT NULL".into(),
                    "COMMENT ON TABLE for each table".into(),
                    "SQL only — no markdown fences".into(),
                ],
                common_failures: vec![
                    "money as float".into(),
                    "missing ON DELETE".into(),
                    "markdown fences".into(),
                ],
                recovery: vec!["regenerate DDL from entity list with constraints checklist".into()],
                examples: vec!["SaaS billing: orgs → users → plans → subscriptions → invoices".into()],
                model_compat: weak_compat(),
                regression_score: 0.88,
                version: "0.3.0".into(),
            },
            ExperienceCard {
                id: "sql.query-safe".into(),
                title: "SQL query: correct window + clean output".into(),
                family: TaskFamily::DatabaseSql,
                applicable_when: vec![
                    "sql".into(),
                    "select".into(),
                    "query".into(),
                    "join".into(),
                    "查询".into(),
                ],
                task_breakdown: vec![
                    "restate filters in SQL terms".into(),
                    "explicit columns; JOIN with predicates".into(),
                    "date windows inclusive/correct".into(),
                    "emit bare SQL only".into(),
                ],
                tool_order: vec!["Read".into(), "Write".into(), "Bash".into()],
                key_checks: vec![
                    "no SELECT *".into(),
                    "JOIN conditions present; qualify columns".into(),
                    "bare SQL only — no markdown fences".into(),
                ],
                common_failures: vec![
                    "markdown fences around simple SQL".into(),
                    "cartesian joins".into(),
                    "wrong date window off-by-one".into(),
                ],
                recovery: vec!["strip fences; qualify columns with aliases".into()],
                examples: vec![
                    "Activated users within 7 days of signup: JOIN events, COUNT DISTINCT, LIMIT 20".into(),
                ],
                model_compat: weak_compat(),
                regression_score: 0.9,
                version: "0.3.0".into(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieve_web_card() {
        let pack = builtin_web_and_rust_pack();
        let hits = pack.retrieve("build a dark landing webpage UI", 2);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "web.design-implement-verify");
        let seg = pack.to_prompt_segment("webpage", 1);
        assert!(seg.contains("Experience Pack"));
    }

    #[test]
    fn retrieve_office_and_sql_cards() {
        let pack = builtin_web_and_rust_pack();
        assert_eq!(pack.cards.len(), 12);
        assert_eq!(
            pack.retrieve("写一份 pptx 演示文稿 briefing", 1)[0].id,
            "office.pptx-briefing"
        );
        assert_eq!(
            pack.retrieve("export june sales .xlsx workbook", 1)[0].id,
            "office.xlsx-workbook"
        );
        assert_eq!(
            pack.retrieve("design database schema with foreign keys", 1)[0].id,
            "db.schema-first"
        );
        assert_eq!(
            pack.retrieve("write SQL select join query", 1)[0].id,
            "sql.query-safe"
        );
    }
}
