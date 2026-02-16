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
use crate::claude::{cli::build_command, permissions::resolve_permissions};
use crate::cli::{CycleDisplay, StatusLine};
use crate::cycle::config::FlowConfig;

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
    /// Captured stdout output
    pub stdout: String,
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
        let cycle = self
            .config
            .get_cycle(cycle_name)
            .with_context(|| format!("Unknown cycle: '{cycle_name}'"))?;

        let permissions = resolve_permissions(&self.config.global, cycle);

        Ok(PreparedCycle {
            cycle_name: cycle_name.to_string(),
            prompt: cycle.prompt.clone(),
            permissions,
        })
    }

    /// Execute a cycle with rich display and stream-JSON parsing.
    ///
    /// Parses stream-JSON events for real-time display and populates rich
    /// fields (turns, cost, denials) from the result blob. Includes a
    /// mid-cycle circuit breaker that kills the subprocess if a tool is
    /// denied `circuit_breaker_threshold` times in a row.
    pub async fn execute_with_display(
        &self,
        cycle_name: &str,
        circuit_breaker_threshold: u32,
    ) -> Result<CycleResult> {
        let prepared = self.prepare(cycle_name)?;
        let cmd = build_command(&prepared.prompt, &prepared.permissions);
        let display = CycleDisplay::new(cycle_name);

        display.print_header();
        let mut status_line = StatusLine::new(cycle_name);

        let (accumulator, stderr, exit_code, duration_secs) =
            run_command_with_display(cmd, &display, &mut status_line, circuit_breaker_threshold)
                .await?;

        status_line.clear();

        // Extract rich fields from the accumulated result
        let (result_text, num_turns, total_cost_usd, denial_count) = match &accumulator.result {
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
            ),
            _ => (None, None, None, None),
        };

        Ok(CycleResult {
            cycle_name: prepared.cycle_name,
            success: exit_code == Some(0),
            exit_code,
            stdout: String::new(), // Not captured in display mode
            stderr,
            duration_secs,
            result_text,
            num_turns,
            total_cost_usd,
            permission_denial_count: denial_count,
        })
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
    fn test_prepare_returns_cycle_prompt() {
        let executor = CycleExecutor::new(test_config());
        let prepared = executor.prepare("coding").unwrap();
        assert_eq!(prepared.prompt, "You are Flow's coding cycle.");
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
            stdout: String::new(),
            stderr: String::new(),
            duration_secs: 0,
            result_text: None,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
        };
        assert!(result.result_text.is_none());
        assert!(result.num_turns.is_none());
        assert!(result.total_cost_usd.is_none());
        assert!(result.permission_denial_count.is_none());
    }

    #[test]
    fn test_cycle_result_optional_fields_with_values() {
        let result = CycleResult {
            cycle_name: "coding".to_string(),
            success: true,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration_secs: 120,
            result_text: Some("Implemented feature X".to_string()),
            num_turns: Some(53),
            total_cost_usd: Some(2.15),
            permission_denial_count: Some(3),
        };
        assert_eq!(result.result_text.as_deref(), Some("Implemented feature X"));
        assert_eq!(result.num_turns, Some(53));
        assert_eq!(result.total_cost_usd, Some(2.15));
        assert_eq!(result.permission_denial_count, Some(3));
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
}
