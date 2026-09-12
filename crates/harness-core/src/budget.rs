use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetSnapshot {
    pub limit: u64,
    pub spent: u64,
    pub reserved: u64,
    /// Provider usage can exceed a host's estimate. Record truth and stop, never clamp it away.
    pub overdrawn: bool,
}
#[derive(Clone)]
pub struct BudgetPool(Arc<Mutex<BudgetSnapshot>>);
impl BudgetPool {
    pub fn new(limit: u64) -> Result<Self> {
        if limit == 0 {
            return Err(Error::Invalid("zero budget".into()));
        }
        Ok(Self(Arc::new(Mutex::new(BudgetSnapshot {
            limit,
            ..Default::default()
        }))))
    }
    /// Trusted storage only. In-flight reservations have unknown provider usage;
    /// conservatively charge them on restart rather than making them free.
    pub fn from_checkpoint(mut state: BudgetSnapshot) -> Result<Self> {
        if state.limit == 0 {
            return Err(Error::Invalid("zero restored budget".into()));
        }
        state.spent = state
            .spent
            .checked_add(state.reserved)
            .ok_or(Error::Budget)?;
        state.reserved = 0;
        state.overdrawn |= state.spent > state.limit;
        Ok(Self(Arc::new(Mutex::new(state))))
    }
    pub fn snapshot(&self) -> Result<BudgetSnapshot> {
        self.0
            .lock()
            .map(|s| *s)
            .map_err(|_| Error::Host("budget lock poisoned".into()))
    }
    pub fn reserve(&self, maximum_tokens: u64) -> Result<Reservation> {
        if maximum_tokens == 0 {
            return Err(Error::Invalid("zero reservation".into()));
        }
        let mut state = self.0.lock().map_err(|_| Error::Budget)?;
        let used = state
            .spent
            .checked_add(state.reserved)
            .ok_or(Error::Budget)?;
        if state.overdrawn || maximum_tokens > state.limit.saturating_sub(used) {
            return Err(Error::Budget);
        }
        state.reserved += maximum_tokens;
        Ok(Reservation {
            pool: self.clone(),
            maximum_tokens,
            settled: false,
            sent: false,
        })
    }
}
/// A cancelled or disconnected *sent* request is charged its reserved maximum,
/// since the provider may still bill it. An unsent reservation is released on drop.
pub struct Reservation {
    pool: BudgetPool,
    maximum_tokens: u64,
    settled: bool,
    sent: bool,
}
impl Reservation {
    pub fn mark_sent(&mut self) {
        self.sent = true;
    }
    pub fn settle(mut self, actual_tokens: u64) -> Result<()> {
        let overdrawn = self.finish(actual_tokens)?;
        if overdrawn {
            Err(Error::Budget)
        } else {
            Ok(())
        }
    }
    fn finish(&mut self, actual: u64) -> Result<bool> {
        let mut state = self.pool.0.lock().map_err(|_| Error::Budget)?;
        state.reserved = state
            .reserved
            .checked_sub(self.maximum_tokens)
            .ok_or(Error::Budget)?;
        state.spent = state.spent.saturating_add(actual);
        state.overdrawn |= state.spent.saturating_add(state.reserved) > state.limit;
        self.settled = true;
        Ok(state.overdrawn)
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.settled {
            let charge = if self.sent { self.maximum_tokens } else { 0 };
            let _ = self.finish(charge);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservations_cannot_multiply_budget() {
        let pool = BudgetPool::new(100).unwrap();
        let a = pool.reserve(70).unwrap();
        assert!(matches!(pool.reserve(31), Err(Error::Budget)));
        a.settle(30).unwrap();
        assert_eq!(pool.snapshot().unwrap().spent, 30);
        assert!(pool.reserve(70).is_ok());
    }
    #[test]
    fn unknown_sent_usage_is_not_free() {
        let pool = BudgetPool::new(100).unwrap();
        {
            let mut r = pool.reserve(60).unwrap();
            r.mark_sent();
        }
        assert_eq!(pool.snapshot().unwrap().spent, 60);
        {
            let _r = pool.reserve(40).unwrap();
        }
        assert_eq!(pool.snapshot().unwrap().reserved, 0);
    }
    #[test]
    fn overage_is_recorded_not_hidden() {
        let pool = BudgetPool::new(10).unwrap();
        assert!(pool.reserve(10).unwrap().settle(12).is_err());
        assert_eq!(pool.snapshot().unwrap().spent, 12);
        assert!(pool.reserve(1).is_err());
    }
}
