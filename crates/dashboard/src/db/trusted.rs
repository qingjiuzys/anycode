//! Trusted completion rules for sessions.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustedStatus {
    Unverified,
    Verified,
    Blocked,
}

impl TrustedStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Verified => "verified",
            Self::Blocked => "blocked",
        }
    }
}

/// A session is trusted-complete when required gates pass, or when there are no gates and the
/// session finished successfully (`run` / `repl` without goal gates).
#[must_use]
pub fn compute_trusted_status(
    required_total: i64,
    required_failed: i64,
    required_pending: i64,
    session_status: Option<&str>,
) -> TrustedStatus {
    if required_total == 0 {
        return match session_status {
            Some("completed") => TrustedStatus::Verified,
            Some("failed") => TrustedStatus::Blocked,
            _ => TrustedStatus::Unverified,
        };
    }
    if required_failed > 0 {
        return TrustedStatus::Blocked;
    }
    if required_pending > 0 {
        return TrustedStatus::Unverified;
    }
    TrustedStatus::Verified
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gates_running_is_unverified() {
        assert_eq!(
            compute_trusted_status(0, 0, 0, Some("running")),
            TrustedStatus::Unverified
        );
    }

    #[test]
    fn no_gates_completed_is_verified() {
        assert_eq!(
            compute_trusted_status(0, 0, 0, Some("completed")),
            TrustedStatus::Verified
        );
    }

    #[test]
    fn no_gates_failed_is_blocked() {
        assert_eq!(
            compute_trusted_status(0, 0, 0, Some("failed")),
            TrustedStatus::Blocked
        );
    }

    #[test]
    fn failed_gate_blocks() {
        assert_eq!(
            compute_trusted_status(2, 1, 0, Some("completed")),
            TrustedStatus::Blocked
        );
    }

    #[test]
    fn all_passed_is_verified() {
        assert_eq!(
            compute_trusted_status(3, 0, 0, Some("completed")),
            TrustedStatus::Verified
        );
    }

    #[test]
    fn pending_is_unverified() {
        assert_eq!(
            compute_trusted_status(2, 0, 1, Some("running")),
            TrustedStatus::Unverified
        );
    }
}
