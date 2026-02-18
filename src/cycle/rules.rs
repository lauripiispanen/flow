//! Cycle rules engine
//!
//! Determines which cycles should trigger after a given cycle completes,
//! based on the `after` dependencies and frequency constraints in cycle configuration.

use crate::cycle::config::FlowConfig;
use crate::log::CycleOutcome;

/// Find cycles that should trigger after the given cycle completes.
///
/// A cycle triggers if:
/// 1. Its `after` list contains the completed cycle name
/// 2. Its `min_interval` constraint is satisfied (enough iterations have passed since last run)
///
/// The `log` parameter provides execution history for frequency checking.
/// If `min_interval` is `None`, the cycle always triggers (backward compatible).
/// If `min_interval` is `Some(n)`, at least `n` iterations must have elapsed since
/// this cycle last ran.
///
/// Returns cycle names in config definition order.
#[must_use]
pub fn find_triggered_cycles<'a>(
    config: &'a FlowConfig,
    completed_cycle: &str,
    log: &[CycleOutcome],
) -> Vec<&'a str> {
    config
        .cycles
        .iter()
        .filter(|c| c.after.iter().any(|dep| dep == completed_cycle))
        .filter(|c| {
            let Some(min_interval) = c.min_interval else {
                return true; // No constraint — always trigger
            };
            // Count how many log entries ago this cycle last ran.
            // This is immune to iteration-number resets across runs
            // because it only looks at position in the append-only log.
            log.iter()
                .rev()
                .position(|entry| entry.cycle == c.name)
                .is_none_or(|d| u32::try_from(d).unwrap_or(u32::MAX) >= min_interval)
        })
        .map(|c| c.name.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::make_test_outcome;

    const CONFIG_WITH_DEPS: &str = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding cycle"
prompt = "Code"
after = []

[[cycle]]
name = "gardening"
description = "Gardening cycle"
prompt = "Garden"
after = ["coding"]

[[cycle]]
name = "review"
description = "Review cycle"
prompt = "Review"
after = ["coding"]

[[cycle]]
name = "planning"
description = "Planning cycle"
prompt = "Plan"
after = []
"#;

    fn test_config() -> FlowConfig {
        FlowConfig::parse(CONFIG_WITH_DEPS).unwrap()
    }

    fn make_log_entry(iteration: u32, cycle: &str) -> CycleOutcome {
        make_test_outcome(iteration, cycle, "done")
    }

    #[test]
    fn test_coding_triggers_gardening_and_review() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "coding", &[]);
        assert_eq!(triggered, vec!["gardening", "review"]);
    }

    #[test]
    fn test_no_cycles_triggered_after_gardening() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "gardening", &[]);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_no_cycles_triggered_after_planning() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "planning", &[]);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_unknown_cycle_triggers_nothing() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "nonexistent", &[]);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_chain_dependencies() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
after = []

[[cycle]]
name = "gardening"
description = "Gardening"
prompt = "Garden"
after = ["coding"]

[[cycle]]
name = "review"
description = "Review"
prompt = "Review"
after = ["gardening"]
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // After coding, only gardening triggers (not review)
        let after_coding = find_triggered_cycles(&config, "coding", &[]);
        assert_eq!(after_coding, vec!["gardening"]);

        // After gardening, review triggers
        let after_gardening = find_triggered_cycles(&config, "gardening", &[]);
        assert_eq!(after_gardening, vec!["review"]);
    }

    #[test]
    fn test_multiple_dependencies_all_required() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"

[[cycle]]
name = "testing"
description = "Testing"
prompt = "Test"

[[cycle]]
name = "deploy"
description = "Deploy"
prompt = "Deploy"
after = ["coding", "testing"]
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // Deploy appears in results for both coding and testing
        // (since it lists both in `after`, it triggers after either)
        let after_coding = find_triggered_cycles(&config, "coding", &[]);
        assert_eq!(after_coding, vec!["deploy"]);

        let after_testing = find_triggered_cycles(&config, "testing", &[]);
        assert_eq!(after_testing, vec!["deploy"]);
    }

    #[test]
    fn test_empty_cycles_triggers_nothing() {
        use crate::cycle::config::GlobalConfig;
        let config = FlowConfig {
            global: GlobalConfig {
                permissions: vec![],
                max_permission_denials: 10,
                circuit_breaker_repeated: 5,
                max_consecutive_failures: 3,
                summary_interval: 5,
                vars: std::collections::HashMap::new(),
            },
            selector: None,
            cycles: vec![],
        };
        let triggered = find_triggered_cycles(&config, "anything", &[]);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_cycle_does_not_trigger_itself() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
after = []
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let triggered = find_triggered_cycles(&config, "coding", &[]);
        assert!(triggered.is_empty());
    }

    // --- Frequency constraint tests ---
    //
    // min_interval is measured as the number of log entries between the
    // most recent occurrence of the cycle and the end of the log
    // (i.e. distance-from-end).  This is immune to iteration-number
    // resets across runs.

    fn gardening_after_coding_config(min_interval: Option<u32>) -> FlowConfig {
        let interval_line = min_interval.map_or(String::new(), |n| format!("min_interval = {n}"));
        FlowConfig::parse(&format!(
            r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"

[[cycle]]
name = "gardening"
description = "Gardening"
prompt = "Garden"
after = ["coding"]
{interval_line}
"#
        ))
        .unwrap()
    }

    #[test]
    fn test_min_interval_blocks_when_too_recent() {
        let config = gardening_after_coding_config(Some(3));
        // Gardening is 0 entries from the end → distance 0 < 3
        let log = vec![make_log_entry(1, "coding"), make_log_entry(2, "gardening")];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_min_interval_allows_when_enough_elapsed() {
        let config = gardening_after_coding_config(Some(3));
        // Gardening is 3 entries from the end → distance 3 >= 3
        let log = vec![
            make_log_entry(1, "gardening"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
            make_log_entry(4, "coding"),
        ];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_allows_when_never_ran() {
        let config = gardening_after_coding_config(Some(5));
        // Gardening never ran — always triggers
        let log = vec![make_log_entry(1, "coding")];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_no_min_interval_always_triggers() {
        let config = gardening_after_coding_config(None);
        // No constraint — triggers even if gardening is the most recent entry
        let log = vec![make_log_entry(1, "coding"), make_log_entry(2, "gardening")];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_zero_always_triggers() {
        let config = gardening_after_coding_config(Some(0));
        // distance 0 >= 0
        let log = vec![make_log_entry(1, "coding"), make_log_entry(2, "gardening")];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_boundary_exact_match() {
        let config = gardening_after_coding_config(Some(2));
        // Gardening is 2 entries from the end → distance 2 >= 2
        let log = vec![
            make_log_entry(1, "gardening"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
        ];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_boundary_one_short() {
        let config = gardening_after_coding_config(Some(3));
        // Gardening is 2 entries from the end → distance 2 < 3
        let log = vec![
            make_log_entry(1, "gardening"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
        ];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_min_interval_cross_run_not_fooled_by_old_entries() {
        // Regression: the log accumulates entries across runs.
        // min_interval must measure distance from the END of the log,
        // not compare iteration numbers (which reset per run).
        let config = gardening_after_coding_config(Some(5));

        // Previous run: 10 entries. New run: gardening at position -2, coding at -1.
        // Distance from end to last gardening = 1 < 5 → blocked.
        let log = vec![
            // --- previous run ---
            make_log_entry(1, "coding"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
            make_log_entry(4, "coding"),
            make_log_entry(5, "coding"),
            make_log_entry(6, "coding"),
            make_log_entry(7, "coding"),
            make_log_entry(8, "gardening"),
            make_log_entry(9, "coding"),
            make_log_entry(10, "coding"),
            // --- new run (iterations reset) ---
            make_log_entry(1, "coding"),
            make_log_entry(2, "gardening"),
            make_log_entry(3, "coding"),
        ];

        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert!(
            triggered.is_empty(),
            "gardening should be blocked: only 1 entry since last run, need 5"
        );
    }

    #[test]
    fn test_min_interval_cross_run_allows_after_enough_entries() {
        // Same cross-run scenario but enough entries have passed.
        let config = gardening_after_coding_config(Some(3));

        let log = vec![
            // --- previous run ---
            make_log_entry(1, "coding"),
            make_log_entry(2, "gardening"),
            make_log_entry(3, "coding"),
            // --- new run ---
            make_log_entry(1, "coding"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
        ];

        // Gardening is 4 entries from the end → distance 4 >= 3 → triggers
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_only_considers_most_recent_occurrence() {
        let config = gardening_after_coding_config(Some(3));

        // Gardening ran twice: at positions 0 and 3 (from end: 5 and 2).
        // Most recent is at distance 2 < 3 → blocked.
        let log = vec![
            make_log_entry(1, "gardening"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
            make_log_entry(4, "gardening"),
            make_log_entry(5, "coding"),
            make_log_entry(6, "coding"),
        ];

        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_empty_log_triggers_when_no_min_interval() {
        let config = gardening_after_coding_config(None);
        let triggered = find_triggered_cycles(&config, "coding", &[]);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_empty_log_triggers_with_min_interval() {
        let config = gardening_after_coding_config(Some(10));
        // Never ran → always triggers
        let triggered = find_triggered_cycles(&config, "coding", &[]);
        assert_eq!(triggered, vec!["gardening"]);
    }
}
