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
    let current_iteration = u32::try_from(log.len()).unwrap_or(u32::MAX);

    config
        .cycles
        .iter()
        .filter(|c| c.after.iter().any(|dep| dep == completed_cycle))
        .filter(|c| {
            let Some(min_interval) = c.min_interval else {
                return true; // No constraint — always trigger
            };
            // Find the most recent log entry for this cycle
            log.iter()
                .rev()
                .find(|entry| entry.cycle == c.name)
                .map(|entry| entry.iteration)
                .is_none_or(|last| current_iteration.saturating_sub(last) >= min_interval)
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

    #[test]
    fn test_min_interval_blocks_trigger_when_too_recent() {
        let toml = r#"
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
min_interval = 3
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // Log: coding ran at iteration 1, gardening ran at iteration 2
        let log = vec![make_log_entry(1, "coding"), make_log_entry(2, "gardening")];

        // Only 0 iterations since gardening last ran (log.len()=2, last=2, 2-2=0 < 3)
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert!(
            triggered.is_empty(),
            "gardening should be blocked by min_interval"
        );
    }

    #[test]
    fn test_min_interval_allows_trigger_when_enough_elapsed() {
        let toml = r#"
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
min_interval = 3
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // Log: gardening ran at iteration 1, then 3 more coding iterations (2,3,4)
        let log = vec![
            make_log_entry(1, "gardening"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
            make_log_entry(4, "coding"),
        ];

        // 4 iterations total, gardening last at 1, 4-1=3 >= 3
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_allows_trigger_when_never_ran() {
        let toml = r#"
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
min_interval = 5
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // Gardening never ran — should trigger regardless of min_interval
        let log = vec![make_log_entry(1, "coding")];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_no_min_interval_always_triggers() {
        let toml = r#"
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
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // Gardening just ran, but has no min_interval — should still trigger
        let log = vec![make_log_entry(1, "coding"), make_log_entry(2, "gardening")];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_zero_always_triggers() {
        let toml = r#"
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
min_interval = 0
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // min_interval=0 means always eligible
        let log = vec![make_log_entry(1, "coding"), make_log_entry(2, "gardening")];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }

    #[test]
    fn test_min_interval_boundary_exact_match() {
        let toml = r#"
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
min_interval = 2
"#;
        let config = FlowConfig::parse(toml).unwrap();

        // Gardening ran at iteration 1, now at iteration 3 (log.len()=3)
        // 3 - 1 = 2, which equals min_interval — should trigger
        let log = vec![
            make_log_entry(1, "gardening"),
            make_log_entry(2, "coding"),
            make_log_entry(3, "coding"),
        ];
        let triggered = find_triggered_cycles(&config, "coding", &log);
        assert_eq!(triggered, vec!["gardening"]);
    }
}
