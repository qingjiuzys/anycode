//! Project-level trust score derived from delivery readiness signals.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectTrustInputs {
    pub sessions_total: i64,
    pub gates_total: i64,
    pub artifacts_total: i64,
    pub blocked_sessions: i64,
    pub failed_required_gates: i64,
    pub unverified_artifacts: i64,
    pub stale_running_sessions: i64,
}

impl ProjectTrustInputs {
    #[must_use]
    pub fn from_row_counts(
        sessions_count: i64,
        artifacts_count: i64,
        gates_total: i64,
        blocked_sessions: i64,
        failed_required_gates: i64,
        unverified_artifacts: i64,
        stale_running_sessions: i64,
    ) -> Self {
        Self {
            sessions_total: sessions_count,
            gates_total,
            artifacts_total: artifacts_count,
            blocked_sessions,
            failed_required_gates,
            unverified_artifacts,
            stale_running_sessions,
        }
    }
}

#[must_use]
pub fn has_trust_signal(i: &ProjectTrustInputs) -> bool {
    i.sessions_total > 0 || i.gates_total > 0 || i.artifacts_total > 0
}

#[must_use]
pub fn readiness_score(blocked: i64, failed_gates: i64, unverified: i64, stale: i64) -> i64 {
    let mut score = 100i64;
    score -= blocked * 20;
    score -= failed_gates * 15;
    score -= (unverified.min(10)) * 2;
    score -= stale * 10;
    score.clamp(0, 100)
}

#[must_use]
pub fn readiness_from_inputs(i: &ProjectTrustInputs) -> i64 {
    readiness_score(
        i.blocked_sessions,
        i.failed_required_gates,
        i.unverified_artifacts,
        i.stale_running_sessions,
    )
}

/// Delivery readiness as 0.0–1.0; `None` when there is no scorable activity.
#[must_use]
pub fn compute_trust_score(i: &ProjectTrustInputs) -> Option<f64> {
    if !has_trust_signal(i) {
        return None;
    }
    Some(readiness_from_inputs(i) as f64 / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_signal_returns_none() {
        let i = ProjectTrustInputs::default();
        assert!(!has_trust_signal(&i));
        assert_eq!(compute_trust_score(&i), None);
    }

    #[test]
    fn sessions_only_scores_full_when_clean() {
        let i = ProjectTrustInputs {
            sessions_total: 3,
            ..Default::default()
        };
        assert_eq!(compute_trust_score(&i), Some(1.0));
    }

    #[test]
    fn blocked_and_failed_gates_lower_score() {
        let i = ProjectTrustInputs {
            sessions_total: 2,
            gates_total: 1,
            blocked_sessions: 1,
            failed_required_gates: 1,
            ..Default::default()
        };
        let score = compute_trust_score(&i).unwrap();
        assert!(score < 0.8, "score was {score}");
        assert_eq!(readiness_from_inputs(&i), 65);
    }
}
