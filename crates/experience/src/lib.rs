//! Offline experience distillation helpers (teacher lab tooling).
//!
//! Runtime only loads signed/validated packs; teacher API keys never ship in Desktop.

use anycode_core::{ExperienceCard, ExperiencePack, ExperiencePackMeta, TaskFamily};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeacherTrajectory {
    pub id: String,
    pub family: TaskFamily,
    pub prompt: String,
    #[serde(default)]
    pub tool_order: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub passed_gates: bool,
    #[serde(default)]
    pub low_model_replay_gain: f64,
}

/// Compress a successful teacher trajectory into an experience card candidate.
pub fn distill_card(traj: &TeacherTrajectory) -> ExperienceCard {
    ExperienceCard {
        id: format!("distill.{}", traj.id),
        title: format!("Distilled: {}", traj.id),
        family: traj.family,
        applicable_when: vec![traj.prompt.chars().take(80).collect()],
        task_breakdown: traj.notes.clone(),
        tool_order: traj.tool_order.clone(),
        key_checks: traj
            .notes
            .iter()
            .filter(|n| n.to_ascii_lowercase().contains("check"))
            .cloned()
            .collect(),
        common_failures: Vec::new(),
        recovery: vec!["replay failed gate with evidence".into()],
        examples: vec![traj.prompt.clone()],
        model_compat: vec!["weak_local".into()],
        regression_score: traj.low_model_replay_gain,
        version: "0.1.0".into(),
    }
}

/// Only keep trajectories that passed real gates and helped the low model.
pub fn filter_validated(trajs: &[TeacherTrajectory], min_gain: f64) -> Vec<&TeacherTrajectory> {
    trajs
        .iter()
        .filter(|t| t.passed_gates && t.low_model_replay_gain >= min_gain)
        .collect()
}

pub fn sign_pack_hmac_like(pack: &mut ExperiencePack, secret: &str) {
    let payload = pack.signing_payload().unwrap_or_default();
    let mut h = DefaultHasher::new();
    secret.hash(&mut h);
    payload.hash(&mut h);
    pack.meta.signature_hex = format!("{:016x}", h.finish());
    pack.meta.signer = "offline-teacher-lab".into();
    if pack.meta.created_at.is_none() {
        pack.meta.created_at = Some(Utc::now());
    }
}

pub fn verify_pack_hmac_like(pack: &ExperiencePack, secret: &str) -> bool {
    if pack.meta.signature_hex.is_empty() {
        return false;
    }
    let payload = match pack.signing_payload() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let mut h = DefaultHasher::new();
    secret.hash(&mut h);
    payload.hash(&mut h);
    pack.meta.signature_hex == format!("{:016x}", h.finish())
}

pub fn build_pack_from_trajectories(
    id: &str,
    version: &str,
    trajs: &[TeacherTrajectory],
    min_gain: f64,
) -> ExperiencePack {
    let cards = filter_validated(trajs, min_gain)
        .into_iter()
        .map(distill_card)
        .collect::<Vec<_>>();
    let regression_score = if cards.is_empty() {
        0.0
    } else {
        cards.iter().map(|c| c.regression_score).sum::<f64>() / cards.len() as f64
    };
    ExperiencePack {
        meta: ExperiencePackMeta {
            id: id.into(),
            version: version.into(),
            model_compat: vec!["weak_local".into(), "*".into()],
            regression_score,
            created_at: Some(Utc::now()),
            signature_hex: String::new(),
            signer: String::new(),
        },
        cards,
    }
}

pub fn load_pack(path: impl AsRef<Path>) -> anyhow::Result<ExperiencePack> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_pack(path: impl AsRef<Path>, pack: &ExperiencePack) -> anyhow::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(pack)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::TaskFamily;

    #[test]
    fn distill_sign_verify() {
        let trajs = vec![TeacherTrajectory {
            id: "web1".into(),
            family: TaskFamily::WebDesign,
            prompt: "dark landing page".into(),
            tool_order: vec!["Write".into(), "BrowserScreenshot".into()],
            notes: vec!["check contrast".into()],
            passed_gates: true,
            low_model_replay_gain: 0.3,
        }];
        let mut pack = build_pack_from_trajectories("lab", "0.1.0", &trajs, 0.1);
        assert_eq!(pack.cards.len(), 1);
        sign_pack_hmac_like(&mut pack, "test-secret");
        assert!(verify_pack_hmac_like(&pack, "test-secret"));
        assert!(!verify_pack_hmac_like(&pack, "other"));
    }
}
