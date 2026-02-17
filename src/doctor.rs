//! Flow doctor — diagnostics and health checks
//!
//! Analyzes `.flow/log.jsonl` and `cycles.toml` to diagnose issues
//! and suggest fixes. Returns a structured report with categories:
//! errors (must fix), warnings (should fix), info (suggestions).

use crate::cycle::config::FlowConfig;
use crate::log::CycleOutcome;

/// Severity level for a diagnostic finding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    /// Must fix — something is broken
    Error,
    /// Should fix — suboptimal configuration
    Warning,
    /// Suggestion — informational improvement
    Info,
}

/// A single diagnostic finding
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Severity of the finding
    pub severity: Severity,
    /// Short code for the finding (e.g., "D001")
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Suggested fix (optional)
    pub suggestion: Option<String>,
}

/// Diagnostic report from `flow doctor`
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    /// All findings, in order of severity (errors first)
    pub findings: Vec<Finding>,
}

impl DiagnosticReport {
    /// Returns true if the report has no findings at all
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Returns the number of errors
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    /// Returns the number of warnings
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }

    /// Returns the number of info items
    #[must_use]
    pub fn info_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count()
    }
}

/// Run all diagnostic checks and return a report.
#[must_use]
pub fn diagnose(config: &FlowConfig, log: &[CycleOutcome]) -> DiagnosticReport {
    let mut findings = Vec::new();

    check_permission_denials(log, &mut findings);
    check_cycle_health(log, &mut findings);
    check_config_lint(config, &mut findings);
    check_frequency_tuning(config, log, &mut findings);

    // Sort: errors first, then warnings, then info
    findings.sort_by_key(|f| match f.severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    });

    DiagnosticReport { findings }
}

/// D001: Check for permission denials in recent log entries
fn check_permission_denials(log: &[CycleOutcome], findings: &mut Vec<Finding>) {
    for entry in log {
        if let Some(ref denials) = entry.permission_denials {
            if !denials.is_empty() {
                // Deduplicate denied tool names for suggestion
                let mut unique_tools: Vec<&str> = denials.iter().map(String::as_str).collect();
                unique_tools.sort_unstable();
                unique_tools.dedup();

                let suggestions: Vec<String> = unique_tools
                    .iter()
                    .map(|tool| crate::claude::stream::suggest_permission_fix(tool))
                    .collect();

                findings.push(Finding {
                    severity: Severity::Error,
                    code: "D001".to_string(),
                    message: format!(
                        "Cycle '{}' had {} permission denial(s) in iteration {}: {}",
                        entry.cycle,
                        denials.len(),
                        entry.iteration,
                        unique_tools.join(", ")
                    ),
                    suggestion: Some(format!(
                        "Add to cycles.toml permissions: {}",
                        suggestions.join(", ")
                    )),
                });
            }
        }
    }
}

/// D002: Check for cycles that consistently fail
fn check_cycle_health(log: &[CycleOutcome], findings: &mut Vec<Finding>) {
    if log.is_empty() {
        return;
    }

    // Group outcomes by cycle name
    let mut cycle_outcomes: std::collections::HashMap<&str, Vec<&CycleOutcome>> =
        std::collections::HashMap::new();
    for entry in log {
        cycle_outcomes
            .entry(entry.cycle.as_str())
            .or_default()
            .push(entry);
    }

    for (cycle_name, outcomes) in &cycle_outcomes {
        let failure_count = outcomes
            .iter()
            .filter(|o| o.outcome.starts_with("Failed"))
            .count();
        let total = outcomes.len();

        // If more than half of runs failed, flag it
        if total >= 2 && failure_count * 2 > total {
            findings.push(Finding {
                severity: Severity::Warning,
                code: "D002".to_string(),
                message: format!(
                    "Cycle '{cycle_name}' failed {failure_count}/{total} times"
                ),
                suggestion: Some("Check cycle prompt and permissions. Run `flow --cycle <name>` manually to debug.".to_string()),
            });
        }

        // Check for high cost anomalies (> $5 per cycle run)
        let high_cost_runs: Vec<_> = outcomes
            .iter()
            .filter_map(|o| o.total_cost_usd.filter(|&c| c > 5.0))
            .collect();
        if !high_cost_runs.is_empty() {
            let max_cost = high_cost_runs
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            #[allow(clippy::cast_precision_loss)]
            let avg_cost: f64 = high_cost_runs.iter().sum::<f64>() / high_cost_runs.len() as f64;
            findings.push(Finding {
                severity: Severity::Warning,
                code: "D003".to_string(),
                message: format!(
                    "Cycle '{cycle_name}' had {} run(s) exceeding $5.00 (max ${max_cost:.2}, avg ${avg_cost:.2})",
                    high_cost_runs.len()
                ),
                suggestion: Some(
                    "Consider breaking the task into smaller subtasks or adding constraints to the prompt."
                        .to_string(),
                ),
            });
        }
    }
}

/// D004: Lint the config for common issues
fn check_config_lint(config: &FlowConfig, findings: &mut Vec<Finding>) {
    for cycle in &config.cycles {
        // Warn about triggered cycles without min_interval
        if !cycle.after.is_empty() && cycle.min_interval.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                code: "D004".to_string(),
                message: format!(
                    "Cycle '{}' triggers after {:?} but has no min_interval",
                    cycle.name, cycle.after
                ),
                suggestion: Some(format!(
                    "Add `min_interval = 3` to '{}' in cycles.toml to avoid redundant runs",
                    cycle.name
                )),
            });
        }

        // Warn about cycles with empty permissions
        if cycle.permissions.is_empty() && config.global.permissions.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                code: "D005".to_string(),
                message: format!(
                    "Cycle '{}' has no permissions (global or cycle-level)",
                    cycle.name
                ),
                suggestion: Some(
                    "Add at least `Read` to global permissions in cycles.toml".to_string(),
                ),
            });
        }
    }
}

/// D006: Suggest frequency tuning based on actual run patterns
fn check_frequency_tuning(config: &FlowConfig, log: &[CycleOutcome], findings: &mut Vec<Finding>) {
    if log.is_empty() {
        return;
    }

    // For triggered cycles, check if they run too frequently
    for cycle in &config.cycles {
        if cycle.after.is_empty() {
            continue;
        }

        let runs: Vec<&CycleOutcome> = log.iter().filter(|e| e.cycle == cycle.name).collect();
        if runs.len() < 2 {
            continue;
        }

        // Check if consecutive runs are too close together
        let mut close_runs = 0;
        for pair in runs.windows(2) {
            let gap = pair[1].iteration.saturating_sub(pair[0].iteration);
            if gap <= 1 {
                close_runs += 1;
            }
        }

        if close_runs > 0 && cycle.min_interval.is_none_or(|v| v <= 1) {
            findings.push(Finding {
                severity: Severity::Info,
                code: "D006".to_string(),
                message: format!(
                    "Cycle '{}' ran {} consecutive time(s) with <=1 iteration gap",
                    cycle.name, close_runs
                ),
                suggestion: Some(format!(
                    "Consider setting `min_interval = 3` for '{}' to space out runs",
                    cycle.name
                )),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::make_test_outcome as make_outcome;

    fn basic_config() -> FlowConfig {
        FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

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
"#,
        )
        .unwrap()
    }

    // --- DiagnosticReport tests ---

    #[test]
    fn test_clean_report_with_no_issues() {
        let config = basic_config();
        let report = diagnose(&config, &[]);
        assert!(report.is_clean());
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
    }

    #[test]
    fn test_report_counts() {
        let report = DiagnosticReport {
            findings: vec![
                Finding {
                    severity: Severity::Error,
                    code: "E1".to_string(),
                    message: "error".to_string(),
                    suggestion: None,
                },
                Finding {
                    severity: Severity::Warning,
                    code: "W1".to_string(),
                    message: "warning".to_string(),
                    suggestion: None,
                },
                Finding {
                    severity: Severity::Info,
                    code: "I1".to_string(),
                    message: "info".to_string(),
                    suggestion: None,
                },
            ],
        };

        assert_eq!(report.error_count(), 1);
        assert_eq!(report.warning_count(), 1);
        assert_eq!(report.info_count(), 1);
        assert!(!report.is_clean());
    }

    // --- D001: Permission denial detection ---

    #[test]
    fn test_d001_detects_permission_denials() {
        let config = basic_config();
        let mut entry = make_outcome(1, "coding", "done");
        entry.permission_denials = Some(vec!["Edit".to_string(), "Bash".to_string()]);

        let report = diagnose(&config, &[entry]);
        assert_eq!(report.error_count(), 1);

        let finding = &report.findings[0];
        assert_eq!(finding.code, "D001");
        assert_eq!(finding.severity, Severity::Error);
        assert!(finding.message.contains("coding"));
        assert!(finding.message.contains("2 permission denial"));
        assert!(finding.suggestion.is_some());
    }

    #[test]
    fn test_d001_no_findings_when_no_denials() {
        let config = basic_config();
        let entry = make_outcome(1, "coding", "done");

        let report = diagnose(&config, &[entry]);
        assert!(
            !report.findings.iter().any(|f| f.code == "D001"),
            "Should have no D001 findings when there are no denials"
        );
    }

    #[test]
    fn test_d001_deduplicates_tool_names_in_suggestion() {
        let config = basic_config();
        let mut entry = make_outcome(1, "coding", "done");
        entry.permission_denials = Some(vec![
            "Edit".to_string(),
            "Edit".to_string(),
            "Edit".to_string(),
        ]);

        let report = diagnose(&config, &[entry]);
        let finding = report.findings.iter().find(|f| f.code == "D001").unwrap();
        // Should mention 3 denials but suggest fix for Edit only once
        assert!(finding.message.contains("3 permission denial"));
        let suggestion = finding.suggestion.as_ref().unwrap();
        // Only one permission fix suggestion (not three repeated ones)
        assert_eq!(
            suggestion.matches("Edit(./**)").count(),
            1,
            "Should deduplicate tool suggestions: {suggestion}"
        );
    }

    // --- D002: Cycle health ---

    #[test]
    fn test_d002_detects_frequent_failures() {
        let config = basic_config();
        let log = vec![
            make_outcome(1, "coding", "Failed with exit code 1"),
            make_outcome(2, "coding", "Failed with exit code 1"),
            make_outcome(3, "coding", "Completed successfully"),
        ];

        let report = diagnose(&config, &log);
        let d002 = report.findings.iter().find(|f| f.code == "D002");
        assert!(d002.is_some(), "Should detect frequent failures");
        assert!(d002.unwrap().message.contains("2/3"));
    }

    #[test]
    fn test_d002_no_warning_for_single_failure() {
        let config = basic_config();
        let log = vec![make_outcome(1, "coding", "Failed with exit code 1")];

        let report = diagnose(&config, &log);
        let d002 = report.findings.iter().find(|f| f.code == "D002");
        assert!(
            d002.is_none(),
            "Should not warn with only 1 run (needs >= 2)"
        );
    }

    #[test]
    fn test_d002_no_warning_when_mostly_successful() {
        let config = basic_config();
        let log = vec![
            make_outcome(1, "coding", "Completed successfully"),
            make_outcome(2, "coding", "Completed successfully"),
            make_outcome(3, "coding", "Failed with exit code 1"),
        ];

        let report = diagnose(&config, &log);
        let d002 = report.findings.iter().find(|f| f.code == "D002");
        assert!(d002.is_none(), "Should not warn when mostly successful");
    }

    // --- D003: Cost anomalies ---

    #[test]
    fn test_d003_detects_high_cost() {
        let config = basic_config();
        let mut entry = make_outcome(1, "coding", "done");
        entry.total_cost_usd = Some(7.50);

        let report = diagnose(&config, &[entry]);
        let d003 = report.findings.iter().find(|f| f.code == "D003");
        assert!(d003.is_some(), "Should detect high cost");
        assert!(d003.unwrap().message.contains("1 run(s)"));
        assert!(d003.unwrap().message.contains("$7.50"));
    }

    #[test]
    fn test_d003_no_warning_at_exactly_five_dollars() {
        let config = basic_config();
        let mut entry = make_outcome(1, "coding", "done");
        entry.total_cost_usd = Some(5.0);

        let report = diagnose(&config, &[entry]);
        let d003 = report.findings.iter().find(|f| f.code == "D003");
        assert!(
            d003.is_none(),
            "Should not warn at exactly $5.00 (threshold is >$5)"
        );
    }

    #[test]
    fn test_d003_aggregates_multiple_high_cost_runs() {
        let config = basic_config();
        let mut entry1 = make_outcome(1, "coding", "done");
        entry1.total_cost_usd = Some(6.00);
        let mut entry2 = make_outcome(2, "coding", "done");
        entry2.total_cost_usd = Some(8.00);
        let mut entry3 = make_outcome(3, "coding", "done");
        entry3.total_cost_usd = Some(3.00); // normal cost, should not be counted

        let report = diagnose(&config, &[entry1, entry2, entry3]);
        let d003_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.code == "D003")
            .collect();
        assert_eq!(
            d003_findings.len(),
            1,
            "Should produce one aggregated finding, not one per run"
        );
        assert!(d003_findings[0].message.contains("2 run(s)"));
        assert!(d003_findings[0].message.contains("$8.00")); // max
    }

    #[test]
    fn test_d003_no_warning_for_normal_cost() {
        let config = basic_config();
        let mut entry = make_outcome(1, "coding", "done");
        entry.total_cost_usd = Some(2.50);

        let report = diagnose(&config, &[entry]);
        let d003 = report.findings.iter().find(|f| f.code == "D003");
        assert!(d003.is_none(), "Should not warn for normal cost");
    }

    // --- D004: Config lint ---

    #[test]
    fn test_d004_warns_triggered_cycle_without_min_interval() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"

[[cycle]]
name = "gardening"
description = "Gardening"
prompt = "Garden"
after = ["coding"]
"#,
        )
        .unwrap();

        let report = diagnose(&config, &[]);
        let d004 = report.findings.iter().find(|f| f.code == "D004");
        assert!(d004.is_some(), "Should warn about missing min_interval");
        assert!(d004.unwrap().message.contains("gardening"));
    }

    #[test]
    fn test_d004_no_warning_when_min_interval_set() {
        let config = basic_config(); // gardening has min_interval = 3
        let report = diagnose(&config, &[]);
        let d004 = report.findings.iter().find(|f| f.code == "D004");
        assert!(d004.is_none());
    }

    // --- D005: No permissions ---

    #[test]
    fn test_d005_warns_cycle_with_no_permissions() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#,
        )
        .unwrap();

        let report = diagnose(&config, &[]);
        let d005 = report.findings.iter().find(|f| f.code == "D005");
        assert!(d005.is_some(), "Should warn about no permissions");
    }

    // --- D006: Frequency tuning ---

    #[test]
    fn test_d006_suggests_frequency_tuning_for_close_runs() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"

[[cycle]]
name = "gardening"
description = "Gardening"
prompt = "Garden"
after = ["coding"]
"#,
        )
        .unwrap();

        let log = vec![
            make_outcome(1, "coding", "done"),
            make_outcome(2, "gardening", "done"),
            make_outcome(3, "gardening", "done"),
        ];

        let report = diagnose(&config, &log);
        let d006 = report.findings.iter().find(|f| f.code == "D006");
        assert!(
            d006.is_some(),
            "Should suggest frequency tuning when runs are <= 1 iteration apart"
        );
    }

    #[test]
    fn test_d006_skips_when_min_interval_already_set() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

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
"#,
        )
        .unwrap();

        let log = vec![
            make_outcome(1, "coding", "done"),
            make_outcome(2, "gardening", "done"),
            make_outcome(3, "gardening", "done"),
        ];

        let report = diagnose(&config, &log);
        let d006 = report.findings.iter().find(|f| f.code == "D006");
        assert!(
            d006.is_none(),
            "Should not suggest frequency tuning when min_interval > 1 is already set"
        );
    }

    // --- Ordering ---

    #[test]
    fn test_findings_ordered_by_severity() {
        let config = FlowConfig::parse(
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
"#,
        )
        .unwrap();

        let mut entry = make_outcome(1, "coding", "done");
        entry.permission_denials = Some(vec!["Edit".to_string()]);

        let report = diagnose(&config, &[entry]);

        // Errors should come before warnings/info
        if report.findings.len() >= 2 {
            let severities: Vec<_> = report.findings.iter().map(|f| &f.severity).collect();
            for pair in severities.windows(2) {
                let order_a = match pair[0] {
                    Severity::Error => 0,
                    Severity::Warning => 1,
                    Severity::Info => 2,
                };
                let order_b = match pair[1] {
                    Severity::Error => 0,
                    Severity::Warning => 1,
                    Severity::Info => 2,
                };
                assert!(order_a <= order_b, "Findings should be ordered by severity");
            }
        }
    }
}
