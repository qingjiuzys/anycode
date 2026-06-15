//! LLM query source — controls retry/backoff policy (foreground vs background).

use serde::{Deserialize, Serialize};

/// Why an LLM request was issued; mirrors Claude Code query-source tiers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuerySource {
    /// User-blocking main agent turn (REPL / dashboard session).
    #[default]
    MainTurn,
    /// Context compaction / summarization during a live session.
    Compact,
    /// Session title generation.
    Title,
    /// Post-turn or post-task summary.
    Summary,
    /// Security / routing classifiers.
    Classifier,
    /// Verification or gate agents.
    Verification,
    /// Hook or side-channel prompts.
    HookPrompt,
    /// Best-effort background work — do not amplify 429/529 retries.
    Background,
}

impl QuerySource {
    #[must_use]
    pub fn is_foreground(self) -> bool {
        matches!(
            self,
            Self::MainTurn | Self::Compact | Self::Verification | Self::HookPrompt
        )
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MainTurn => "main_turn",
            Self::Compact => "compact",
            Self::Title => "title",
            Self::Summary => "summary",
            Self::Classifier => "classifier",
            Self::Verification => "verification",
            Self::HookPrompt => "hook_prompt",
            Self::Background => "background",
        }
    }
}
