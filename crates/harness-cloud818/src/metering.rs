//! Provider-usage fact envelope, NOT another wallet and NOT a fabricated billing API.
//! Send through a transactional outbox once 818cloud's metering endpoint is agreed.
use anycode_harness_core::{digest::digest, Error, Result, RunContext};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageReceipt {
    pub version: u32,
    pub idempotency_key: String,
    pub product: String,
    pub root_run_id: Uuid,
    pub run_id: Uuid,
    pub parent_run_id: Option<Uuid>,
    pub scope_digest: String,
    pub turn: u64,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    pub measured: bool,
}
impl UsageReceipt {
    pub fn new(
        ctx: &RunContext,
        turn: u64,
        provider: &str,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        measured: bool,
    ) -> Result<Self> {
        if turn_and_model_invalid(turn, provider, model) {
            return Err(Error::Invalid("usage receipt metadata".into()));
        }
        let scope = ctx.scope().binding()?;
        let key = digest(&("anycode-usage-v1", &scope, ctx.id(), turn))?;
        Ok(Self {
            version: 1,
            idempotency_key: key,
            product: "anycode".into(),
            root_run_id: ctx.root_id(),
            run_id: ctx.id(),
            parent_run_id: ctx.parent_id(),
            scope_digest: scope,
            turn,
            provider: provider.into(),
            model: model.into(),
            input_tokens,
            output_tokens,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            measured,
        })
    }

    /// Persist only Kernel `usage` events that already recorded measured tokens.
    /// Unmeasured reservations are not wallet debits and must not be settled.
    pub fn from_measured_event(
        event: &anycode_harness_core::events::Event,
        provider: &str,
        model: &str,
    ) -> Result<Self> {
        if event.kind != "usage"
            || event.data.get("measured") != Some(&serde_json::Value::Bool(true))
        {
            return Err(Error::Invalid("usage event is not a measured fact".into()));
        }
        let turn = event.data.get("turn").and_then(|v| v.as_u64()).unwrap_or(0);
        if turn_and_model_invalid(turn, provider, model) {
            return Err(Error::Invalid("usage receipt metadata".into()));
        }
        let usage = event
            .data
            .get("usage")
            .ok_or_else(|| Error::Invalid("measured usage payload".into()))?;
        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| Error::Invalid("measured input_tokens".into()))?;
        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| Error::Invalid("measured output_tokens".into()))?;
        let cache_creation_tokens = usage
            .get("cache_creation_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_read_tokens = usage
            .get("cache_read_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let key = digest(&("anycode-usage-v1", &event.scope_digest, event.run_id, turn))?;
        Ok(Self {
            version: 1,
            idempotency_key: key,
            product: "anycode".into(),
            root_run_id: event.root_id,
            run_id: event.run_id,
            parent_run_id: event.parent_run_id,
            scope_digest: event.scope_digest.clone(),
            turn,
            provider: provider.into(),
            model: model.into(),
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            measured: true,
        })
    }

    pub fn billable(&self) -> bool {
        self.measured
    }
}

fn turn_and_model_invalid(turn: u64, provider: &str, model: &str) -> bool {
    turn == 0
        || provider.is_empty()
        || model.is_empty()
        || provider.len() > 128
        || model.len() > 256
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_harness_core::events::Event;
    use serde_json::json;

    fn usage_event(measured: bool) -> Event {
        Event {
            version: 1,
            run_id: Uuid::new_v4(),
            root_id: Uuid::new_v4(),
            parent_run_id: None,
            scope_digest: "scope".into(),
            kind: "usage".into(),
            data: json!({
                "turn": 1,
                "measured": measured,
                "usage": {
                    "input_tokens": 4,
                    "output_tokens": 2,
                    "cache_creation_tokens": 7,
                    "cache_read_tokens": 11
                }
            }),
        }
    }

    #[test]
    fn unmeasured_reservation_is_not_a_wallet_debit() {
        assert!(UsageReceipt::from_measured_event(&usage_event(false), "p", "m").is_err());
    }

    #[test]
    fn measured_usage_is_a_fact_without_currency() {
        let receipt =
            UsageReceipt::from_measured_event(&usage_event(true), "harness-kernel", "measured")
                .unwrap();
        assert!(receipt.billable());
        let payload = serde_json::to_value(&receipt).unwrap();
        assert!(payload.get("cny").is_none());
        assert!(payload.get("wallet").is_none());
        assert_eq!(payload["input_tokens"], 4);
        assert_eq!(payload["cache_creation_tokens"], 7);
        assert_eq!(payload["cache_read_tokens"], 11);
    }
}
