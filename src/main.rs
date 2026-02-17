//! Flow - Automated coding pipeline runner
//!
//! CLI entry point for the Flow orchestrator.

// Allow multiple crate versions from dependencies (can't easily control)
#![allow(clippy::multiple_crate_versions)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

use flow::claude::stream::suggest_permission_fix;
use flow::cli::render_diagnostic_report;
use flow::cycle::config::FlowConfig;
use flow::cycle::executor::CycleExecutor;
use flow::cycle::rules::find_triggered_cycles;
use flow::cycle::selector::select_cycle;
use flow::doctor::diagnose;
use flow::log::jsonl::JsonlLogger;
use flow::log::CycleOutcome;

/// Automated coding pipeline runner
///
/// Orchestrates Claude Code CLI in structured cycles (coding, gardening,
/// review, planning) with controlled permissions and observability.
#[derive(Parser, Debug)]
#[command(name = "flow", version, about)]
struct Cli {
    /// Name of the cycle to execute (shorthand for `flow run --cycle <name>`)
    #[arg(long)]
    cycle: Option<String>,

    /// Path to the cycles.toml configuration file
    #[arg(long, default_value = "cycles.toml")]
    config: PathBuf,

    /// Directory for log files (.flow by default)
    #[arg(long, default_value = ".flow")]
    log_dir: PathBuf,

    /// Maximum number of iterations to run (default: 1)
    #[arg(long, default_value = "1")]
    max_iterations: u32,

    /// Path to TODO.md for cycle selector context (default: TODO.md)
    #[arg(long, default_value = "TODO.md")]
    todo: PathBuf,

    /// Subcommand to run
    #[command(subcommand)]
    command: Option<Command>,
}

/// Available subcommands
#[derive(Subcommand, Debug, PartialEq, Eq)]
enum Command {
    /// Run diagnostics on your Flow configuration and log history
    Doctor,
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

    // Handle subcommands
    if cli.command == Some(Command::Doctor) {
        return run_doctor(&cli);
    }

    // Load configuration
    let config = FlowConfig::from_path(&cli.config)
        .with_context(|| format!("Failed to load config from '{}'", cli.config.display()))?;

    // Determine run mode: fixed cycle or AI-selected
    let fixed_cycle = cli.cycle.clone();
    let use_selector = fixed_cycle.is_none();

    // Validate the requested cycle if specified
    if let Some(ref name) = fixed_cycle {
        config.get_cycle(name).with_context(|| {
            format!(
                "Unknown cycle '{}'. Available cycles: {}",
                name,
                available_cycle_names(&config)
            )
        })?;
    }

    // Require --cycle for single-iteration runs without selector
    if use_selector && cli.max_iterations <= 1 {
        anyhow::bail!(
            "Missing --cycle argument. Usage: flow --cycle <name>, flow --max-iterations N (AI-selected), or flow doctor"
        );
    }

    // Initialize
    let circuit_breaker = config.global.circuit_breaker_repeated;
    let max_denials = config.global.max_permission_denials;
    let executor = CycleExecutor::new(config.clone());
    let logger = JsonlLogger::new(&cli.log_dir).context("Failed to initialize JSONL logger")?;
    let mut iteration: u32 = 1;
    let max_iterations = cli.max_iterations;

    if max_iterations > 1 {
        if use_selector {
            eprintln!(
                "Starting autonomous run: up to {max_iterations} iterations with AI cycle selection"
            );
        } else {
            eprintln!(
                "Starting multi-iteration run: up to {max_iterations} iterations of '{}'",
                fixed_cycle.as_deref().unwrap_or("?")
            );
        }
    }

    // Main iteration loop
    loop {
        if iteration > max_iterations {
            break;
        }

        if max_iterations > 1 {
            eprintln!(
                "\n{} Iteration {iteration}/{max_iterations}",
                ">>>".bold().cyan()
            );
        }

        // Determine which cycle to run
        let cycle_name = if let Some(ref name) = fixed_cycle {
            name.clone()
        } else {
            // AI-driven cycle selection
            let log_entries = logger
                .read_all()
                .context("Failed to read log for selector")?;
            let todo_content = std::fs::read_to_string(&cli.todo).unwrap_or_default();

            eprintln!("{} Selecting next cycle...", ">>>".bold().yellow());
            let selection = select_cycle(&config, &log_entries, &todo_content)
                .await
                .context("Cycle selection failed")?;
            eprintln!(
                "{} Selected '{}': {}",
                ">>>".bold().green(),
                selection.cycle,
                selection.reason
            );
            selection.cycle
        };

        // Execute the selected cycle
        let result = execute_and_log(
            &executor,
            &logger,
            &cycle_name,
            &mut iteration,
            circuit_breaker,
        )
        .await?;

        // Stop on failure
        if !result.success {
            eprintln!(
                "Cycle '{cycle_name}' failed in iteration {}, stopping.",
                iteration - 1
            );
            std::process::exit(result.exit_code.unwrap_or(1));
        }

        // Between-cycle gate: stop if too many permission denials
        check_denial_gate(
            result.permission_denial_count.unwrap_or(0),
            max_denials,
            &result.cycle_name,
        );

        // Auto-trigger dependent cycles
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

    if max_iterations > 1 {
        if use_selector {
            eprintln!("\nCompleted {max_iterations} autonomous iteration(s)");
        } else {
            eprintln!(
                "\nCompleted {max_iterations} iteration(s) of '{}'",
                fixed_cycle.as_deref().unwrap_or("?")
            );
        }
    }

    Ok(())
}

/// Run the `flow doctor` diagnostic command.
fn run_doctor(cli: &Cli) -> Result<()> {
    let config = FlowConfig::from_path(&cli.config)
        .with_context(|| format!("Failed to load config from '{}'", cli.config.display()))?;

    let logger = JsonlLogger::new(&cli.log_dir).context("Failed to initialize JSONL logger")?;
    let log_entries = logger.read_all().unwrap_or_default();

    let report = diagnose(&config, &log_entries);
    let output = render_diagnostic_report(&report);
    eprintln!("{output}");

    if report.error_count() > 0 {
        std::process::exit(1);
    }

    Ok(())
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

    #[test]
    fn test_cli_parses_max_iterations() {
        let cli =
            Cli::try_parse_from(["flow", "--cycle", "coding", "--max-iterations", "5"]).unwrap();
        assert_eq!(cli.max_iterations, 5);
        assert_eq!(cli.cycle.as_deref(), Some("coding"));
    }

    #[test]
    fn test_cli_max_iterations_defaults_to_one() {
        let cli = Cli::try_parse_from(["flow", "--cycle", "coding"]).unwrap();
        assert_eq!(cli.max_iterations, 1);
    }

    #[test]
    fn test_cli_parses_doctor_subcommand() {
        let cli = Cli::try_parse_from(["flow", "doctor"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Doctor)));
        assert!(cli.cycle.is_none());
    }

    #[test]
    fn test_cli_parses_cycle_flag() {
        let cli = Cli::try_parse_from(["flow", "--cycle", "coding"]).unwrap();
        assert_eq!(cli.cycle.as_deref(), Some("coding"));
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parses_todo_flag() {
        let cli =
            Cli::try_parse_from(["flow", "--cycle", "coding", "--todo", "my-todo.md"]).unwrap();
        assert_eq!(cli.todo, PathBuf::from("my-todo.md"));
    }

    #[test]
    fn test_cli_todo_defaults_to_todo_md() {
        let cli = Cli::try_parse_from(["flow", "--cycle", "coding"]).unwrap();
        assert_eq!(cli.todo, PathBuf::from("TODO.md"));
    }

    #[test]
    fn test_cli_max_iterations_without_cycle_is_valid() {
        // When --max-iterations > 1, --cycle is optional (uses selector)
        let cli = Cli::try_parse_from(["flow", "--max-iterations", "10"]).unwrap();
        assert!(cli.cycle.is_none());
        assert_eq!(cli.max_iterations, 10);
    }
}
