//! Structured, bounded concurrency. Dropping a parent future cancels its children;
//! there are no untracked tokio::spawn tasks or process-wide recursion counters.
use anycode_harness_core::{Capabilities, Error, Result, RunContext};
use async_trait::async_trait;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Assignment {
    pub agent: String,
    pub prompt: String,
    pub capabilities: Capabilities,
}
#[derive(Clone, Debug)]
pub struct ChildOutcome {
    pub run_id: Uuid,
    pub output: Value,
    pub partial: bool,
}
#[async_trait]
pub trait ChildExecutor: Send + Sync {
    /// The executor must call the SAME Kernel; the selected agent is just a profile.
    /// `ctx` carries attenuated authority and shared budget. Do not create a new root.
    async fn execute(&self, ctx: RunContext, assignment: Assignment) -> Result<ChildOutcome>;
}
pub struct Supervisor {
    root: Uuid,
    slots: Arc<Semaphore>,
    max_depth: u16,
}
struct CancelOnDrop(RunContext);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
impl Supervisor {
    pub fn new(root: &RunContext, concurrency: usize, max_depth: u16) -> Result<Self> {
        if !(1..=32).contains(&concurrency) || !(1..=8).contains(&max_depth) {
            return Err(Error::Invalid("subagent concurrency/depth".into()));
        }
        Ok(Self {
            root: root.root_id(),
            slots: Arc::new(Semaphore::new(concurrency)),
            max_depth,
        })
    }
    pub async fn run(
        &self,
        parent: &RunContext,
        assignment: Assignment,
        executor: &dyn ChildExecutor,
    ) -> Result<ChildOutcome> {
        parent.check()?;
        parent.capabilities().require("agent.spawn")?;
        if parent.root_id() != self.root {
            return Err(Error::Denied("supervisor belongs to another root".into()));
        }
        if assignment.agent.is_empty()
            || assignment.agent.len() > 128
            || assignment.prompt.is_empty()
            || assignment.prompt.len() > 128 * 1024
        {
            return Err(Error::Invalid("subagent assignment bounds".into()));
        }
        // Fail fast: parents retaining all slots while awaiting nested children would
        // deadlock if this used acquire().await. Caller may retry after a sibling ends.
        let _permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Capacity)?;
        let child = parent.child(
            &assignment.capabilities,
            self.max_depth,
            Duration::from_secs(3600),
        )?;
        let expected = child.id();
        let guard = CancelOnDrop(child.clone());
        let result = tokio::select! {
            _ = child.cancelled() => Err(Error::Cancelled),
            result = executor.execute(child.clone(), assignment) => result,
        }?;
        if result.run_id != expected {
            return Err(Error::Conflict(
                "child result belongs to another run".into(),
            ));
        }
        drop(guard);
        Ok(result)
    }

    /// Reserve a child context without executing. Join/Cancel use this handle.
    pub fn prepare(
        &self,
        parent: &RunContext,
        assignment: &Assignment,
    ) -> Result<(tokio::sync::OwnedSemaphorePermit, RunContext)> {
        parent.check()?;
        parent.capabilities().require("agent.spawn")?;
        if parent.root_id() != self.root {
            return Err(Error::Denied("supervisor belongs to another root".into()));
        }
        if assignment.agent.is_empty()
            || assignment.agent.len() > 128
            || assignment.prompt.is_empty()
            || assignment.prompt.len() > 128 * 1024
        {
            return Err(Error::Invalid("subagent assignment bounds".into()));
        }
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Capacity)?;
        let child = parent.child(
            &assignment.capabilities,
            self.max_depth,
            Duration::from_secs(3600),
        )?;
        Ok((permit, child))
    }

    pub async fn run_prepared(
        &self,
        permit: tokio::sync::OwnedSemaphorePermit,
        child: RunContext,
        assignment: Assignment,
        executor: &dyn ChildExecutor,
    ) -> Result<ChildOutcome> {
        let expected = child.id();
        let guard = CancelOnDrop(child.clone());
        let result = tokio::select! {
            _ = child.cancelled() => Err(Error::Cancelled),
            result = executor.execute(child.clone(), assignment) => result,
        }?;
        if result.run_id != expected {
            return Err(Error::Conflict(
                "child result belongs to another run".into(),
            ));
        }
        drop(guard);
        drop(permit);
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use anycode_harness_core::{budget::BudgetPool, Scope};
    fn root() -> RunContext {
        RunContext::root(
            Scope {
                subject: Uuid::new_v4(),
                organization: None,
                tenant: None,
                project: Uuid::new_v4(),
                device: None,
            },
            Capabilities::new(["agent.spawn".into(), "fs.read".into()]).unwrap(),
            BudgetPool::new(1000).unwrap(),
            Duration::from_secs(60),
        )
        .unwrap()
    }
    struct Echo;
    #[async_trait]
    impl ChildExecutor for Echo {
        async fn execute(&self, ctx: RunContext, _: Assignment) -> Result<ChildOutcome> {
            assert_eq!(ctx.depth(), 1);
            ctx.budget().reserve(10)?.settle(7)?;
            Ok(ChildOutcome {
                run_id: ctx.id(),
                output: Value::Null,
                partial: false,
            })
        }
    }
    #[tokio::test]
    async fn child_consumes_parent_budget_without_changing_parent_depth() {
        let p = root();
        let sup = Supervisor::new(&p, 2, 4).unwrap();
        sup.run(
            &p,
            Assignment {
                agent: "explore".into(),
                prompt: "read".into(),
                capabilities: Capabilities::new(["fs.read".into()]).unwrap(),
            },
            &Echo,
        )
        .await
        .unwrap();
        assert_eq!(p.depth(), 0);
        assert_eq!(p.budget().snapshot().unwrap().spent, 7);
        assert!(p.check().is_ok());
    }
    #[tokio::test]
    async fn unrelated_root_cannot_borrow_supervisor() {
        let p = root();
        let q = root();
        let sup = Supervisor::new(&p, 2, 4).unwrap();
        assert!(sup
            .run(
                &q,
                Assignment {
                    agent: "explore".into(),
                    prompt: "read".into(),
                    capabilities: q.capabilities().clone()
                },
                &Echo
            )
            .await
            .is_err());
    }
}
