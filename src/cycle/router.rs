//! Step router — determines the next step to execute in a multi-step cycle.
//!
//! Supports two routing modes:
//! - **Sequential** (default): proceed to the next step in TOML order.
//! - **LLM**: invoke Claude Code to choose the next step based on the
//!   completed step's output text and the available step names.

use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::claude::cli::{build_command, run_for_result};
use crate::cycle::config::{StepConfig, StepRouter};

/// The result of routing after a step completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteDecision {
    /// Proceed to a specific step by name.
    GoTo {
        /// The name of the next step to execute.
        step_name: String,
        /// The reason for this routing decision.
        reason: String,
    },
    /// The cycle is complete — no more steps to execute.
    Done {
        /// The reason for finishing.
        reason: String,
    },
}

/// Track how many times each step has been visited in the current cycle execution.
#[derive(Debug, Default)]
pub(crate) struct VisitTracker {
    visits: HashMap<String, u32>,
}

impl VisitTracker {
    /// Create a new empty visit tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a visit to a step. Returns the new visit count.
    pub fn record(&mut self, step_name: &str) -> u32 {
        let count = self.visits.entry(step_name.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Get the current visit count for a step.
    #[must_use]
    fn count(&self, step_name: &str) -> u32 {
        self.visits.get(step_name).copied().unwrap_or(0)
    }

    /// Check whether visiting a step would exceed its `max_visits` limit.
    #[must_use]
    pub fn would_exceed(&self, step_name: &str, max_visits: u32) -> bool {
        self.count(step_name) >= max_visits
    }
}

/// Determine the next step index for sequential routing.
///
/// Returns `None` if the current step is the last one (cycle complete).
#[must_use]
const fn route_sequential(current_step_index: usize, total_steps: usize) -> Option<usize> {
    let next = current_step_index + 1;
    if next < total_steps {
        Some(next)
    } else {
        None
    }
}

/// Build the prompt for the LLM step router.
///
/// Provides the completed step's output text and the list of available step
/// names so the LLM can choose what to do next.
#[must_use]
fn build_router_prompt(
    completed_step_name: &str,
    result_text: &str,
    available_steps: &[&str],
) -> String {
    let step_list: Vec<String> = available_steps
        .iter()
        .map(|name| format!("- {name}"))
        .collect();

    format!(
        r#"You are a step router for an automated coding pipeline.

The step "{completed_step_name}" just completed. Here is its output:

---
{result_text}
---

## Available Steps
{steps}

## Instructions
Based on the step output, decide which step should execute next.
- If the work is complete and no more steps are needed, respond with "DONE".
- Otherwise, choose the most appropriate next step from the list above.

Respond with ONLY a JSON object on a single line, no other text:
{{"next": "<step_name or DONE>", "reason": "<one sentence explanation>"}}"#,
        steps = step_list.join("\n"),
    )
}

/// Parse the LLM router's response to determine the next step.
///
/// Looks for a JSON object with `"next"` and `"reason"` fields.
/// Falls back to matching step names in the text if JSON parsing fails.
#[must_use]
fn parse_router_response(response: &str, available_steps: &[&str]) -> Option<RouteDecision> {
    // Try JSON parsing first
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let (Some(next), Some(reason)) = (
                    value.get("next").and_then(|v| v.as_str()),
                    value.get("reason").and_then(|v| v.as_str()),
                ) {
                    if next.eq_ignore_ascii_case("done") {
                        return Some(RouteDecision::Done {
                            reason: reason.to_string(),
                        });
                    }
                    if available_steps.contains(&next) {
                        return Some(RouteDecision::GoTo {
                            step_name: next.to_string(),
                            reason: reason.to_string(),
                        });
                    }
                }
            }
        }
    }

    // Fallback: look for "DONE" in response
    if response.contains("DONE") {
        return Some(RouteDecision::Done {
            reason: "Extracted from response text (JSON parse failed)".to_string(),
        });
    }

    // Fallback: look for a known step name in the response
    for &step_name in available_steps {
        if response.contains(step_name) {
            return Some(RouteDecision::GoTo {
                step_name: step_name.to_string(),
                reason: "Extracted from response text (JSON parse failed)".to_string(),
            });
        }
    }

    None
}

/// Invoke Claude Code to route to the next step.
///
/// Builds a router prompt, invokes Claude with no tool permissions,
/// and parses the response to get a `RouteDecision`.
async fn route_with_llm(
    completed_step_name: &str,
    result_text: &str,
    available_steps: &[&str],
) -> Result<RouteDecision> {
    let prompt = build_router_prompt(completed_step_name, result_text, available_steps);
    let cmd = build_command(&prompt, &[]);
    let response = run_for_result(cmd).await?;

    parse_router_response(&response, available_steps)
        .context("Failed to parse step routing from Claude response")
}

/// Determine the next step to execute after the current step completes.
///
/// For `Sequential` routing, this is a simple index increment.
/// For `Llm` routing, this invokes Claude Code to make the decision.
///
/// Returns `Ok(None)` when the cycle is complete (no more steps).
pub(crate) async fn determine_next_step(
    completed_step: &StepConfig,
    completed_step_index: usize,
    result_text: &str,
    all_steps: &[StepConfig],
    visit_tracker: &VisitTracker,
) -> Result<Option<RouteDecision>> {
    match completed_step.router {
        StepRouter::Sequential => Ok(route_sequential(completed_step_index, all_steps.len()).map(
            |next_idx| RouteDecision::GoTo {
                step_name: all_steps[next_idx].name.clone(),
                reason: "Sequential progression".to_string(),
            },
        )),
        StepRouter::Llm => {
            let available: Vec<&str> = all_steps
                .iter()
                .filter(|s| !visit_tracker.would_exceed(&s.name, s.max_visits))
                .map(|s| s.name.as_str())
                .collect();

            if available.is_empty() {
                return Ok(Some(RouteDecision::Done {
                    reason: "All steps have reached their max_visits limit".to_string(),
                }));
            }

            let decision = route_with_llm(&completed_step.name, result_text, &available).await?;
            Ok(Some(decision))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- VisitTracker tests ---

    #[test]
    fn test_visit_tracker_starts_empty() {
        let tracker = VisitTracker::new();
        assert_eq!(tracker.count("plan"), 0);
    }

    #[test]
    fn test_visit_tracker_records_visits() {
        let mut tracker = VisitTracker::new();
        assert_eq!(tracker.record("plan"), 1);
        assert_eq!(tracker.record("plan"), 2);
        assert_eq!(tracker.count("plan"), 2);
    }

    #[test]
    fn test_visit_tracker_independent_steps() {
        let mut tracker = VisitTracker::new();
        tracker.record("plan");
        tracker.record("implement");
        assert_eq!(tracker.count("plan"), 1);
        assert_eq!(tracker.count("implement"), 1);
    }

    #[test]
    fn test_visit_tracker_would_exceed() {
        let mut tracker = VisitTracker::new();
        tracker.record("plan");
        tracker.record("plan");
        tracker.record("plan");
        assert!(tracker.would_exceed("plan", 3));
        assert!(!tracker.would_exceed("plan", 4));
    }

    #[test]
    fn test_visit_tracker_would_not_exceed_when_empty() {
        let tracker = VisitTracker::new();
        assert!(!tracker.would_exceed("plan", 3));
    }

    // --- route_sequential tests ---

    #[test]
    fn test_route_sequential_first_step() {
        assert_eq!(route_sequential(0, 3), Some(1));
    }

    #[test]
    fn test_route_sequential_middle_step() {
        assert_eq!(route_sequential(1, 3), Some(2));
    }

    #[test]
    fn test_route_sequential_last_step() {
        assert_eq!(route_sequential(2, 3), None);
    }

    #[test]
    fn test_route_sequential_single_step() {
        assert_eq!(route_sequential(0, 1), None);
    }

    // --- build_router_prompt tests ---

    #[test]
    fn test_build_router_prompt_contains_step_name() {
        let prompt = build_router_prompt("plan-review", "Plan approved", &["plan", "implement"]);
        assert!(prompt.contains("plan-review"));
        assert!(prompt.contains("Plan approved"));
    }

    #[test]
    fn test_build_router_prompt_lists_available_steps() {
        let prompt = build_router_prompt("review", "Output", &["plan", "implement", "test"]);
        assert!(prompt.contains("- plan"));
        assert!(prompt.contains("- implement"));
        assert!(prompt.contains("- test"));
    }

    // --- parse_router_response tests ---

    #[test]
    fn test_parse_router_response_valid_goto() {
        let response = r#"{"next": "implement", "reason": "Plan was approved"}"#;
        let decision = parse_router_response(response, &["plan", "implement", "review"]).unwrap();
        assert_eq!(
            decision,
            RouteDecision::GoTo {
                step_name: "implement".to_string(),
                reason: "Plan was approved".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_router_response_done() {
        let response = r#"{"next": "DONE", "reason": "All work complete"}"#;
        let decision = parse_router_response(response, &["plan", "implement"]).unwrap();
        assert_eq!(
            decision,
            RouteDecision::Done {
                reason: "All work complete".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_router_response_done_case_insensitive() {
        let response = r#"{"next": "done", "reason": "Finished"}"#;
        let decision = parse_router_response(response, &["plan"]).unwrap();
        assert_eq!(
            decision,
            RouteDecision::Done {
                reason: "Finished".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_router_response_json_with_surrounding_text() {
        let response =
            "Here is my decision:\n{\"next\": \"plan\", \"reason\": \"Retry plan\"}\nDone.";
        let decision = parse_router_response(response, &["plan", "implement"]).unwrap();
        assert_eq!(
            decision,
            RouteDecision::GoTo {
                step_name: "plan".to_string(),
                reason: "Retry plan".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_router_response_fallback_done() {
        let response = "The work is DONE and nothing more is needed.";
        let decision = parse_router_response(response, &["plan", "implement"]).unwrap();
        assert!(matches!(decision, RouteDecision::Done { .. }));
    }

    #[test]
    fn test_parse_router_response_fallback_step_name() {
        let response = "We should go to the implement step next.";
        let decision = parse_router_response(response, &["plan", "implement"]).unwrap();
        assert_eq!(
            decision,
            RouteDecision::GoTo {
                step_name: "implement".to_string(),
                reason: "Extracted from response text (JSON parse failed)".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_router_response_no_match() {
        let response = "I have no idea what to do next.";
        let decision = parse_router_response(response, &["plan", "implement"]);
        assert!(decision.is_none());
    }

    #[test]
    fn test_parse_router_response_done_takes_priority_over_step_name_in_fallback() {
        // When both "DONE" and a step name appear in text (no JSON), DONE wins
        let response = "The implement step is not needed, we are DONE.";
        let decision = parse_router_response(response, &["plan", "implement"]).unwrap();
        assert!(
            matches!(decision, RouteDecision::Done { .. }),
            "DONE should take priority over step name in fallback: {decision:?}"
        );
    }

    #[test]
    fn test_parse_router_response_invalid_step_in_json() {
        // JSON has valid format but step name doesn't exist in available list
        let response = r#"{"next": "nonexistent", "reason": "test"}"#;
        let decision = parse_router_response(response, &["plan", "implement"]);
        assert!(decision.is_none());
    }

    #[test]
    fn test_parse_router_response_prefers_json_over_fallback() {
        // JSON says plan, text mentions implement — JSON should win
        let response =
            "Let's go to implement.\n{\"next\": \"plan\", \"reason\": \"Need to re-plan\"}";
        let decision = parse_router_response(response, &["plan", "implement"]).unwrap();
        assert_eq!(
            decision,
            RouteDecision::GoTo {
                step_name: "plan".to_string(),
                reason: "Need to re-plan".to_string(),
            }
        );
    }

    // --- determine_next_step tests (synchronous variants) ---

    fn make_step(name: &str, router: StepRouter, max_visits: u32) -> StepConfig {
        StepConfig {
            name: name.to_string(),
            session: None,
            prompt: format!("Do {name}"),
            permissions: vec![],
            router,
            max_visits,
        }
    }

    #[tokio::test]
    async fn test_determine_next_step_sequential_middle() {
        let steps = vec![
            make_step("plan", StepRouter::Sequential, 3),
            make_step("implement", StepRouter::Sequential, 3),
            make_step("test", StepRouter::Sequential, 3),
        ];
        let tracker = VisitTracker::new();
        let result = determine_next_step(&steps[0], 0, "Done planning", &steps, &tracker)
            .await
            .unwrap();
        assert_eq!(
            result,
            Some(RouteDecision::GoTo {
                step_name: "implement".to_string(),
                reason: "Sequential progression".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn test_determine_next_step_sequential_last() {
        let steps = vec![
            make_step("plan", StepRouter::Sequential, 3),
            make_step("implement", StepRouter::Sequential, 3),
        ];
        let tracker = VisitTracker::new();
        let result = determine_next_step(&steps[1], 1, "Done implementing", &steps, &tracker)
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
