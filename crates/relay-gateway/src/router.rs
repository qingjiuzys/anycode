use crate::store::{RelayAccount, RelayStore, DEFAULT_AGNES_CHAT_URL};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AccountPick {
    pub account: RelayAccount,
    pub upstream_model: String,
}

pub fn resolve_upstream_model(store: &RelayStore, requested: &str) -> String {
    let req = requested.trim();
    if req.is_empty() {
        return store.config.default_model.clone();
    }
    if let Some(m) = store.models.iter().find(|m| m.id == req) {
        return m
            .upstream_model
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| req.to_string());
    }
    req.to_string()
}

pub fn active_accounts(store: &RelayStore) -> Vec<RelayAccount> {
    store
        .accounts
        .iter()
        .filter(|a| a.status == "active")
        .cloned()
        .collect()
}

pub fn pick_accounts(store: &RelayStore, rr: &AtomicUsize) -> Vec<RelayAccount> {
    let accounts = active_accounts(store);
    if accounts.is_empty() {
        return accounts;
    }

    let mode = store.config.rotation_mode.trim();
    if mode == "pinned" {
        return pick_pinned_account(store, accounts);
    }
    if mode == "weighted" {
        return pick_weighted_accounts(&accounts, rr);
    }
    pick_round_robin_accounts(accounts, rr)
}

fn pick_pinned_account(store: &RelayStore, accounts: Vec<RelayAccount>) -> Vec<RelayAccount> {
    if let Some(id) = store.config.selected_account_id.as_deref() {
        if !id.is_empty() && id != "auto" {
            if let Some(one) = accounts.into_iter().find(|a| a.id == id) {
                return vec![one];
            }
            return Vec::new();
        }
    }
    accounts.into_iter().take(1).collect()
}

fn pick_round_robin_accounts(
    mut accounts: Vec<RelayAccount>,
    rr: &AtomicUsize,
) -> Vec<RelayAccount> {
    if accounts.len() <= 1 {
        return accounts;
    }
    let start = rr.fetch_add(1, Ordering::Relaxed) % accounts.len();
    accounts.rotate_left(start);
    accounts
}

fn pick_weighted_accounts(accounts: &[RelayAccount], rr: &AtomicUsize) -> Vec<RelayAccount> {
    let mut pool = Vec::new();
    for account in accounts {
        let weight = account.weight.max(1) as usize;
        for _ in 0..weight {
            pool.push(account.clone());
        }
    }
    if pool.is_empty() {
        return Vec::new();
    }
    if pool.len() == 1 {
        return vec![pool[0].clone()];
    }

    let start = rr.fetch_add(1, Ordering::Relaxed) % pool.len();
    let mut ordered = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for offset in 0..pool.len() {
        let account = &pool[(start + offset) % pool.len()];
        if seen.insert(account.id.clone()) {
            ordered.push(account.clone());
        }
    }
    ordered
}

pub fn account_chat_url(account: &RelayAccount) -> String {
    account
        .base_url
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| DEFAULT_AGNES_CHAT_URL.to_string())
}

pub fn model_for_account(store: &RelayStore, model_id: &str, account_id: &str) -> String {
    if let Some(m) = store
        .models
        .iter()
        .find(|m| m.id == model_id && m.account_id.as_deref() == Some(account_id))
    {
        return m
            .upstream_model
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| model_id.to_string());
    }
    resolve_upstream_model(store, model_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{RelayConfig, RelayModel, RelayStore};

    fn sample_store() -> RelayStore {
        RelayStore {
            accounts: vec![
                RelayAccount {
                    id: "a1".into(),
                    name: "One".into(),
                    api_key: Some("k1".into()),
                    base_url: None,
                    status: "active".into(),
                    weight: 1,
                },
                RelayAccount {
                    id: "a2".into(),
                    name: "Two".into(),
                    api_key: Some("k2".into()),
                    base_url: None,
                    status: "active".into(),
                    weight: 3,
                },
            ],
            config: RelayConfig::default(),
            models: vec![RelayModel {
                id: "my-chat".into(),
                display_name: Some("My Chat".into()),
                upstream_model: Some("agnes-chat".into()),
                account_id: None,
            }],
        }
    }

    #[test]
    fn resolve_model_mapping() {
        let store = sample_store();
        assert_eq!(resolve_upstream_model(&store, "my-chat"), "agnes-chat");
        assert_eq!(resolve_upstream_model(&store, "other"), "other");
    }

    #[test]
    fn pick_selected_account() {
        let mut store = sample_store();
        store.config.rotation_mode = "pinned".into();
        store.config.selected_account_id = Some("a2".into());
        let rr = AtomicUsize::new(0);
        let picked = pick_accounts(&store, &rr);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].id, "a2");
    }

    #[test]
    fn round_robin_rotates() {
        let store = sample_store();
        let rr = AtomicUsize::new(0);
        assert_eq!(pick_accounts(&store, &rr)[0].id, "a1");
        assert_eq!(pick_accounts(&store, &rr)[0].id, "a2");
    }

    #[test]
    fn weighted_favors_heavier_account_over_ticks() {
        let mut store = sample_store();
        store.config.rotation_mode = "weighted".into();
        let rr = AtomicUsize::new(0);
        let mut a1 = 0usize;
        let mut a2 = 0usize;
        for _ in 0..12 {
            match pick_accounts(&store, &rr)[0].id.as_str() {
                "a1" => a1 += 1,
                "a2" => a2 += 1,
                _ => {}
            }
        }
        assert!(a2 > a1);
    }
}
