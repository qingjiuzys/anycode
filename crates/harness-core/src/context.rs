use crate::{budget::BudgetPool, digest::digest, Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Authentication is not authorization. Construct on the trusted host only AFTER
/// account introspection AND product/project ACL lookup. Never deserialize RunContext.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub subject: Uuid,
    pub organization: Option<Uuid>,
    pub tenant: Option<Uuid>,
    pub project: Uuid,
    pub device: Option<Uuid>,
}
impl Scope {
    pub fn validate(&self) -> Result<()> {
        if self.subject.is_nil()
            || self.project.is_nil()
            || self.organization.is_some() != self.tenant.is_some()
            || self.organization.is_some_and(|id| id.is_nil())
            || self.tenant.is_some_and(|id| id.is_nil())
            || self.device.is_some_and(|id| id.is_nil())
        {
            return Err(Error::Invalid("incomplete identity/project scope".into()));
        }
        Ok(())
    }
    pub fn binding(&self) -> Result<String> {
        self.validate()?;
        digest(self)
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Capabilities(BTreeSet<String>);
impl Capabilities {
    pub fn new(values: impl IntoIterator<Item = String>) -> Result<Self> {
        let set: BTreeSet<_> = values.into_iter().collect();
        if set.iter().any(|s| {
            s.is_empty()
                || s.len() > 128
                || s.contains('*')
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        }) {
            return Err(Error::Invalid(
                "capabilities must be exact identifiers, never wildcards".into(),
            ));
        }
        Ok(Self(set))
    }
    pub fn allows(&self, name: &str) -> bool {
        self.0.contains(name)
    }
    pub fn require(&self, name: &str) -> Result<()> {
        if self.allows(name) {
            Ok(())
        } else {
            Err(Error::Denied(name.into()))
        }
    }
    pub fn attenuate(&self, requested: &Self) -> Result<Self> {
        if !requested.0.is_subset(&self.0) {
            return Err(Error::Denied(
                "child requested capabilities outside parent".into(),
            ));
        }
        Ok(requested.clone())
    }
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}
#[derive(Clone)]
struct Cancellation {
    own: Arc<AtomicBool>,
    ancestors: Vec<Arc<AtomicBool>>,
}
impl Cancellation {
    fn root() -> Self {
        Self {
            own: Arc::new(AtomicBool::new(false)),
            ancestors: vec![],
        }
    }
    fn child(&self) -> Self {
        let mut ancestors = self.ancestors.clone();
        ancestors.push(self.own.clone());
        Self {
            own: Arc::new(AtomicBool::new(false)),
            ancestors,
        }
    }
    fn cancelled(&self) -> bool {
        self.own.load(Ordering::Acquire) || self.ancestors.iter().any(|s| s.load(Ordering::Acquire))
    }
}
#[derive(Clone)]
pub struct RunContext {
    scope: Arc<Scope>,
    id: Uuid,
    root: Uuid,
    parent: Option<Uuid>,
    depth: u16,
    capabilities: Capabilities,
    budget: BudgetPool,
    cancel: Cancellation,
    deadline: Instant,
}
impl RunContext {
    pub fn root(
        scope: Scope,
        capabilities: Capabilities,
        budget: BudgetPool,
        lifetime: Duration,
    ) -> Result<Self> {
        scope.validate()?;
        let capabilities = Capabilities::new(capabilities.iter().map(str::to_owned))?;
        if lifetime.is_zero() {
            return Err(Error::Invalid("zero lifetime".into()));
        }
        let id = Uuid::new_v4();
        let deadline = Instant::now()
            .checked_add(lifetime)
            .ok_or_else(|| Error::Invalid("lifetime overflow".into()))?;
        Ok(Self {
            scope: Arc::new(scope),
            id,
            root: id,
            parent: None,
            depth: 0,
            capabilities,
            budget,
            cancel: Cancellation::root(),
            deadline,
        })
    }
    pub fn child(
        &self,
        capabilities: &Capabilities,
        max_depth: u16,
        lifetime: Duration,
    ) -> Result<Self> {
        self.check()?;
        if lifetime.is_zero() || self.depth >= max_depth {
            return Err(Error::Denied("subagent depth/lifetime limit".into()));
        }
        let local_deadline = Instant::now()
            .checked_add(lifetime)
            .ok_or_else(|| Error::Invalid("lifetime overflow".into()))?;
        Ok(Self {
            scope: self.scope.clone(),
            id: Uuid::new_v4(),
            root: self.root,
            parent: Some(self.id),
            depth: self.depth + 1,
            capabilities: self.capabilities.attenuate(capabilities)?,
            budget: self.budget.clone(),
            cancel: self.cancel.child(),
            deadline: self.deadline.min(local_deadline),
        })
    }
    pub fn check(&self) -> Result<()> {
        if self.cancel.cancelled() || Instant::now() >= self.deadline {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    pub async fn cancelled(&self) {
        while self.check().is_ok() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    pub fn cancel(&self) {
        self.cancel.own.store(true, Ordering::Release);
    }
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn root_id(&self) -> Uuid {
        self.root
    }
    pub fn parent_id(&self) -> Option<Uuid> {
        self.parent
    }
    pub fn depth(&self) -> u16 {
        self.depth
    }
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
    pub fn budget(&self) -> &BudgetPool {
        &self.budget
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> RunContext {
        RunContext::root(
            Scope {
                subject: Uuid::new_v4(),
                organization: None,
                tenant: None,
                project: Uuid::new_v4(),
                device: None,
            },
            Capabilities::new(["fs.read".into()]).unwrap(),
            BudgetPool::new(1000).unwrap(),
            Duration::from_secs(60),
        )
        .unwrap()
    }
    #[test]
    fn siblings_have_independent_lineage_and_cancel() {
        let p = root();
        let a = p
            .child(p.capabilities(), 4, Duration::from_secs(5))
            .unwrap();
        let b = p
            .child(p.capabilities(), 4, Duration::from_secs(5))
            .unwrap();
        assert_eq!(a.depth(), 1);
        assert_eq!(b.depth(), 1);
        a.cancel();
        assert!(a.check().is_err());
        assert!(b.check().is_ok());
        p.cancel();
        assert!(b.check().is_err());
    }
    #[test]
    fn child_cannot_gain_a_capability() {
        let p = root();
        assert!(p
            .child(
                &Capabilities::new(["computer.input".into()]).unwrap(),
                4,
                Duration::from_secs(2)
            )
            .is_err());
    }
    #[test]
    fn wildcard_and_incomplete_tenant_denied() {
        assert!(Capabilities::new(["*".into()]).is_err());
        let mut s = root().scope().clone();
        s.tenant = Some(Uuid::new_v4());
        assert!(s.validate().is_err());
    }
}
