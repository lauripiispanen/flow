//! Shared test utilities
//!
//! Common helpers used across test modules. Only compiled in test builds.

use crate::log::jsonl::CycleOutcome;
use chrono::Utc;

/// Create a minimal `CycleOutcome` for testing with sensible defaults.
///
/// Sets `duration_secs = 60` and leaves all optional fields as `None`.
#[must_use]
pub fn make_test_outcome(iteration: u32, cycle: &str, outcome: &str) -> CycleOutcome {
    CycleOutcome {
        iteration,
        cycle: cycle.to_string(),
        timestamp: Utc::now(),
        outcome: outcome.to_string(),
        files_changed: vec![],
        tests_passed: 0,
        duration_secs: 60,
        num_turns: None,
        total_cost_usd: None,
        permission_denial_count: None,
        permission_denials: None,
        steps: None,
    }
}
