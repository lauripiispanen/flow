//! Cycle executor
//!
//! Executes cycles by looking up configuration, resolving permissions,
//! building the Claude Code CLI command, and running it as a subprocess.

use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

use crate::claude::stream::{parse_event, StreamAccumulator, StreamEvent};
use crate::claude::{
    cli::{build_command, build_command_with_session},
    permissions::{resolve_permissions, resolve_step_permissions},
    session::SessionManager,
};
use crate::cli::{CycleDisplay, StatusLine};
use crate::cycle::config::FlowConfig;
use crate::cycle::context::{build_context, inject_context};
use crate::log::jsonl::CycleOutcome;

/// Prepared cycle ready for execution
#[derive(Debug)]
pub struct PreparedCycle {
    /// Cycle name
    pub cycle_name: String,
    /// The prompt to send to Claude Code
    pub prompt: String,
    /// Resolved permissions (global + cycle-specific, deduplicated)
    pub permissions: Vec<String>,
}

/// Result of executing a cycle
#[derive(Debug)]
pub struct CycleResult {
    /// Name of the cycle that was executed
    pub cycle_name: String,
    /// Whether the cycle completed successfully (exit code 0)
    pub success: bool,
    /// Process exit code (None if killed by signal)
    pub exit_code: Option<i32>,
    /// Captured stderr output
    pub stderr: String,
    /// Duration of the cycle in seconds
    pub duration_secs: u64,
    /// Human-readable result summary from Claude's response
    pub result_text: Option<String>,
    /// Number of conversation turns
    pub num_turns: Option<u32>,
    /// Total cost in USD
    pub total_cost_usd: Option<f64>,
    /// Number of permission denials during the cycle
    pub permission_denial_count: Option<u32>,
    /// List of denied tool names (e.g., `["Edit", "Bash"]`)
    pub permission_denials: Option<Vec<String>>,
    /// Files modified during the cycle (deduplicated, from Edit/Write tool uses)
    pub files_changed: Vec<String>,
    /// Total number of tests that passed, parsed from cargo test output in tool results
    pub tests_passed: u32,
}

/// Executes cycles by invoking Claude Code CLI
pub struct CycleExecutor {
    config: FlowConfig,
}

impl CycleExecutor {
    /// Create a new executor with the given configuration
    #[must_use]
    pub const fn new(config: FlowConfig) -> Self {
        Self { config }
    }

    /// Prepare a cycle for execution.
    ///
    /// Validates the cycle exists and resolves effective permissions.
    pub fn prepare(&self, cycle_name: &str) -> Result<PreparedCycle> {
        self.prepare_with_context(cycle_name, &[])
    }

    /// Prepare a cycle for execution with log history context injection.
    ///
    /// Validates the cycle exists, resolves effective permissions, and injects
    /// historical context into the prompt based on the cycle's `context` mode.
    pub fn prepare_with_context(
        &self,
        cycle_name: &str,
        log_entries: &[CycleOutcome],
    ) -> Result<PreparedCycle> {
        let cycle = self
            .config
            .get_cycle(cycle_name)
            .with_context(|| format!("Unknown cycle: '{cycle_name}'"))?;

        let permissions = resolve_permissions(&self.config.global, cycle);
        let context = build_context(&cycle.context, log_entries);
        let prompt = inject_context(&cycle.prompt, context);

        Ok(PreparedCycle {
            cycle_name: cycle_name.to_string(),
            prompt,
            permissions,
        })
    }

    /// Execute a cycle with rich display and stream-JSON parsing.
    ///
    /// For single-step cycles, executes the cycle's top-level prompt directly.
    /// For multi-step cycles, executes each step sequentially, maintaining session
    /// affinity for steps that share the same `session` tag.
    ///
    /// Includes a mid-cycle circuit breaker that kills the subprocess if a tool is
    /// denied `circuit_breaker_threshold` times in a row.
    ///
    /// Log entries are injected into the prompt as context based on the cycle's
    /// `context` mode configuration.
    pub async fn execute_with_display(
        &self,
        cycle_name: &str,
        circuit_breaker_threshold: u32,
        log_entries: &[CycleOutcome],
    ) -> Result<CycleResult> {
        let cycle = self
            .config
            .get_cycle(cycle_name)
            .with_context(|| format!("Unknown cycle: '{cycle_name}'"))?;

        let display = CycleDisplay::new(cycle_name);
        display.print_header();

        if cycle.is_multi_step() {
            self.execute_steps(cycle_name, circuit_breaker_threshold, log_entries, &display)
                .await
        } else {
            self.execute_single_step(cycle_name, circuit_breaker_threshold, log_entries, &display)
                .await
        }
    }

    /// Execute a single-step cycle (existing behavior).
    async fn execute_single_step(
        &self,
        cycle_name: &str,
        circuit_breaker_threshold: u32,
        log_entries: &[CycleOutcome],
        display: &CycleDisplay,
    ) -> Result<CycleResult> {
        let prepared = self.prepare_with_context(cycle_name, log_entries)?;
        let cmd = build_command(&prepared.prompt, &prepared.permissions);
        let mut status_line = StatusLine::new(cycle_name);

        let (accumulator, stderr, exit_code, duration_secs) =
            run_command_with_display(cmd, display, &mut status_line, circuit_breaker_threshold)
                .await?;

        status_line.clear();

        Ok(build_cycle_result(
            prepared.cycle_name,
            exit_code,
            stderr,
            duration_secs,
            &accumulator,
        ))
    }

    /// Execute a multi-step cycle, running each step sequentially.
    ///
    /// Steps sharing the same `session` tag continue the same Claude Code session.
    /// If any step fails (non-zero exit code), execution stops immediately.
    /// The final `CycleResult` aggregates data across all steps.
    async fn execute_steps(
        &self,
        cycle_name: &str,
        circuit_breaker_threshold: u32,
        log_entries: &[CycleOutcome],
        display: &CycleDisplay,
    ) -> Result<CycleResult> {
        let cycle = self
            .config
            .get_cycle(cycle_name)
            .with_context(|| format!("Unknown cycle: '{cycle_name}'"))?;

        let context = build_context(&cycle.context, log_entries);
        let mut session_mgr = SessionManager::new();

        // Aggregated metrics across all steps
        let mut total_duration_secs: u64 = 0;
        let mut total_turns: u32 = 0;
        let mut total_cost: f64 = 0.0;
        let mut total_denials: u32 = 0;
        let mut all_denials: Vec<String> = Vec::new();
        let mut all_files_changed: Vec<String> = Vec::new();
        let mut total_tests_passed: u32 = 0;
        let mut last_result_text: Option<String> = None;
        let mut last_exit_code: Option<i32> = None;
        let mut combined_stderr = String::new();

        for step in &cycle.steps {
            let step_label = format!("{cycle_name}/{}", step.name);
            let mut status_line = StatusLine::new(&step_label);

            // Inject context into the step prompt
            let step_prompt = inject_context(&step.prompt, context.clone());

            // Resolve permissions: global + cycle + step
            let permissions = resolve_step_permissions(&self.config.global, cycle, step);

            // Build command, resuming session if tag has been seen before
            let resume_args = session_mgr.resume_args(step.session.as_deref());
            let cmd = build_command_with_session(&step_prompt, &permissions, &resume_args);

            let (accumulator, stderr, exit_code, duration_secs) =
                run_command_with_display(cmd, display, &mut status_line, circuit_breaker_threshold)
                    .await?;

            status_line.clear();

            // Register the session ID for future steps with the same tag
            if let (Some(tag), Some(sid)) = (&step.session, &accumulator.session_id) {
                session_mgr.register(tag, sid.clone());
            }

            // Aggregate step results
            total_duration_secs += duration_secs;
            if !stderr.is_empty() {
                if !combined_stderr.is_empty() {
                    combined_stderr.push('\n');
                }
                combined_stderr.push_str(&stderr);
            }

            if let Some(StreamEvent::Result {
                result_text,
                num_turns,
                total_cost_usd,
                permission_denials,
                ..
            }) = &accumulator.result
            {
                last_result_text = Some(result_text.clone());
                total_turns = total_turns.saturating_add(*num_turns);
                total_cost += total_cost_usd;
                total_denials = total_denials
                    .saturating_add(u32::try_from(permission_denials.len()).unwrap_or(u32::MAX));
                all_denials.extend(permission_denials.clone());
            }

            // Aggregate files changed across steps (deduplicated)
            for file in &accumulator.files_changed {
                if !all_files_changed.contains(file) {
                    all_files_changed.push(file.clone());
                }
            }

            total_tests_passed = total_tests_passed.saturating_add(accumulator.tests_passed);

            last_exit_code = exit_code;

            // Fail-fast: stop if this step failed
            if exit_code != Some(0) {
                break;
            }
        }

        Ok(CycleResult {
            cycle_name: cycle_name.to_string(),
            success: last_exit_code == Some(0),
            exit_code: last_exit_code,
            stderr: combined_stderr,
            duration_secs: total_duration_secs,
            result_text: last_result_text,
            num_turns: if total_turns > 0 {
                Some(total_turns)
            } else {
                None
            },
            total_cost_usd: if total_cost > 0.0 {
                Some(total_cost)
            } else {
                None
            },
            permission_denial_count: if total_denials > 0 {
                Some(total_denials)
            } else {
                None
            },
            permission_denials: if all_denials.is_empty() {
                None
            } else {
                Some(all_denials)
            },
            files_changed: all_files_changed,
            tests_passed: total_tests_passed,
        })
    }
}

/// Build a `CycleResult` from raw subprocess output and accumulated stream data.
fn build_cycle_result(
    cycle_name: String,
    exit_code: Option<i32>,
    stderr: String,
    duration_secs: u64,
    accumulator: &StreamAccumulator,
) -> CycleResult {
    let (result_text, num_turns, total_cost_usd, denial_count, denials) = match &accumulator.result
    {
        Some(StreamEvent::Result {
            result_text,
            num_turns,
            total_cost_usd,
            permission_denials,
            ..
        }) => (
            Some(result_text.clone()),
            Some(*num_turns),
            Some(*total_cost_usd),
            Some(u32::try_from(permission_denials.len()).unwrap_or(u32::MAX)),
            if permission_denials.is_empty() {
                None
            } else {
                Some(permission_denials.clone())
            },
        ),
        _ => (None, None, None, None, None),
    };

    CycleResult {
        cycle_name,
        success: exit_code == Some(0),
        exit_code,
        stderr,
        duration_secs,
        result_text,
        num_turns,
        total_cost_usd,
        permission_denial_count: denial_count,
        permission_denials: denials,
        files_changed: accumulator.files_changed.clone(),
        tests_passed: accumulator.tests_passed,
    }
}

/// Run a command with stream-JSON parsing and display.
///
/// Parses each stdout line as a stream-JSON event, renders it via the display,
/// and accumulates data. Updates the status line after each event. Implements a
/// circuit breaker that kills the subprocess if a tool is denied `threshold`
/// consecutive times.
///
/// Returns `(accumulator, stderr, exit_code, duration_secs)`.
async fn run_command_with_display(
    cmd: std::process::Command,
    display: &CycleDisplay,
    status_line: &mut StatusLine,
    circuit_breaker_threshold: u32,
) -> Result<(StreamAccumulator, String, Option<i32>, u64)> {
    let mut tokio_cmd = TokioCommand::from(cmd);
    tokio_cmd.stdout(Stdio::piped());
    tokio_cmd.stderr(Stdio::piped());

    let start = Instant::now();

    let mut child = tokio_cmd
        .spawn()
        .context("Failed to spawn Claude Code process")?;

    let child_stdout = child.stdout.take().context("Failed to capture stdout")?;
    let child_stderr = child.stderr.take().context("Failed to capture stderr")?;

    // Read stderr in background
    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(child_stderr);
        let mut lines = reader.lines();
        let mut captured = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    // Process stdout line-by-line with stream-JSON parsing
    let mut accumulator = StreamAccumulator::new();
    let mut consecutive_denials: u32 = 0;
    let mut reader = BufReader::new(child_stdout);
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        let bytes_read = reader.read_line(&mut line_buf).await.unwrap_or(0);
        if bytes_read == 0 {
            break; // EOF or error
        }

        if let Some(event) = parse_event(&line_buf) {
            display.render_event(&event);
            accumulator.process(&event);
            status_line.update(&event);
            status_line.print();

            // Circuit breaker: track consecutive tool errors
            match &event {
                StreamEvent::ToolResult { is_error: true, .. } => {
                    consecutive_denials += 1;
                    if circuit_breaker_threshold > 0
                        && consecutive_denials >= circuit_breaker_threshold
                    {
                        eprintln!(
                            "Circuit breaker: {consecutive_denials} consecutive tool errors, killing subprocess"
                        );
                        let _ = child.kill().await;
                        break;
                    }
                }
                StreamEvent::ToolResult {
                    is_error: false, ..
                }
                | StreamEvent::ToolUse { .. } => {
                    consecutive_denials = 0;
                }
                _ => {}
            }
        }
    }

    let status = child.wait().await.context("Failed waiting for process")?;
    let stderr_result = stderr_handle.await.context("stderr reader panicked")?;
    let duration_secs = start.elapsed().as_secs();

    Ok((accumulator, stderr_result, status.code(), duration_secs))
}

/// Run a command, streaming output to terminal and capturing it.
///
/// Spawns the process with piped stdout/stderr, reads them concurrently,
/// forwards each line to the terminal, and returns the captured output.
///
/// Returns `(stdout, stderr, exit_code, duration_secs)`.
pub async fn run_command(cmd: std::process::Command) -> Result<(String, String, Option<i32>, u64)> {
    let mut tokio_cmd = TokioCommand::from(cmd);
    tokio_cmd.stdout(Stdio::piped());
    tokio_cmd.stderr(Stdio::piped());

    let start = Instant::now();

    let mut child = tokio_cmd
        .spawn()
        .context("Failed to spawn Claude Code process")?;

    // Take ownership of stdout/stderr handles
    let child_stdout = child.stdout.take().context("Failed to capture stdout")?;
    let child_stderr = child.stderr.take().context("Failed to capture stderr")?;

    // Read stdout and stderr concurrently
    let stdout_handle = tokio::spawn(async move {
        let reader = BufReader::new(child_stdout);
        let mut lines = reader.lines();
        let mut captured = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("{line}");
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(child_stderr);
        let mut lines = reader.lines();
        let mut captured = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("{line}");
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    // Wait for process to finish and collect output
    let status = child.wait().await.context("Failed waiting for process")?;
    let stdout_result = stdout_handle.await.context("stdout reader panicked")?;
    let stderr_result = stderr_handle.await.context("stderr reader panicked")?;

    let duration_secs = start.elapsed().as_secs();

    Ok((stdout_result, stderr_result, status.code(), duration_secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::config::FlowConfig;

    const TEST_CONFIG: &str = r#"
[global]
permissions = ["Read", "Edit(./src/**)"]

[[cycle]]
name = "coding"
description = "Pick a task and implement with TDD"
prompt = "You are Flow's coding cycle."
permissions = ["Edit(./tests/**)", "Bash(cargo test *)"]
after = []
context = "summaries"

[[cycle]]
name = "review"
description = "Code review"
prompt = "You are Flow's review cycle."
permissions = []
"#;

    fn test_config() -> FlowConfig {
        FlowConfig::parse(TEST_CONFIG).unwrap()
    }

    #[test]
    fn test_new_creates_executor() {
        let config = test_config();
        let _executor = CycleExecutor::new(config);
    }

    #[test]
    fn test_prepare_rejects_unknown_cycle() {
        let executor = CycleExecutor::new(test_config());
        let result = executor.prepare("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_prepare_returns_cycle_name() {
        let executor = CycleExecutor::new(test_config());
        let prepared = executor.prepare("coding").unwrap();
        assert_eq!(prepared.cycle_name, "coding");
    }

    #[test]
    fn test_prepare_returns_cycle_prompt_with_context_injected() {
        // coding has context = "summaries", so even with empty log the prompt
        // should have the context block prepended
        let executor = CycleExecutor::new(test_config());
        let prepared = executor.prepare("coding").unwrap();
        assert!(
            prepared.prompt.contains("You are Flow's coding cycle."),
            "Original prompt should be present: {}",
            prepared.prompt
        );
        assert!(
            prepared.prompt.contains("Previous Iteration Summaries"),
            "Context header should be prepended: {}",
            prepared.prompt
        );
    }

    #[test]
    fn test_prepare_none_context_returns_raw_prompt() {
        // review has context = "none" (default), so prompt should be unchanged
        let executor = CycleExecutor::new(test_config());
        let prepared = executor.prepare("review").unwrap();
        assert_eq!(prepared.prompt, "You are Flow's review cycle.");
    }

    #[test]
    fn test_prepare_resolves_permissions_merging_global_and_cycle() {
        let executor = CycleExecutor::new(test_config());
        let prepared = executor.prepare("coding").unwrap();
        assert_eq!(
            prepared.permissions,
            vec![
                "Read",
                "Edit(./src/**)",
                "Edit(./tests/**)",
                "Bash(cargo test *)",
            ]
        );
    }

    // --- prepare_with_context tests ---

    use crate::testutil::make_test_outcome as make_outcome;

    #[test]
    fn test_prepare_with_context_injects_summaries() {
        let executor = CycleExecutor::new(test_config());
        let log = vec![make_outcome(1, "review", "Code looked good")];
        let prepared = executor.prepare_with_context("coding", &log).unwrap();
        // coding has context = "summaries"
        assert!(
            prepared.prompt.contains("Code looked good"),
            "Summary outcome should be injected: {}",
            prepared.prompt
        );
        assert!(
            prepared.prompt.contains("You are Flow's coding cycle."),
            "Original prompt must be present: {}",
            prepared.prompt
        );
    }

    #[test]
    fn test_prepare_with_context_none_mode_ignores_log() {
        let executor = CycleExecutor::new(test_config());
        let log = vec![make_outcome(1, "coding", "Implemented something")];
        let prepared = executor.prepare_with_context("review", &log).unwrap();
        // review has context = "none" (default)
        assert_eq!(
            prepared.prompt, "You are Flow's review cycle.",
            "No context should be injected for none mode"
        );
    }

    #[test]
    fn test_prepare_with_context_rejects_unknown_cycle() {
        let executor = CycleExecutor::new(test_config());
        let result = executor.prepare_with_context("nonexistent", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_prepare_review_gets_only_global_permissions() {
        let executor = CycleExecutor::new(test_config());
        let prepared = executor.prepare("review").unwrap();
        assert_eq!(prepared.permissions, vec!["Read", "Edit(./src/**)"]);
    }

    // --- run_command tests (test the subprocess execution directly) ---

    #[tokio::test]
    async fn test_run_command_captures_stdout() {
        let mut cmd = std::process::Command::new("echo");
        cmd.arg("hello world");

        let (stdout, _stderr, exit_code, _duration) = run_command(cmd).await.unwrap();
        assert_eq!(stdout, "hello world");
        assert_eq!(exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_run_command_captures_stderr() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("echo error >&2");

        let (_stdout, stderr, exit_code, _duration) = run_command(cmd).await.unwrap();
        assert_eq!(stderr, "error");
        assert_eq!(exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_run_command_captures_exit_code() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("exit 42");

        let (_stdout, _stderr, exit_code, _duration) = run_command(cmd).await.unwrap();
        assert_eq!(exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_run_command_captures_multiline_stdout() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("echo line1; echo line2; echo line3");

        let (stdout, _stderr, _exit_code, _duration) = run_command(cmd).await.unwrap();
        assert_eq!(stdout, "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_run_command_reports_failure() {
        let cmd = std::process::Command::new("false");

        let (_stdout, _stderr, exit_code, _duration) = run_command(cmd).await.unwrap();
        assert_eq!(exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_run_command_tracks_duration() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("sleep 0.01");

        let (_stdout, _stderr, _exit_code, duration) = run_command(cmd).await.unwrap();
        // Duration is in seconds, sleep 0.01 should round to 0
        assert!(duration < 5, "Expected fast execution, got {duration}s");
    }

    #[test]
    fn test_cycle_result_optional_fields_default_to_none() {
        let result = CycleResult {
            cycle_name: "test".to_string(),
            success: true,
            exit_code: Some(0),
            stderr: String::new(),
            duration_secs: 0,
            result_text: None,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            files_changed: vec![],
            tests_passed: 0,
        };
        assert!(result.result_text.is_none());
        assert!(result.num_turns.is_none());
        assert!(result.total_cost_usd.is_none());
        assert!(result.permission_denial_count.is_none());
        assert!(result.files_changed.is_empty());
        assert_eq!(result.tests_passed, 0);
    }

    #[test]
    fn test_cycle_result_optional_fields_with_values() {
        let result = CycleResult {
            cycle_name: "coding".to_string(),
            success: true,
            exit_code: Some(0),
            stderr: String::new(),
            duration_secs: 120,
            result_text: Some("Implemented feature X".to_string()),
            num_turns: Some(53),
            total_cost_usd: Some(2.15),
            permission_denial_count: Some(3),
            permission_denials: Some(vec![
                "Edit".to_string(),
                "Bash".to_string(),
                "Edit".to_string(),
            ]),
            files_changed: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            tests_passed: 42,
        };
        assert_eq!(result.result_text.as_deref(), Some("Implemented feature X"));
        assert_eq!(result.num_turns, Some(53));
        assert_eq!(result.total_cost_usd, Some(2.15));
        assert_eq!(result.permission_denial_count, Some(3));
        assert_eq!(result.permission_denials.as_ref().unwrap().len(), 3);
        assert_eq!(result.files_changed, vec!["src/main.rs", "src/lib.rs"]);
        assert_eq!(result.tests_passed, 42);
    }

    // --- multi-step cycle config tests ---

    const MULTI_STEP_CONFIG: &str = r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "coding"
description = "Multi-step coding cycle"
after = []
context = "none"

[[cycle.step]]
name = "plan"
session = "architect"
prompt = "Read TODO.md and write a plan."
permissions = ["Edit(./.flow/current-plan.md)"]

[[cycle.step]]
name = "implement"
session = "coder"
prompt = "Read the plan and implement it."
permissions = ["Edit(./src/**)", "Bash(cargo *)"]

[[cycle.step]]
name = "review"
session = "architect"
prompt = "Review the implementation."
"#;

    fn multi_step_config() -> FlowConfig {
        FlowConfig::parse(MULTI_STEP_CONFIG).unwrap()
    }

    #[test]
    fn test_resolve_step_permissions_merges_global_cycle_and_step() {
        let config = multi_step_config();
        let cycle = config.get_cycle("coding").unwrap();
        let plan_step = &cycle.steps[0];
        let resolved = resolve_step_permissions(&config.global, cycle, plan_step);
        // global: Read | cycle: (none) | step: Edit(./.flow/current-plan.md)
        assert_eq!(resolved, vec!["Read", "Edit(./.flow/current-plan.md)"]);
    }

    #[test]
    fn test_resolve_step_permissions_deduplicates() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "implement"
prompt = "Implement."
permissions = ["Read", "Edit(./src/**)"]
"#,
        )
        .unwrap();
        let cycle = config.get_cycle("coding").unwrap();
        let step = &cycle.steps[0];
        let resolved = resolve_step_permissions(&config.global, cycle, step);
        // "Read" from global, "Read" from step deduped, only "Edit(./src/**)" added
        assert_eq!(resolved, vec!["Read", "Edit(./src/**)"]);
    }

    #[test]
    fn test_resolve_step_permissions_additive_on_cycle_perms() {
        let config = FlowConfig::parse(
            r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "coding"
description = "Coding"
after = []
permissions = ["Glob"]

[[cycle.step]]
name = "implement"
prompt = "Implement."
permissions = ["Edit(./src/**)"]
"#,
        )
        .unwrap();
        let cycle = config.get_cycle("coding").unwrap();
        let step = &cycle.steps[0];
        let resolved = resolve_step_permissions(&config.global, cycle, step);
        assert_eq!(resolved, vec!["Read", "Glob", "Edit(./src/**)"]);
    }

    #[test]
    fn test_is_multi_step_cycle_true_for_multi_step() {
        let config = multi_step_config();
        let cycle = config.get_cycle("coding").unwrap();
        assert!(cycle.is_multi_step());
    }

    #[test]
    fn test_is_multi_step_cycle_false_for_single_step() {
        let config = test_config();
        let cycle = config.get_cycle("coding").unwrap();
        assert!(!cycle.is_multi_step());
    }

    // --- run_command_with_display tests ---

    #[tokio::test]
    async fn test_run_command_with_display_parses_stream_json() {
        let display = CycleDisplay::new("test");
        let mut status_line = StatusLine::new("test");
        let stream_json = r#"{"type":"system","subtype":"init","model":"claude-opus-4-6","session_id":"abc"}
{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}
{"type":"result","subtype":"success","is_error":false,"num_turns":3,"result":"Done","total_cost_usd":1.50,"duration_ms":5000,"permission_denials":[]}"#;

        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(format!("printf '%s\\n' '{stream_json}'"));

        // Use echo with the JSON lines
        let mut cmd2 = std::process::Command::new("echo");
        cmd2.arg(stream_json);

        let (acc, _stderr, exit_code, _duration) =
            run_command_with_display(cmd2, &display, &mut status_line, 5)
                .await
                .unwrap();

        assert_eq!(exit_code, Some(0));
        assert!(acc.result.is_some());
    }

    #[tokio::test]
    async fn test_run_command_with_display_captures_result_fields() {
        let display = CycleDisplay::new("test");
        let mut status_line = StatusLine::new("test");
        let line = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":10,"result":"Task completed","total_cost_usd":2.50,"duration_ms":30000,"permission_denials":["Edit"]}"#;

        let mut cmd = std::process::Command::new("echo");
        cmd.arg(line);

        let (acc, _stderr, _exit_code, _duration) =
            run_command_with_display(cmd, &display, &mut status_line, 5)
                .await
                .unwrap();

        assert_eq!(acc.permission_denial_count(), 1);
        let Some(StreamEvent::Result {
            num_turns,
            total_cost_usd,
            result_text,
            ..
        }) = &acc.result
        else {
            panic!("Expected Result event, got {:?}", acc.result);
        };
        assert_eq!(*num_turns, 10);
        assert!((total_cost_usd - 2.50).abs() < f64::EPSILON);
        assert_eq!(result_text, "Task completed");
    }

    #[tokio::test]
    async fn test_run_command_with_display_captures_files_changed() {
        let display = CycleDisplay::new("test");
        let mut status_line = StatusLine::new("test");
        // Simulate Edit and Write tool uses followed by a result
        let lines = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/main.rs"}}]}}
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":"src/lib.rs"}}]}}
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/main.rs"}}]}}
{"type":"result","subtype":"success","is_error":false,"num_turns":3,"result":"Done","total_cost_usd":1.0,"duration_ms":1000,"permission_denials":[]}"#;

        let mut cmd = std::process::Command::new("echo");
        cmd.arg(lines);

        let (acc, _stderr, _exit_code, _duration) =
            run_command_with_display(cmd, &display, &mut status_line, 5)
                .await
                .unwrap();

        // src/main.rs appears twice but should be deduplicated
        assert_eq!(acc.files_changed, vec!["src/main.rs", "src/lib.rs"]);
    }

    // --- build_cycle_result tests ---

    #[test]
    fn test_build_cycle_result_with_result_event() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
        });
        acc.process(&StreamEvent::ToolResult {
            is_error: false,
            content: "test result: ok. 10 passed; 0 failed; 0 ignored".to_string(),
        });
        acc.process(&StreamEvent::Result {
            is_error: false,
            result_text: "Implemented feature".to_string(),
            num_turns: 5,
            total_cost_usd: 1.23,
            duration_ms: 30000,
            permission_denials: vec!["Bash".to_string()],
        });

        let result = build_cycle_result("coding".to_string(), Some(0), String::new(), 120, &acc);

        assert_eq!(result.cycle_name, "coding");
        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.duration_secs, 120);
        assert_eq!(result.result_text.as_deref(), Some("Implemented feature"));
        assert_eq!(result.num_turns, Some(5));
        assert_eq!(result.total_cost_usd, Some(1.23));
        assert_eq!(result.permission_denial_count, Some(1));
        assert_eq!(result.permission_denials, Some(vec!["Bash".to_string()]));
        assert_eq!(result.files_changed, vec!["src/main.rs"]);
        assert_eq!(result.tests_passed, 10);
    }

    #[test]
    fn test_build_cycle_result_without_result_event() {
        let acc = StreamAccumulator::new();
        let result = build_cycle_result(
            "coding".to_string(),
            Some(1),
            "error output".to_string(),
            30,
            &acc,
        );

        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(result.stderr, "error output");
        assert!(result.result_text.is_none());
        assert!(result.num_turns.is_none());
        assert!(result.total_cost_usd.is_none());
        assert!(result.permission_denial_count.is_none());
        assert!(result.permission_denials.is_none());
        assert!(result.files_changed.is_empty());
        assert_eq!(result.tests_passed, 0);
    }

    #[test]
    fn test_build_cycle_result_empty_denials_become_none() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::Result {
            is_error: false,
            result_text: "Done".to_string(),
            num_turns: 3,
            total_cost_usd: 0.50,
            duration_ms: 10000,
            permission_denials: vec![],
        });

        let result = build_cycle_result("review".to_string(), Some(0), String::new(), 10, &acc);

        assert!(result.permission_denials.is_none());
        assert_eq!(result.permission_denial_count, Some(0));
    }
}
