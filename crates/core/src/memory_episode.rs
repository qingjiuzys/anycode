//! Structured episodic events and V2 memory metadata (compatible with legacy Memory rows).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::memory_model::MemoryType;

/// Fine-grained memory kind layered on top of legacy [`MemoryType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    #[default]
    Episode,
    Preference,
    Fact,
    Decision,
    Strategy,
    Skill,
}

impl MemoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Episode => "episode",
            Self::Preference => "preference",
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Strategy => "strategy",
            Self::Skill => "skill",
        }
    }

    pub fn from_str_loose(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "episode" => Some(Self::Episode),
            "preference" => Some(Self::Preference),
            "fact" => Some(Self::Fact),
            "decision" => Some(Self::Decision),
            "strategy" => Some(Self::Strategy),
            "skill" => Some(Self::Skill),
            _ => None,
        }
    }

    /// Default legacy MemoryType bucket for a kind.
    pub fn default_memory_type(self) -> MemoryType {
        match self {
            Self::Preference => MemoryType::User,
            Self::Episode | Self::Decision | Self::Strategy | Self::Skill | Self::Fact => {
                MemoryType::Project
            }
        }
    }
}

/// 记忆评分调查统计（`surveyRating` frontmatter：`count` / `mean` / `total`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SurveyRating {
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub mean: f32,
    #[serde(default)]
    pub total: u32,
}

impl SurveyRating {
    /// 追加一次评分（bad/fine/good → 1/2/3），更新均值。
    pub fn record(&mut self, rating: u8) {
        let rating = rating.max(1);
        self.total += rating as u32;
        self.count += 1;
        self.mean = self.total as f32 / self.count as f32;
    }
}

/// V2 metadata attached to consolidated memories (optional on legacy rows).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MemoryMetaV2 {
    #[serde(default)]
    pub kind: MemoryKind,
    #[serde(default)]
    pub importance: f32,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub evidence_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub forgotten: bool,
    /// 记忆评分回写（`surveyRating`）；`None` 表示尚无评分。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survey_rating: Option<SurveyRating>,
}

/// Structured waking-period event (not raw tool dumps).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EpisodeEvent {
    TaskIntent {
        summary: String,
        #[serde(default)]
        family: String,
    },
    KeyDecision {
        decision: String,
        #[serde(default)]
        rationale: String,
    },
    ToolTrace {
        tool: String,
        #[serde(default)]
        outcome: String,
        #[serde(default)]
        ok: bool,
    },
    Acceptance {
        criterion: String,
        passed: bool,
        #[serde(default)]
        evidence: String,
    },
    UserCorrection {
        before: String,
        after: String,
    },
    Deliverable {
        path_or_title: String,
        #[serde(default)]
        summary: String,
    },
}

impl EpisodeEvent {
    pub fn to_structured_text(&self) -> String {
        match self {
            Self::TaskIntent { summary, family } => {
                format!("[intent family={family}] {summary}")
            }
            Self::KeyDecision {
                decision,
                rationale,
            } => format!("[decision] {decision} ({rationale})"),
            Self::ToolTrace { tool, outcome, ok } => {
                format!("[tool {tool} ok={ok}] {outcome}")
            }
            Self::Acceptance {
                criterion,
                passed,
                evidence,
            } => format!("[accept passed={passed}] {criterion} :: {evidence}"),
            Self::UserCorrection { before, after } => {
                format!("[correction] {before} → {after}")
            }
            Self::Deliverable {
                path_or_title,
                summary,
            } => format!("[deliverable] {path_or_title}: {summary}"),
        }
    }

    /// Stable content hash aligned with runtime evidence index style.
    pub fn evidence_hash(payload: &str) -> String {
        let mut h = DefaultHasher::new();
        payload.hash(&mut h);
        format!("{:016x}", h.finish())
    }
}

/// Waking buffer entry awaiting dream consolidation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeRecord {
    pub id: String,
    pub session_id: String,
    pub task_id: String,
    pub events: Vec<EpisodeEvent>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub evidence_hash: String,
}

impl EpisodeRecord {
    pub fn recompute_hash(&mut self) {
        let blob = self
            .events
            .iter()
            .map(EpisodeEvent::to_structured_text)
            .collect::<Vec<_>>()
            .join("\n");
        self.evidence_hash = EpisodeEvent::evidence_hash(&blob);
    }
}

/// Secret-like patterns that must not enter long-term memory.
pub fn looks_like_secret(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    const NEEDLES: &[&str] = &[
        "api_key",
        "apikey",
        "secret_key",
        "private_key",
        "-----begin",
        "password=",
        "authorization: bearer",
        "wxp_",
    ];
    if NEEDLES.iter().any(|n| lower.contains(n)) {
        return true;
    }
    // `sk-` API keys: only match at a token boundary, otherwise normal words
    // like "task-x" / "risk-based" / "desk-top" would be killed as secrets.
    let bytes = lower.as_bytes();
    for (i, _) in lower.match_indices("sk-") {
        let prev_ok = i == 0 || !matches!(bytes[i - 1], b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_');
        if prev_ok {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_secrets_and_hashes_evidence() {
        assert!(looks_like_secret("Authorization: Bearer sk-abc"));
        assert!(looks_like_secret("key = sk-live-9f2c"));
        assert!(!looks_like_secret("prefer dark theme"));
        // `sk-` inside ordinary words must not kill the episode
        assert!(!looks_like_secret("finish task-x migration"));
        assert!(!looks_like_secret("risk-based rollout"));
        assert!(!looks_like_secret("desk-top layout"));
        let h = EpisodeEvent::evidence_hash("hello");
        assert_eq!(h.len(), 16);
    }
}
