//! Local dream consolidation: denoise, cluster, promote, forget — offline from compaction.

use anycode_core::{
    looks_like_secret, EpisodeEvent, EpisodeRecord, Memory, MemoryKind, MemoryMetaV2, MemoryScope,
    MemoryType,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Result of one dream pass (previewable / undoable at product layer).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DreamReport {
    pub run_id: String,
    pub ingested_episodes: usize,
    pub promoted: Vec<Memory>,
    pub skipped_secrets: usize,
    pub duplicates_merged: usize,
    pub conflicts: Vec<DreamConflict>,
    pub forgotten_ids: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConflict {
    pub existing_id: String,
    pub candidate_title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamEngineSettings {
    /// Minimum importance to promote preference/fact.
    pub min_importance: f32,
    /// Decay: forget non-pinned memories older than this many days with low importance.
    pub forget_after_days: i64,
    pub max_promotions_per_run: usize,
}

impl Default for DreamEngineSettings {
    fn default() -> Self {
        Self {
            min_importance: 0.35,
            forget_after_days: 90,
            max_promotions_per_run: 64,
        }
    }
}

/// Persist waking episodes under `~/.anycode/memory/episodes/`.
pub fn episodes_dir(base: impl AsRef<Path>) -> PathBuf {
    base.as_ref().join("episodes")
}

pub fn dream_log_path(base: impl AsRef<Path>) -> PathBuf {
    base.as_ref().join("dream-runs.jsonl")
}

pub fn append_episode(base: impl AsRef<Path>, mut record: EpisodeRecord) -> std::io::Result<()> {
    record.recompute_hash();
    let dir = episodes_dir(&base);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", record.id));
    let body = serde_json::to_vec_pretty(&record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, body)
}

pub fn load_pending_episodes(base: impl AsRef<Path>) -> std::io::Result<Vec<EpisodeRecord>> {
    let dir = episodes_dir(&base);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)?;
        if let Ok(rec) = serde_json::from_str::<EpisodeRecord>(&raw) {
            out.push(rec);
        }
    }
    out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(out)
}

fn importance_for_event(ev: &EpisodeEvent) -> f32 {
    match ev {
        EpisodeEvent::UserCorrection { .. } => 0.95,
        EpisodeEvent::KeyDecision { .. } => 0.8,
        EpisodeEvent::Acceptance { passed, .. } => {
            if *passed {
                0.7
            } else {
                0.55
            }
        }
        EpisodeEvent::Deliverable { .. } => 0.65,
        EpisodeEvent::TaskIntent { .. } => 0.4,
        EpisodeEvent::ToolTrace { ok, .. } => {
            if *ok {
                0.25
            } else {
                0.45
            }
        }
    }
}

fn kind_for_event(ev: &EpisodeEvent) -> MemoryKind {
    match ev {
        EpisodeEvent::UserCorrection { .. } => MemoryKind::Preference,
        EpisodeEvent::KeyDecision { .. } => MemoryKind::Decision,
        EpisodeEvent::TaskIntent { .. } => MemoryKind::Episode,
        EpisodeEvent::ToolTrace { .. } => MemoryKind::Episode,
        EpisodeEvent::Acceptance { .. } => MemoryKind::Strategy,
        EpisodeEvent::Deliverable { .. } => MemoryKind::Fact,
    }
}

fn title_for_event(ev: &EpisodeEvent) -> String {
    let raw = ev.to_structured_text();
    if raw.chars().count() > 120 {
        raw.chars().take(120).collect::<String>() + "…"
    } else {
        raw
    }
}

fn normalize_key(text: &str) -> String {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Consolidate pending episodes into candidate long-term memories (pure, local).
pub fn consolidate_episodes(
    episodes: &[EpisodeRecord],
    existing: &[Memory],
    settings: &DreamEngineSettings,
) -> DreamReport {
    let run_id = format!("dream_{}", Uuid::new_v4());
    let mut report = DreamReport {
        run_id,
        ingested_episodes: episodes.len(),
        ..Default::default()
    };

    let mut existing_keys: HashMap<String, String> = HashMap::new();
    for mem in existing {
        existing_keys.insert(normalize_key(&mem.content), mem.id.clone());
        existing_keys.insert(normalize_key(&mem.title), mem.id.clone());
    }

    let mut promotions = 0usize;
    'episodes: for ep in episodes {
        for ev in &ep.events {
            if promotions >= settings.max_promotions_per_run {
                report
                    .notes
                    .push("hit max_promotions_per_run; remaining deferred".into());
                break 'episodes;
            }
            let text = ev.to_structured_text();
            if looks_like_secret(&text) {
                report.skipped_secrets += 1;
                continue;
            }
            let importance = importance_for_event(ev);
            if importance < settings.min_importance {
                continue;
            }
            let key = normalize_key(&text);
            if let Some(existing_id) = existing_keys.get(&key) {
                report.duplicates_merged += 1;
                report.conflicts.push(DreamConflict {
                    existing_id: existing_id.clone(),
                    candidate_title: title_for_event(ev),
                    reason: "near-duplicate content".into(),
                });
                continue;
            }
            // Preference conflict: same topic, different correction.
            if let EpisodeEvent::UserCorrection { after, .. } = ev {
                for mem in existing {
                    if mem.mem_type == MemoryType::User
                        && mem.content.contains("prefer")
                        && after.len() > 3
                        && !mem.content.contains(after.as_str())
                    {
                        report.conflicts.push(DreamConflict {
                            existing_id: mem.id.clone(),
                            candidate_title: title_for_event(ev),
                            reason: "preference may conflict with existing user memory".into(),
                        });
                    }
                }
            }
            let kind = kind_for_event(ev);
            let mem_type = match kind {
                MemoryKind::Preference => MemoryType::User,
                _ => MemoryType::Project,
            };
            let now = Utc::now();
            let memory = Memory {
                id: format!("dream_{}", Uuid::new_v4()),
                mem_type,
                title: title_for_event(ev),
                content: text.clone(),
                tags: vec![
                    "dream".into(),
                    kind.as_str().into(),
                    format!("evidence:{}", ep.evidence_hash),
                ],
                scope: MemoryScope::Private,
                created_at: now,
                updated_at: now,
                meta: Some(MemoryMetaV2 {
                    kind,
                    importance,
                    confidence: 0.7,
                    valid_from: Some(ep.created_at),
                    valid_until: None,
                    source: format!("episode:{}", ep.id),
                    evidence_hash: ep.evidence_hash.clone(),
                    ttl_secs: None,
                    conflicts_with: Vec::new(),
                    pinned: false,
                    forgotten: false,
                    survey_rating: None,
                }),
            };
            existing_keys.insert(key, memory.id.clone());
            report.promoted.push(memory);
            promotions += 1;
        }
    }

    let cutoff = Utc::now() - chrono::Duration::days(settings.forget_after_days);
    for mem in existing {
        if mem.meta.as_ref().is_some_and(|m| m.pinned || m.forgotten) {
            continue;
        }
        let importance = mem.meta.as_ref().map(|m| m.importance).unwrap_or(0.5);
        if mem.updated_at < cutoff && importance < settings.min_importance {
            report.forgotten_ids.push(mem.id.clone());
        }
    }

    report
}

/// Append a dream report line for UI history / undo.
pub fn append_dream_report(base: impl AsRef<Path>, report: &DreamReport) -> std::io::Result<()> {
    let path = dream_log_path(&base);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(report).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::EpisodeEvent;

    #[test]
    fn consolidates_preference_and_skips_secret() {
        let ep = EpisodeRecord {
            id: "e1".into(),
            session_id: "s".into(),
            task_id: "t".into(),
            events: vec![
                EpisodeEvent::UserCorrection {
                    before: "light theme".into(),
                    after: "prefer dark tech theme".into(),
                },
                EpisodeEvent::ToolTrace {
                    tool: "Bash".into(),
                    outcome: "api_key=sk-secret".into(),
                    ok: true,
                },
            ],
            created_at: Utc::now(),
            evidence_hash: String::new(),
        };
        let mut ep = ep;
        ep.recompute_hash();
        let report = consolidate_episodes(&[ep], &[], &DreamEngineSettings::default());
        assert_eq!(report.skipped_secrets, 1);
        assert!(!report.promoted.is_empty());
        assert_eq!(
            report.promoted[0].meta.as_ref().unwrap().kind,
            MemoryKind::Preference
        );
    }
}
