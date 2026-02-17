//! Iteration context injection for cycle prompts
//!
//! Builds a context block from JSONL log history based on a cycle's `ContextMode`.
//! This context block is prepended to the cycle's prompt so Claude knows what
//! happened in previous iterations.

use crate::cycle::config::ContextMode;
use crate::log::jsonl::CycleOutcome;

/// Build a context block from log history based on the given `ContextMode`.
///
/// Returns `None` when `mode` is `ContextMode::None`, meaning no context
/// should be added to the prompt. Returns `Some(text)` for `Full` and
/// `Summaries` modes, even if the log is empty (in which case the block
/// indicates no history exists yet).
#[must_use]
pub fn build_context(mode: &ContextMode, outcomes: &[CycleOutcome]) -> Option<String> {
    match mode {
        ContextMode::None => None,
        ContextMode::Summaries => Some(build_summaries_context(outcomes)),
        ContextMode::Full => Some(build_full_context(outcomes)),
    }
}

/// Format context as a brief summary list — one line per iteration.
fn build_summaries_context(outcomes: &[CycleOutcome]) -> String {
    let mut lines = vec!["## Previous Iteration Summaries".to_string(), String::new()];

    if outcomes.is_empty() {
        lines.push("No previous iterations.".to_string());
    } else {
        for outcome in outcomes {
            lines.push(format!(
                "- Iteration {} [{}]: {}",
                outcome.iteration, outcome.cycle, outcome.outcome
            ));
        }
    }

    lines.join("\n")
}

/// Format context as full JSONL history — structured details per iteration.
fn build_full_context(outcomes: &[CycleOutcome]) -> String {
    let mut lines = vec!["## Full Iteration History".to_string(), String::new()];

    if outcomes.is_empty() {
        lines.push("No previous iterations.".to_string());
    } else {
        for outcome in outcomes {
            lines.push(format!(
                "### Iteration {} — {}",
                outcome.iteration, outcome.cycle
            ));
            lines.push(format!("Timestamp: {}", outcome.timestamp));
            lines.push(format!("Outcome: {}", outcome.outcome));
            lines.push(format!("Duration: {}s", outcome.duration_secs));
            if let Some(turns) = outcome.num_turns {
                lines.push(format!("Turns: {turns}"));
            }
            if let Some(cost) = outcome.total_cost_usd {
                lines.push(format!("Cost: ${cost:.4}"));
            }
            if !outcome.files_changed.is_empty() {
                lines.push(format!(
                    "Files changed: {}",
                    outcome.files_changed.join(", ")
                ));
            }
            if let Some(denials) = outcome.permission_denial_count {
                if denials > 0 {
                    lines.push(format!("Permission denials: {denials}"));
                }
            }
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

/// Inject a context block into a prompt string.
///
/// If context is `None`, returns the original prompt unchanged.
/// If context is `Some(block)`, prepends the block followed by a separator.
#[must_use]
pub fn inject_context(prompt: &str, context: Option<String>) -> String {
    context.map_or_else(
        || prompt.to_string(),
        |block| format!("{block}\n\n---\n\n{prompt}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::jsonl::CycleOutcome;
    use chrono::Utc;

    fn make_outcome(iteration: u32, cycle: &str, outcome: &str) -> CycleOutcome {
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
        }
    }

    // --- build_context: ContextMode::None ---

    #[test]
    fn test_context_none_returns_none() {
        let outcomes = vec![make_outcome(1, "coding", "Implemented feature X")];
        let result = build_context(&ContextMode::None, &outcomes);
        assert_eq!(result, None);
    }

    #[test]
    fn test_context_none_with_empty_log_returns_none() {
        let result = build_context(&ContextMode::None, &[]);
        assert_eq!(result, None);
    }

    // --- build_context: ContextMode::Summaries ---

    #[test]
    fn test_summaries_empty_log_returns_some() {
        let result = build_context(&ContextMode::Summaries, &[]);
        assert!(result.is_some());
    }

    #[test]
    fn test_summaries_empty_log_indicates_no_history() {
        let result = build_context(&ContextMode::Summaries, &[]).unwrap();
        assert!(
            result.contains("No previous iterations"),
            "Expected 'No previous iterations' in: {result}"
        );
    }

    #[test]
    fn test_summaries_includes_iteration_number() {
        let outcomes = vec![make_outcome(3, "coding", "Some work done")];
        let result = build_context(&ContextMode::Summaries, &outcomes).unwrap();
        assert!(
            result.contains("Iteration 3"),
            "Missing iteration number: {result}"
        );
    }

    #[test]
    fn test_summaries_includes_cycle_name() {
        let outcomes = vec![make_outcome(1, "gardening", "Cleaned up deps")];
        let result = build_context(&ContextMode::Summaries, &outcomes).unwrap();
        assert!(result.contains("gardening"), "Missing cycle name: {result}");
    }

    #[test]
    fn test_summaries_includes_outcome_text() {
        let outcomes = vec![make_outcome(1, "coding", "Implemented the logger")];
        let result = build_context(&ContextMode::Summaries, &outcomes).unwrap();
        assert!(
            result.contains("Implemented the logger"),
            "Missing outcome text: {result}"
        );
    }

    #[test]
    fn test_summaries_multiple_outcomes_all_included() {
        let outcomes = vec![
            make_outcome(1, "coding", "Built feature A"),
            make_outcome(2, "gardening", "Cleaned deps"),
            make_outcome(3, "review", "Reviewed changes"),
        ];
        let result = build_context(&ContextMode::Summaries, &outcomes).unwrap();
        assert!(
            result.contains("Iteration 1"),
            "Missing iteration 1: {result}"
        );
        assert!(
            result.contains("Iteration 2"),
            "Missing iteration 2: {result}"
        );
        assert!(
            result.contains("Iteration 3"),
            "Missing iteration 3: {result}"
        );
        assert!(
            result.contains("Built feature A"),
            "Missing outcome 1: {result}"
        );
        assert!(
            result.contains("Cleaned deps"),
            "Missing outcome 2: {result}"
        );
        assert!(
            result.contains("Reviewed changes"),
            "Missing outcome 3: {result}"
        );
    }

    #[test]
    fn test_summaries_has_header() {
        let result = build_context(&ContextMode::Summaries, &[]).unwrap();
        assert!(
            result.contains("Previous Iteration Summaries"),
            "Missing header: {result}"
        );
    }

    // --- build_context: ContextMode::Full ---

    #[test]
    fn test_full_empty_log_returns_some() {
        let result = build_context(&ContextMode::Full, &[]);
        assert!(result.is_some());
    }

    #[test]
    fn test_full_empty_log_indicates_no_history() {
        let result = build_context(&ContextMode::Full, &[]).unwrap();
        assert!(
            result.contains("No previous iterations"),
            "Expected 'No previous iterations' in: {result}"
        );
    }

    #[test]
    fn test_full_includes_iteration_number() {
        let outcomes = vec![make_outcome(5, "coding", "Big feature")];
        let result = build_context(&ContextMode::Full, &outcomes).unwrap();
        assert!(
            result.contains("Iteration 5"),
            "Missing iteration: {result}"
        );
    }

    #[test]
    fn test_full_includes_cycle_name_in_header() {
        let outcomes = vec![make_outcome(1, "review", "Code review done")];
        let result = build_context(&ContextMode::Full, &outcomes).unwrap();
        assert!(result.contains("review"), "Missing cycle name: {result}");
    }

    #[test]
    fn test_full_includes_outcome_text() {
        let outcomes = vec![make_outcome(1, "coding", "Implemented context injector")];
        let result = build_context(&ContextMode::Full, &outcomes).unwrap();
        assert!(
            result.contains("Implemented context injector"),
            "Missing outcome: {result}"
        );
    }

    #[test]
    fn test_full_includes_duration() {
        let mut outcome = make_outcome(1, "coding", "done");
        outcome.duration_secs = 142;
        let result = build_context(&ContextMode::Full, &[outcome]).unwrap();
        assert!(result.contains("142"), "Missing duration: {result}");
    }

    #[test]
    fn test_full_includes_num_turns_when_present() {
        let mut outcome = make_outcome(1, "coding", "done");
        outcome.num_turns = Some(37);
        let result = build_context(&ContextMode::Full, &[outcome]).unwrap();
        assert!(result.contains("37"), "Missing turns: {result}");
    }

    #[test]
    fn test_full_includes_cost_when_present() {
        let mut outcome = make_outcome(1, "coding", "done");
        outcome.total_cost_usd = Some(1.23);
        let result = build_context(&ContextMode::Full, &[outcome]).unwrap();
        assert!(result.contains("1.23"), "Missing cost: {result}");
    }

    #[test]
    fn test_full_includes_files_changed() {
        let mut outcome = make_outcome(1, "coding", "done");
        outcome.files_changed = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        let result = build_context(&ContextMode::Full, &[outcome]).unwrap();
        assert!(result.contains("src/main.rs"), "Missing files: {result}");
        assert!(result.contains("src/lib.rs"), "Missing files: {result}");
    }

    #[test]
    fn test_full_omits_empty_files_changed() {
        let outcome = make_outcome(1, "coding", "done");
        let result = build_context(&ContextMode::Full, &[outcome]).unwrap();
        assert!(
            !result.contains("Files changed"),
            "Should omit files section when empty: {result}"
        );
    }

    #[test]
    fn test_full_includes_permission_denials_when_nonzero() {
        let mut outcome = make_outcome(1, "coding", "done");
        outcome.permission_denial_count = Some(3);
        let result = build_context(&ContextMode::Full, &[outcome]).unwrap();
        assert!(
            result.contains("Permission denials: 3"),
            "Missing denial count: {result}"
        );
    }

    #[test]
    fn test_full_omits_permission_denials_when_zero() {
        let mut outcome = make_outcome(1, "coding", "done");
        outcome.permission_denial_count = Some(0);
        let result = build_context(&ContextMode::Full, &[outcome]).unwrap();
        assert!(
            !result.contains("Permission denials"),
            "Should omit denial section when zero: {result}"
        );
    }

    #[test]
    fn test_full_has_header() {
        let result = build_context(&ContextMode::Full, &[]).unwrap();
        assert!(
            result.contains("Full Iteration History"),
            "Missing header: {result}"
        );
    }

    // --- inject_context ---

    #[test]
    fn test_inject_context_none_returns_prompt_unchanged() {
        let prompt = "You are the coding cycle.";
        let result = inject_context(prompt, None);
        assert_eq!(result, prompt);
    }

    #[test]
    fn test_inject_context_some_prepends_block() {
        let prompt = "You are the coding cycle.";
        let block = "## Previous History\n\n- Iteration 1: Done".to_string();
        let result = inject_context(prompt, Some(block.clone()));
        assert!(
            result.starts_with(&block),
            "Context block should be first: {result}"
        );
        assert!(result.contains(prompt), "Original prompt missing: {result}");
    }

    #[test]
    fn test_inject_context_separator_between_context_and_prompt() {
        let prompt = "Run coding cycle.";
        let block = "## History".to_string();
        let result = inject_context(prompt, Some(block));
        assert!(result.contains("---"), "Expected separator '---': {result}");
    }

    #[test]
    fn test_inject_context_prompt_comes_after_separator() {
        let prompt = "Run coding cycle.";
        let block = "## History\n\nSome context.".to_string();
        let result = inject_context(prompt, Some(block));
        let sep_pos = result.find("---").unwrap();
        let prompt_pos = result.find(prompt).unwrap();
        assert!(prompt_pos > sep_pos, "Prompt should come after separator");
    }
}
