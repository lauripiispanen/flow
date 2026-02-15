//! Permission resolver for Claude Code `--allowedTools` flags
//!
//! Merges global and per-cycle permissions using an additive model:
//! the resolved set is the union of global + cycle-specific permissions.

use std::collections::HashSet;

use crate::cycle::config::{CycleConfig, GlobalConfig};

/// Resolve the effective permissions for a cycle by merging global and
/// cycle-specific permissions. Returns a deduplicated list with global
/// permissions first, followed by any cycle-specific additions.
#[must_use]
pub fn resolve_permissions(global: &GlobalConfig, cycle: &CycleConfig) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for perm in &global.permissions {
        if seen.insert(perm.as_str()) {
            result.push(perm.clone());
        }
    }

    for perm in &cycle.permissions {
        if seen.insert(perm.as_str()) {
            result.push(perm.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::config::FlowConfig;

    #[test]
    fn test_resolve_merges_global_and_cycle_permissions() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read", "Edit(./src/**)"]

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
permissions = ["Edit(./tests/**)", "Bash(cargo test *)"]
"#,
        )
        .unwrap();

        let cycle = config.get_cycle("coding").unwrap();
        let resolved = resolve_permissions(&config.global, cycle);

        assert_eq!(
            resolved,
            vec![
                "Read",
                "Edit(./src/**)",
                "Edit(./tests/**)",
                "Bash(cargo test *)",
            ]
        );
    }

    #[test]
    fn test_resolve_deduplicates_permissions() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read", "Bash(cargo *)"]

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
permissions = ["Read", "Bash(cargo *)"]
"#,
        )
        .unwrap();

        let cycle = config.get_cycle("coding").unwrap();
        let resolved = resolve_permissions(&config.global, cycle);

        assert_eq!(resolved, vec!["Read", "Bash(cargo *)"]);
    }

    #[test]
    fn test_resolve_empty_cycle_permissions_returns_global() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "review"
description = "Review"
prompt = "Review"
"#,
        )
        .unwrap();

        let cycle = config.get_cycle("review").unwrap();
        let resolved = resolve_permissions(&config.global, cycle);

        assert_eq!(resolved, vec!["Read"]);
    }

    #[test]
    fn test_resolve_empty_global_permissions_returns_cycle() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
permissions = ["Read", "Edit(./src/**)"]
"#,
        )
        .unwrap();

        let cycle = config.get_cycle("coding").unwrap();
        let resolved = resolve_permissions(&config.global, cycle);

        assert_eq!(resolved, vec!["Read", "Edit(./src/**)"]);
    }

    #[test]
    fn test_resolve_both_empty_returns_empty() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = []

[[cycle]]
name = "review"
description = "Review"
prompt = "Review"
"#,
        )
        .unwrap();

        let cycle = config.get_cycle("review").unwrap();
        let resolved = resolve_permissions(&config.global, cycle);

        assert!(resolved.is_empty());
    }

    #[test]
    fn test_resolve_preserves_order_global_then_cycle() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
permissions = ["Edit(./src/**)", "Bash(cargo *)"]
"#,
        )
        .unwrap();

        let cycle = config.get_cycle("coding").unwrap();
        let resolved = resolve_permissions(&config.global, cycle);

        assert_eq!(resolved[0], "Read");
        assert_eq!(resolved[1], "Edit(./src/**)");
        assert_eq!(resolved[2], "Bash(cargo *)");
    }

    #[test]
    fn test_resolve_deduplicates_preserving_first_occurrence() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read", "Bash(cargo *)"]

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
permissions = ["Bash(cargo *)", "Edit(./src/**)"]
"#,
        )
        .unwrap();

        let cycle = config.get_cycle("coding").unwrap();
        let resolved = resolve_permissions(&config.global, cycle);

        // "Bash(cargo *)" appears in global first, so cycle duplicate is dropped
        assert_eq!(resolved, vec!["Read", "Bash(cargo *)", "Edit(./src/**)"]);
    }
}
