//! Cycle rules engine
//!
//! Determines which cycles should trigger after a given cycle completes,
//! based on the `after` dependencies in cycle configuration.

use crate::cycle::config::FlowConfig;

/// Find cycles that should trigger after the given cycle completes.
///
/// A cycle triggers if its `after` list contains the completed cycle name.
/// Returns cycle names in config definition order.
#[must_use]
pub fn find_triggered_cycles<'a>(config: &'a FlowConfig, completed_cycle: &str) -> Vec<&'a str> {
    config
        .cycles
        .iter()
        .filter(|c| c.after.iter().any(|dep| dep == completed_cycle))
        .map(|c| c.name.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_coding_triggers_gardening_and_review() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "coding");
        assert_eq!(triggered, vec!["gardening", "review"]);
    }

    #[test]
    fn test_no_cycles_triggered_after_gardening() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "gardening");
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_no_cycles_triggered_after_planning() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "planning");
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_unknown_cycle_triggers_nothing() {
        let config = test_config();
        let triggered = find_triggered_cycles(&config, "nonexistent");
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
        let after_coding = find_triggered_cycles(&config, "coding");
        assert_eq!(after_coding, vec!["gardening"]);

        // After gardening, review triggers
        let after_gardening = find_triggered_cycles(&config, "gardening");
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
        let after_coding = find_triggered_cycles(&config, "coding");
        assert_eq!(after_coding, vec!["deploy"]);

        let after_testing = find_triggered_cycles(&config, "testing");
        assert_eq!(after_testing, vec!["deploy"]);
    }

    #[test]
    fn test_empty_cycles_triggers_nothing() {
        use crate::cycle::config::GlobalConfig;
        let config = FlowConfig {
            global: GlobalConfig {
                permissions: vec![],
            },
            cycles: vec![],
        };
        let triggered = find_triggered_cycles(&config, "anything");
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
        let triggered = find_triggered_cycles(&config, "coding");
        assert!(triggered.is_empty());
    }
}
