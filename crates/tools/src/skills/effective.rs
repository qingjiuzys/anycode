//! Effective skill allowlist: global ∩ per-agent ∩ project governance.

use std::collections::{HashMap, HashSet};

/// Inputs for computing which skill ids an agent may list and execute.
#[derive(Debug, Clone, Default)]
pub struct SkillsGovernance {
    /// From `config.skills.allowlist` — when set, only these ids (after scan) are candidates.
    pub global_allowlist: Option<Vec<String>>,
    /// From `config.skills.agent_allowlists`.
    pub agent_allowlists: HashMap<String, Vec<String>>,
    /// Enabled skill ids for the current project (`project_skills.enabled = 1`), when DB is available.
    pub project_enabled: Option<HashSet<String>>,
}

impl SkillsGovernance {
    /// Returns allowed skill ids for `agent_type`, or `None` when no governance filters apply.
    #[must_use]
    pub fn effective_ids(&self, agent_type: &str) -> Option<HashSet<String>> {
        let mut sets: Vec<HashSet<String>> = Vec::new();

        if let Some(global) = &self.global_allowlist {
            let s: HashSet<String> = global
                .iter()
                .map(|id| id.trim())
                .filter(|id| !id.is_empty())
                .map(ToString::to_string)
                .collect();
            if !s.is_empty() {
                sets.push(s);
            }
        }

        if let Some(agent_ids) = self.agent_allowlists.get(agent_type) {
            let s: HashSet<String> = agent_ids
                .iter()
                .map(|id| id.trim())
                .filter(|id| !id.is_empty())
                .map(ToString::to_string)
                .collect();
            if !s.is_empty() {
                sets.push(s);
            }
        }

        if let Some(project) = &self.project_enabled {
            if !project.is_empty() {
                sets.push(project.clone());
            }
        }

        if sets.is_empty() {
            return None;
        }

        let mut out = sets[0].clone();
        for s in sets.iter().skip(1) {
            out = out.intersection(s).cloned().collect();
        }
        Some(out)
    }

    #[must_use]
    pub fn is_allowed(&self, agent_type: &str, skill_id: &str) -> bool {
        match self.effective_ids(agent_type) {
            None => true,
            Some(set) => set.contains(skill_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersects_global_agent_and_project() {
        let mut agent_allowlists = HashMap::new();
        agent_allowlists.insert("general-purpose".to_string(), vec!["a".into(), "b".into()]);
        let gov = SkillsGovernance {
            global_allowlist: Some(vec!["a".into(), "b".into(), "c".into()]),
            agent_allowlists,
            project_enabled: Some(["b".into()].into_iter().collect()),
        };
        let eff = gov.effective_ids("general-purpose").unwrap();
        assert_eq!(eff, ["b".into()].into_iter().collect::<HashSet<_>>());
        assert!(gov.is_allowed("general-purpose", "b"));
        assert!(!gov.is_allowed("general-purpose", "a"));
    }
}
