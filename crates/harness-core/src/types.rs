use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A provider-native message. Core persists it without lossy text conversion.
/// Host conversion is required; UI/application events must never become system messages.
pub type ProviderMessage = Value;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// Trusted registry metadata, not model- or skill-provided metadata.
    pub capability: String,
    pub read_only: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub value: Value,
    pub is_error: bool,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
}
impl Usage {
    /// Conservative billed tokens. Cache hits are recorded, not priced in CNY.
    pub fn total(self) -> u64 {
        self.input_tokens
            .saturating_add(self.output_tokens)
            .saturating_add(self.cache_creation_tokens)
            .saturating_add(self.cache_read_tokens)
    }
}
#[derive(Clone, Debug)]
pub struct Hop {
    pub assistant: ProviderMessage,
    pub calls: Vec<Invocation>,
    /// None means unmetered/unknown; the reserved maximum is charged, NOT zero.
    pub usage: Option<Usage>,
}
#[derive(Clone, Debug)]
pub struct Limits {
    pub max_turns: usize,
    pub max_tools_per_turn: usize,
    pub max_context_bytes: usize,
    pub max_result_bytes: usize,
    pub max_arguments_bytes: usize,
    /// Conservative upper bound for input+output; NOT chars/4 labeled exact tokens.
    pub reservation_per_hop: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_turns: 64,
            max_tools_per_turn: 32,
            max_context_bytes: 4 * 1024 * 1024,
            max_result_bytes: 256 * 1024,
            max_arguments_bytes: 128 * 1024,
            reservation_per_hop: 32768,
        }
    }
}
