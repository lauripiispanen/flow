//! Cycle selector — AI-driven cycle selection for multi-iteration runs
//!
//! Summarizes JSONL log history and TODO.md state to build a prompt
//! for Claude Code, which returns the next cycle to execute.

use std::collections::HashMap;

use anyhow::{Context, Result};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command as TokioCommand;

use crate::claude::cli::build_command;
use crate::claude::stream::{parse_event, StreamAccumulator, StreamEvent};
use crate::cycle::config::FlowConfig;
use crate::log::CycleOutcome;

/// A pending task extracted from TODO.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoTask {
    /// Priority level (P0, P1, P2, P3)
    pub priority: String,
    /// Task description
    pub description: String,
}

/// Parse TODO.md content and extract pending (unchecked) tasks with priorities.
///
/// Looks for lines matching `- [ ] <description>` followed by a line containing
/// `Priority: P<n>`. Only returns unchecked tasks.
#[must_use]
pub fn parse_todo_tasks(content: &str) -> Vec<TodoTask> {
    let lines: Vec<&str> = content.lines().collect();
    let mut tasks = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match unchecked task lines: "- [ ] <description>"
        if let Some(desc) = trimmed.strip_prefix("- [ ] ") {
            let description = desc.trim().to_string();
            if description.is_empty() {
                continue;
            }

            // Look at subsequent lines for "Priority: P<n>"
            let mut priority = None;
            for lookahead in lines.iter().skip(i + 1).take(5) {
                let la = lookahead.trim();
                if la.starts_with("- [") {
                    // Hit the next task, stop looking
                    break;
                }
                if let Some(rest) = la.strip_prefix("- Priority:") {
                    priority = Some(rest.trim().to_string());
                    break;
                }
                // Also match standalone "Priority: P0" lines (indented metadata)
                if let Some(rest) = la.strip_prefix("Priority:") {
                    priority = Some(rest.trim().to_string());
                    break;
                }
            }

            if let Some(priority) = priority {
                tasks.push(TodoTask {
                    priority,
                    description,
                });
            }
        }
    }

    tasks
}

/// Format parsed TODO tasks as a compact string for the selector prompt.
#[must_use]
pub fn format_todo_summary(tasks: &[TodoTask]) -> String {
    if tasks.is_empty() {
        return "No pending tasks found in TODO.md".to_string();
    }

    let mut by_priority: HashMap<&str, Vec<&str>> = HashMap::new();
    for task in tasks {
        by_priority
            .entry(&task.priority)
            .or_default()
            .push(&task.description);
    }

    let mut lines = Vec::new();
    for p in &["P0", "P1", "P2", "P3"] {
        if let Some(descs) = by_priority.get(p) {
            lines.push(format!("{p}: {} task(s)", descs.len()));
            for desc in descs {
                lines.push(format!("  - {desc}"));
            }
        }
    }

    lines.join("\n")
}

/// Compact summary of recent log history for the cycle selector prompt.
#[derive(Debug, Clone, PartialEq)]
pub struct LogSummary {
    /// Total number of iterations in the log
    pub total_iterations: u32,
    /// Per-cycle execution counts
    pub cycle_counts: HashMap<String, u32>,
    /// Per-cycle success rates (0.0 to 1.0)
    pub cycle_success_rates: HashMap<String, f64>,
    /// Total cost across all logged cycles
    pub total_cost_usd: f64,
    /// Last N outcomes (most recent first)
    pub recent_outcomes: Vec<RecentOutcome>,
}

/// A compact representation of a single recent outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentOutcome {
    /// Iteration number
    pub iteration: u32,
    /// Cycle name
    pub cycle: String,
    /// Whether the cycle succeeded
    pub success: bool,
    /// One-line summary
    pub outcome: String,
}

/// Summarize a JSONL log into a compact form for the cycle selector.
///
/// Takes the last `max_recent` outcomes for the `recent_outcomes` field.
#[must_use]
pub fn summarize_log(log: &[CycleOutcome], max_recent: usize) -> LogSummary {
    let total_iterations = log.iter().map(|o| o.iteration).max().unwrap_or(0);

    let mut cycle_counts: HashMap<String, u32> = HashMap::new();
    let mut cycle_successes: HashMap<String, u32> = HashMap::new();
    let mut total_cost = 0.0;

    for outcome in log {
        *cycle_counts.entry(outcome.cycle.clone()).or_insert(0) += 1;

        // Consider success if outcome doesn't start with "Failed"
        let success = !outcome.outcome.starts_with("Failed");
        if success {
            *cycle_successes.entry(outcome.cycle.clone()).or_insert(0) += 1;
        }

        if let Some(cost) = outcome.total_cost_usd {
            total_cost += cost;
        }
    }

    let cycle_success_rates: HashMap<String, f64> = cycle_counts
        .iter()
        .map(|(name, &count)| {
            let successes = cycle_successes.get(name).copied().unwrap_or(0);
            let rate = if count > 0 {
                f64::from(successes) / f64::from(count)
            } else {
                0.0
            };
            (name.clone(), rate)
        })
        .collect();

    let recent_outcomes: Vec<RecentOutcome> = log
        .iter()
        .rev()
        .take(max_recent)
        .map(|o| RecentOutcome {
            iteration: o.iteration,
            cycle: o.cycle.clone(),
            success: !o.outcome.starts_with("Failed"),
            outcome: o.outcome.clone(),
        })
        .collect();

    LogSummary {
        total_iterations,
        cycle_counts,
        cycle_success_rates,
        total_cost_usd: total_cost,
        recent_outcomes,
    }
}

/// Format a `LogSummary` as a human-readable string for inclusion in a selector prompt.
#[must_use]
pub fn format_log_summary(summary: &LogSummary, config: &FlowConfig) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "Total iterations: {} | Total cost: ${:.2}",
        summary.total_iterations, summary.total_cost_usd
    ));

    // Cycle balance: show all configured cycles, even if they haven't run
    let mut balance_parts = Vec::new();
    for cycle in &config.cycles {
        let count = summary.cycle_counts.get(&cycle.name).copied().unwrap_or(0);
        let rate = summary
            .cycle_success_rates
            .get(&cycle.name)
            .copied()
            .unwrap_or(0.0);
        balance_parts.push(format!(
            "{}={} ({:.0}% success)",
            cycle.name,
            count,
            rate * 100.0
        ));
    }
    lines.push(format!("Cycle balance: {}", balance_parts.join(", ")));

    if !summary.recent_outcomes.is_empty() {
        lines.push("Recent:".to_string());
        for outcome in &summary.recent_outcomes {
            let status = if outcome.success { "ok" } else { "FAIL" };
            lines.push(format!(
                "  #{} {} [{}]: {}",
                outcome.iteration, outcome.cycle, status, outcome.outcome
            ));
        }
    }

    lines.join("\n")
}

/// The result of cycle selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleSelection {
    /// The name of the selected cycle
    pub cycle: String,
    /// The reason for selection
    pub reason: String,
}

/// Build the prompt for the cycle selector.
///
/// Composes log summary, TODO summary, and available cycles into a prompt
/// that asks Claude to return a JSON selection.
#[must_use]
pub fn build_selector_prompt(
    config: &FlowConfig,
    log: &[CycleOutcome],
    todo_content: &str,
) -> String {
    let summary = summarize_log(log, 5);
    let log_text = format_log_summary(&summary, config);
    let tasks = parse_todo_tasks(todo_content);
    let todo_text = format_todo_summary(&tasks);

    let cycle_list: Vec<String> = config
        .cycles
        .iter()
        .map(|c| format!("- {}: {}", c.name, c.description))
        .collect();

    format!(
        r#"You are Flow's cycle selector. Analyze the current state and choose the next cycle to execute.

## Run History
{log_text}

## TODO.md State
{todo_text}

## Available Cycles
{cycle_names}

## Selection Criteria
1. **Priority**: If there are pending P0 tasks, prefer "coding" to make progress
2. **Balance**: Cycles that haven't run recently should get priority
3. **Context**: If a recent cycle failed, consider "gardening" or "review" before retrying coding
4. **Health**: If permission denials or errors are increasing, prefer "review" to diagnose

Choose the next cycle. Respond with ONLY a JSON object on a single line, no other text:
{{"cycle": "<name>", "reason": "<one sentence explanation>"}}"#,
        cycle_names = cycle_list.join("\n"),
    )
}

/// Parse a cycle selection from the selector's response text.
///
/// Looks for a JSON object containing `"cycle"` and `"reason"` fields.
/// Falls back to matching cycle names in the text if JSON parsing fails.
#[must_use]
pub fn parse_selection(response: &str, config: &FlowConfig) -> Option<CycleSelection> {
    // Try to find and parse a JSON object in the response
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let (Some(cycle), Some(reason)) = (
                    value.get("cycle").and_then(|v| v.as_str()),
                    value.get("reason").and_then(|v| v.as_str()),
                ) {
                    // Validate the cycle exists
                    if config.get_cycle(cycle).is_some() {
                        return Some(CycleSelection {
                            cycle: cycle.to_string(),
                            reason: reason.to_string(),
                        });
                    }
                }
            }
        }
    }

    // Fallback: look for a known cycle name mentioned in the response
    for cycle in &config.cycles {
        if response.contains(&cycle.name) {
            return Some(CycleSelection {
                cycle: cycle.name.clone(),
                reason: "Extracted from response text (JSON parse failed)".to_string(),
            });
        }
    }

    None
}

/// Select the next cycle to execute using Claude Code.
///
/// Builds a selector prompt with log and TODO context, invokes Claude Code,
/// and parses the response to determine which cycle to run next.
///
/// # Arguments
/// * `config` - Flow configuration with available cycles
/// * `log` - Recent log history
/// * `todo_content` - Raw TODO.md content
///
/// # Returns
/// The selected cycle, or an error if Claude Code fails or no cycle can be parsed.
pub async fn select_cycle(
    config: &FlowConfig,
    log: &[CycleOutcome],
    todo_content: &str,
) -> Result<CycleSelection> {
    let prompt = build_selector_prompt(config, log, todo_content);

    // Build a minimal command — selector needs no tool permissions
    let cmd = build_command(&prompt, &[]);

    let mut child = TokioCommand::from(cmd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn claude for cycle selection")?;

    let stdout = child.stdout.take().context("No stdout from claude")?;
    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut accumulator = StreamAccumulator::new();

    while let Some(line) = lines
        .next_line()
        .await
        .context("Failed to read selector output")?
    {
        if let Some(event) = parse_event(&line) {
            accumulator.process(&event);
            // We only care about the final result
            if matches!(event, StreamEvent::Result { .. }) {
                break;
            }
        }
    }

    // Ensure the child process finishes
    let _ = child.wait().await;

    // Extract result text from the accumulator
    let result_text = match &accumulator.result {
        Some(StreamEvent::Result { result_text, .. }) => result_text.clone(),
        _ => accumulator.text_fragments.join(""),
    };

    if result_text.is_empty() {
        anyhow::bail!("Cycle selector returned empty response");
    }

    parse_selection(&result_text, config)
        .context("Failed to parse cycle selection from Claude response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_outcome(iteration: u32, cycle: &str, outcome: &str, cost: Option<f64>) -> CycleOutcome {
        CycleOutcome {
            iteration,
            cycle: cycle.to_string(),
            timestamp: Utc::now(),
            outcome: outcome.to_string(),
            files_changed: vec![],
            tests_passed: 0,
            duration_secs: 120,
            num_turns: Some(30),
            total_cost_usd: cost,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        }
    }

    fn make_config(cycle_names: &[&str]) -> FlowConfig {
        FlowConfig::parse(&format!(
            "[global]\npermissions = []\n\n{}",
            cycle_names
                .iter()
                .map(|name| format!(
                    "[[cycle]]\nname = \"{name}\"\ndescription = \"{name}\"\nprompt = \"{name}\"\n"
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ))
        .unwrap()
    }

    // --- summarize_log tests ---

    #[test]
    fn test_summarize_empty_log() {
        let summary = summarize_log(&[], 5);
        assert_eq!(summary.total_iterations, 0);
        assert!(summary.cycle_counts.is_empty());
        assert!(summary.recent_outcomes.is_empty());
        assert_eq!(summary.total_cost_usd, 0.0);
    }

    #[test]
    fn test_summarize_single_entry() {
        let log = vec![make_outcome(1, "coding", "Implemented feature", Some(2.0))];
        let summary = summarize_log(&log, 5);

        assert_eq!(summary.total_iterations, 1);
        assert_eq!(summary.cycle_counts.get("coding"), Some(&1));
        assert_eq!(summary.total_cost_usd, 2.0);
        assert_eq!(summary.recent_outcomes.len(), 1);
        assert!(summary.recent_outcomes[0].success);
    }

    #[test]
    fn test_summarize_multiple_cycles() {
        let log = vec![
            make_outcome(1, "coding", "Implemented feature", Some(2.0)),
            make_outcome(2, "gardening", "Cleaned up code", Some(1.5)),
            make_outcome(3, "coding", "Added tests", Some(1.8)),
        ];
        let summary = summarize_log(&log, 5);

        assert_eq!(summary.total_iterations, 3);
        assert_eq!(summary.cycle_counts.get("coding"), Some(&2));
        assert_eq!(summary.cycle_counts.get("gardening"), Some(&1));
        assert_eq!(summary.total_cost_usd, 5.3);
    }

    #[test]
    fn test_summarize_tracks_failures() {
        let log = vec![
            make_outcome(1, "coding", "Implemented feature", Some(2.0)),
            make_outcome(2, "coding", "Failed with exit code 1", Some(0.5)),
        ];
        let summary = summarize_log(&log, 5);

        assert_eq!(summary.cycle_counts.get("coding"), Some(&2));
        let rate = summary.cycle_success_rates.get("coding").unwrap();
        assert!((rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_summarize_recent_outcomes_limited() {
        let log: Vec<CycleOutcome> = (1..=10)
            .map(|i| make_outcome(i, "coding", &format!("Task {i}"), Some(1.0)))
            .collect();
        let summary = summarize_log(&log, 3);

        assert_eq!(summary.recent_outcomes.len(), 3);
        // Most recent first
        assert_eq!(summary.recent_outcomes[0].iteration, 10);
        assert_eq!(summary.recent_outcomes[1].iteration, 9);
        assert_eq!(summary.recent_outcomes[2].iteration, 8);
    }

    #[test]
    fn test_summarize_cost_with_none_values() {
        let log = vec![
            make_outcome(1, "coding", "done", Some(2.0)),
            make_outcome(2, "coding", "done", None),
        ];
        let summary = summarize_log(&log, 5);
        assert_eq!(summary.total_cost_usd, 2.0);
    }

    // --- format_log_summary tests ---

    #[test]
    fn test_format_empty_summary() {
        let summary = summarize_log(&[], 5);
        let config = make_config(&["coding", "gardening"]);
        let formatted = format_log_summary(&summary, &config);

        assert!(formatted.contains("Total iterations: 0"));
        assert!(formatted.contains("coding=0"));
        assert!(formatted.contains("gardening=0"));
    }

    #[test]
    fn test_format_summary_shows_balance() {
        let log = vec![
            make_outcome(1, "coding", "done", Some(2.0)),
            make_outcome(2, "coding", "done", Some(1.5)),
            make_outcome(3, "gardening", "done", Some(1.0)),
        ];
        let summary = summarize_log(&log, 5);
        let config = make_config(&["coding", "gardening", "review"]);
        let formatted = format_log_summary(&summary, &config);

        assert!(formatted.contains("coding=2"));
        assert!(formatted.contains("gardening=1"));
        assert!(formatted.contains("review=0"));
        assert!(formatted.contains("Total cost: $4.50"));
    }

    // --- parse_todo_tasks tests ---

    #[test]
    fn test_parse_todo_empty() {
        let tasks = parse_todo_tasks("");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_todo_pending_with_priority() {
        let content = r#"
- [ ] Implement cycle selector
  - Priority: P0
"#;
        let tasks = parse_todo_tasks(content);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].priority, "P0");
        assert_eq!(tasks[0].description, "Implement cycle selector");
    }

    #[test]
    fn test_parse_todo_ignores_completed() {
        let content = r#"
- [x] Already done task
  - Priority: P0

- [ ] Still pending task
  - Priority: P1
"#;
        let tasks = parse_todo_tasks(content);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "Still pending task");
        assert_eq!(tasks[0].priority, "P1");
    }

    #[test]
    fn test_parse_todo_multiple_priorities() {
        let content = r#"
- [ ] Critical task
  - Priority: P0

- [ ] Important task
  - Priority: P1

- [ ] Nice to have
  - Priority: P2
"#;
        let tasks = parse_todo_tasks(content);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].priority, "P0");
        assert_eq!(tasks[1].priority, "P1");
        assert_eq!(tasks[2].priority, "P2");
    }

    #[test]
    fn test_parse_todo_no_priority_skipped() {
        let content = r#"
- [ ] Task without priority info

- [ ] Task with priority
  - Priority: P0
"#;
        let tasks = parse_todo_tasks(content);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "Task with priority");
    }

    #[test]
    fn test_format_todo_summary_empty() {
        let formatted = format_todo_summary(&[]);
        assert_eq!(formatted, "No pending tasks found in TODO.md");
    }

    #[test]
    fn test_format_todo_summary_grouped() {
        let tasks = vec![
            TodoTask {
                priority: "P0".to_string(),
                description: "Critical thing".to_string(),
            },
            TodoTask {
                priority: "P1".to_string(),
                description: "Less critical".to_string(),
            },
            TodoTask {
                priority: "P0".to_string(),
                description: "Another critical".to_string(),
            },
        ];
        let formatted = format_todo_summary(&tasks);
        assert!(formatted.contains("P0: 2 task(s)"));
        assert!(formatted.contains("P1: 1 task(s)"));
        assert!(formatted.contains("Critical thing"));
    }

    // --- build_selector_prompt tests ---

    #[test]
    fn test_build_selector_prompt_includes_cycles() {
        let config = make_config(&["coding", "gardening"]);
        let prompt = build_selector_prompt(&config, &[], "");
        assert!(prompt.contains("coding"));
        assert!(prompt.contains("gardening"));
        assert!(prompt.contains("cycle selector"));
    }

    #[test]
    fn test_build_selector_prompt_includes_log_context() {
        let config = make_config(&["coding"]);
        let log = vec![make_outcome(1, "coding", "Implemented feature", Some(2.0))];
        let prompt = build_selector_prompt(&config, &log, "");
        assert!(prompt.contains("Total iterations: 1"));
        assert!(prompt.contains("coding=1"));
    }

    #[test]
    fn test_build_selector_prompt_includes_todo_context() {
        let config = make_config(&["coding"]);
        let todo = "- [ ] Fix the bug\n  - Priority: P0\n";
        let prompt = build_selector_prompt(&config, &[], todo);
        assert!(prompt.contains("P0: 1 task(s)"));
        assert!(prompt.contains("Fix the bug"));
    }

    // --- parse_selection tests ---

    #[test]
    fn test_parse_selection_valid_json() {
        let config = make_config(&["coding", "gardening"]);
        let response = r#"{"cycle": "coding", "reason": "P0 tasks pending"}"#;
        let selection = parse_selection(response, &config).unwrap();
        assert_eq!(selection.cycle, "coding");
        assert_eq!(selection.reason, "P0 tasks pending");
    }

    #[test]
    fn test_parse_selection_json_with_surrounding_text() {
        let config = make_config(&["coding", "gardening"]);
        let response = "Here is my selection:\n{\"cycle\": \"gardening\", \"reason\": \"Hasn't run recently\"}\nDone.";
        let selection = parse_selection(response, &config).unwrap();
        assert_eq!(selection.cycle, "gardening");
    }

    #[test]
    fn test_parse_selection_invalid_cycle_falls_back() {
        let config = make_config(&["coding", "gardening"]);
        let response = r#"{"cycle": "nonexistent", "reason": "test"}"#;
        // JSON has invalid cycle, but "coding" and "gardening" aren't in text either
        let selection = parse_selection(response, &config);
        assert!(selection.is_none());
    }

    #[test]
    fn test_parse_selection_fallback_to_text_match() {
        let config = make_config(&["coding", "gardening"]);
        let response = "I think we should run the gardening cycle next.";
        let selection = parse_selection(response, &config).unwrap();
        assert_eq!(selection.cycle, "gardening");
    }

    #[test]
    fn test_parse_selection_no_match_returns_none() {
        let config = make_config(&["coding", "gardening"]);
        let response = "I don't know what to do.";
        assert!(parse_selection(response, &config).is_none());
    }

    #[test]
    fn test_parse_selection_prefers_json_over_text() {
        let config = make_config(&["coding", "gardening"]);
        // JSON says gardening, text mentions coding
        let response =
            "Let me suggest coding.\n{\"cycle\": \"gardening\", \"reason\": \"Balance\"}\n";
        let selection = parse_selection(response, &config).unwrap();
        assert_eq!(selection.cycle, "gardening");
    }

    #[test]
    fn test_format_summary_shows_recent() {
        let log = vec![
            make_outcome(1, "coding", "Implemented feature X", Some(2.0)),
            make_outcome(2, "coding", "Failed with exit code 1", Some(0.5)),
        ];
        let summary = summarize_log(&log, 5);
        let config = make_config(&["coding"]);
        let formatted = format_log_summary(&summary, &config);

        assert!(formatted.contains("[FAIL]"));
        assert!(formatted.contains("[ok]"));
        assert!(formatted.contains("Implemented feature X"));
    }
}
