use crate::{digest::digest, Error, Result, RunContext};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApprovalBinding {
    pub scope_digest: String,
    pub run_id: Uuid,
    pub operation: String,
    pub arguments_digest: String,
}
impl ApprovalBinding {
    pub fn new<T: Serialize>(ctx: &RunContext, operation: &str, args: &T) -> Result<Self> {
        ctx.check()?;
        Ok(Self {
            scope_digest: ctx.scope().binding()?,
            run_id: ctx.id(),
            operation: operation.into(),
            arguments_digest: digest(args)?,
        })
    }
}
struct Grant {
    binding: ApprovalBinding,
    expires: Instant,
}
/// Opaque one-use handle, deliberately NOT Deserialize. Keep it out of LLM input.
#[derive(Debug)]
pub struct ApprovalTicket(Uuid);
#[derive(Default)]
pub struct ApprovalVault {
    entries: Mutex<HashMap<Uuid, Grant>>,
}
impl ApprovalVault {
    /// HOST ONLY: call after an authenticated UI approver is authorized for binding.scope.
    /// This vault does not authenticate HTTP requests or replace SecurityLayer.
    pub fn issue_from_trusted_ui(
        &self,
        binding: ApprovalBinding,
        ttl: Duration,
    ) -> Result<ApprovalTicket> {
        if ttl.is_zero() || ttl > Duration::from_secs(300) {
            return Err(Error::Invalid("approval TTL".into()));
        }
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| Error::Denied("approval lock".into()))?;
        entries.retain(|_, entry| entry.expires > Instant::now());
        if entries.len() >= 1024 {
            return Err(Error::Capacity);
        }
        let id = Uuid::new_v4();
        entries.insert(
            id,
            Grant {
                binding,
                expires: Instant::now() + ttl,
            },
        );
        Ok(ApprovalTicket(id))
    }
    pub fn consume(&self, ticket: ApprovalTicket, binding: &ApprovalBinding) -> Result<()> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| Error::Denied("approval lock".into()))?;
        let grant = entries
            .remove(&ticket.0)
            .ok_or_else(|| Error::Denied("approval unknown/consumed".into()))?;
        if grant.expires <= Instant::now() || grant.binding != *binding {
            return Err(Error::Denied("approval binding/expiry mismatch".into()));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> ApprovalBinding {
        ApprovalBinding {
            scope_digest: "scope".into(),
            run_id: Uuid::new_v4(),
            operation: "computer.click".into(),
            arguments_digest: "a".into(),
        }
    }
    #[test]
    fn changed_arguments_cannot_reuse_approval() {
        let vault = ApprovalVault::default();
        let a = binding();
        let token = vault
            .issue_from_trusted_ui(a.clone(), Duration::from_secs(10))
            .unwrap();
        let mut changed = a;
        changed.arguments_digest = "b".into();
        assert!(vault.consume(token, &changed).is_err());
    }
    #[test]
    fn approval_is_single_use() {
        let vault = ApprovalVault::default();
        let b = binding();
        let ticket = vault
            .issue_from_trusted_ui(b.clone(), Duration::from_secs(10))
            .unwrap();
        let copied_for_test = ApprovalTicket(ticket.0);
        vault.consume(ticket, &b).unwrap();
        assert!(vault.consume(copied_for_test, &b).is_err());
    }
}
