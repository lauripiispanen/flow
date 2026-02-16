//! Flow - Automated coding pipeline runner
//!
//! CLI entry point for the Flow orchestrator.

// Allow multiple crate versions from dependencies (can't easily control)
#![allow(clippy::multiple_crate_versions)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use flow::claude::stream::suggest_permission_fix;
use flow::cycle::config::FlowConfig;
use flow::cycle::executor::CycleExecutor;
use flow::cycle::rules::find_triggered_cycles;
use flow::log::jsonl::JsonlLogger;
use flow::log::CycleOutcome;

/// Automated coding pipeline runner
///
/// Orchestrates Claude Code CLI in structured cycles (coding, gardening,
/// review, planning) with controlled permissions and observability.
#[derive(Parser, Debug)]
#[command(name = "flow", version, about)]
struct Cli {
    /// Name of the cycle to execute
    #[arg(long)]
    cycle: String,

    /// Path to the cycles.toml configuration file
    #[arg(long, default_value = "cycles.toml")]
    config: PathBuf,

    /// Directory for log files (.flow by default)
    #[arg(long, default_value = ".flow")]
    log_dir: PathBuf,
}

/// Format an exit code for display, returning "unknown" if the process was killed by signal.
fn format_exit_code(exit_code: Option<i32>) -> String {
    exit_code.map_or_else(|| "unknown".to_string(), |c| c.to_string())
}

/// Build a `CycleOutcome` from a `CycleResult` for JSONL logging.
fn build_outcome(result: &flow::CycleResult, iteration: u32) -> CycleOutcome {
    let outcome_text = result.result_text.clone().unwrap_or_else(|| {
        if result.success {
            "Completed successfully".to_string()
        } else {
            format!(
                "Failed with exit code {}",
                format_exit_code(result.exit_code)
            )
        }
    });

    CycleOutcome {
        iteration,
        cycle: result.cycle_name.clone(),
        timestamp: chrono::Utc::now(),
        outcome: outcome_text,
        files_changed: vec![],
        tests_passed: 0,
        duration_secs: result.duration_secs,
        num_turns: result.num_turns,
        total_cost_usd: result.total_cost_usd,
        permission_denial_count: result.permission_denial_count,
        permission_denials: result.permission_denials.clone(),
    }
}

/// Check if permission denials exceed the threshold and exit if so.
fn check_denial_gate(denials: u32, max_denials: u32, cycle_name: &str) {
    if denials > max_denials {
        eprintln!(
            "Stopping: {denials} permission denials in '{cycle_name}' exceeded threshold ({max_denials}). \
             Fix permissions in cycles.toml before continuing."
        );
        std::process::exit(1);
    }
}

/// Execute a cycle with rich display and log the result. Returns the `CycleResult`.
async fn execute_and_log(
    executor: &CycleExecutor,
    logger: &JsonlLogger,
    cycle_name: &str,
    iteration: &mut u32,
    circuit_breaker_threshold: u32,
) -> Result<flow::CycleResult> {
    let result = executor
        .execute_with_display(cycle_name, circuit_breaker_threshold)
        .await
        .with_context(|| format!("Failed to execute cycle '{cycle_name}'"))?;

    let outcome = build_outcome(&result, *iteration);
    logger
        .append(&outcome)
        .context("Failed to write to JSONL log")?;

    // Print actionable permission fix suggestions
    if let Some(count) = result.permission_denial_count {
        if count > 0 {
            eprintln!("Tip: Add permission strings to cycles.toml to avoid denials.");
            eprintln!("     e.g. {}", suggest_permission_fix("Edit"));
        }
    }

    *iteration += 1;

    Ok(result)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = FlowConfig::from_path(&cli.config)
        .with_context(|| format!("Failed to load config from '{}'", cli.config.display()))?;

    // Validate the requested cycle exists
    config.get_cycle(&cli.cycle).with_context(|| {
        format!(
            "Unknown cycle '{}'. Available cycles: {}",
            cli.cycle,
            available_cycle_names(&config)
        )
    })?;

    // Initialize
    let circuit_breaker = config.global.circuit_breaker_repeated;
    let max_denials = config.global.max_permission_denials;
    let executor = CycleExecutor::new(config.clone());
    let logger = JsonlLogger::new(&cli.log_dir).context("Failed to initialize JSONL logger")?;
    let mut iteration: u32 = 1;

    // Execute the requested cycle
    let result = execute_and_log(
        &executor,
        &logger,
        &cli.cycle,
        &mut iteration,
        circuit_breaker,
    )
    .await?;

    // Auto-trigger dependent cycles if the primary cycle succeeded
    if result.success {
        // Between-cycle gate: stop if too many permission denials
        check_denial_gate(
            result.permission_denial_count.unwrap_or(0),
            max_denials,
            &result.cycle_name,
        );

        // Read log history for frequency-aware triggering
        let log_entries = logger
            .read_all()
            .context("Failed to read log for frequency check")?;
        let triggered = find_triggered_cycles(&config, &result.cycle_name, &log_entries);
        for dep_cycle in triggered {
            eprintln!("Auto-triggering dependent cycle: {dep_cycle}");
            let dep_result = execute_and_log(
                &executor,
                &logger,
                dep_cycle,
                &mut iteration,
                circuit_breaker,
            )
            .await?;

            if !dep_result.success {
                eprintln!("Dependent cycle '{dep_cycle}' failed, stopping.");
                std::process::exit(dep_result.exit_code.unwrap_or(1));
            }

            // Between-cycle gate for dependent cycles too
            check_denial_gate(
                dep_result.permission_denial_count.unwrap_or(0),
                max_denials,
                dep_cycle,
            );
        }
    }

    // Exit with appropriate code
    if result.success {
        Ok(())
    } else {
        std::process::exit(result.exit_code.unwrap_or(1));
    }
}

/// Format available cycle names for error messages.
fn available_cycle_names(config: &FlowConfig) -> String {
    config
        .cycles
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::CycleResult;

    #[test]
    fn test_build_outcome_success() {
        let result = CycleResult {
            cycle_name: "coding".to_string(),
            success: true,
            exit_code: Some(0),
            stdout: "done".to_string(),
            stderr: String::new(),
            duration_secs: 120,
            result_text: None,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
        };

        let outcome = build_outcome(&result, 1);
        assert_eq!(outcome.cycle, "coding");
        assert_eq!(outcome.iteration, 1);
        assert_eq!(outcome.outcome, "Completed successfully");
        assert_eq!(outcome.duration_secs, 120);
        assert!(outcome.files_changed.is_empty());
    }

    #[test]
    fn test_build_outcome_failure() {
        let result = CycleResult {
            cycle_name: "coding".to_string(),
            success: false,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error".to_string(),
            duration_secs: 30,
            result_text: None,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
        };

        let outcome = build_outcome(&result, 3);
        assert_eq!(outcome.outcome, "Failed with exit code 1");
        assert_eq!(outcome.iteration, 3);
    }

    #[test]
    fn test_build_outcome_killed_by_signal() {
        let result = CycleResult {
            cycle_name: "coding".to_string(),
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            duration_secs: 5,
            result_text: None,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
        };

        let outcome = build_outcome(&result, 1);
        assert_eq!(outcome.outcome, "Failed with exit code unknown");
    }

    #[test]
    fn test_build_outcome_uses_result_text_when_present() {
        let result = CycleResult {
            cycle_name: "coding".to_string(),
            success: true,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration_secs: 120,
            result_text: Some("Implemented feature X with 5 tests".to_string()),
            num_turns: Some(53),
            total_cost_usd: Some(2.15),
            permission_denial_count: Some(3),
            permission_denials: Some(vec![
                "Edit".to_string(),
                "Bash".to_string(),
                "Edit".to_string(),
            ]),
        };

        let outcome = build_outcome(&result, 1);
        assert_eq!(outcome.outcome, "Implemented feature X with 5 tests");
        assert_eq!(outcome.num_turns, Some(53));
        assert_eq!(outcome.total_cost_usd, Some(2.15));
        assert_eq!(outcome.permission_denial_count, Some(3));
        assert_eq!(outcome.permission_denials.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_format_exit_code_some() {
        assert_eq!(format_exit_code(Some(0)), "0");
        assert_eq!(format_exit_code(Some(1)), "1");
        assert_eq!(format_exit_code(Some(127)), "127");
    }

    #[test]
    fn test_format_exit_code_none() {
        assert_eq!(format_exit_code(None), "unknown");
    }

    #[test]
    fn test_check_denial_gate_below_threshold_does_not_exit() {
        // Should return normally when denials <= max_denials
        check_denial_gate(0, 10, "coding");
        check_denial_gate(5, 10, "coding");
        check_denial_gate(10, 10, "coding"); // equal is not exceeded
    }

    #[test]
    fn test_available_cycle_names() {
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
"#,
        )
        .unwrap();

        let names = available_cycle_names(&config);
        assert_eq!(names, "coding, gardening");
    }
}
