//! Cycle executor
//!
//! Executes cycles by looking up configuration, resolving permissions,
//! building the Claude Code CLI command, and running it as a subprocess.

use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

use crate::claude::{cli::build_command, permissions::resolve_permissions};
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

    /// Execute a cycle end-to-end.
    ///
    /// Prepares the cycle, spawns the Claude Code subprocess,
    /// streams output to terminal, and captures the result.
    pub async fn execute(&self, cycle_name: &str) -> Result<CycleResult> {
        let prepared = self.prepare(cycle_name)?;
        let cmd = build_command(&prepared.prompt, &prepared.permissions);

        let (stdout, stderr, exit_code, duration_secs) = run_command(cmd).await?;

        Ok(CycleResult {
            cycle_name: prepared.cycle_name,
            success: exit_code == Some(0),
            exit_code,
            stdout,
            stderr,
            duration_secs,
        })
    }
}

/// Run a command, streaming output to terminal and capturing it.
///
/// Spawns the process with piped stdout/stderr, reads them concurrently,
/// forwards each line to the terminal, and returns the captured output.
async fn run_command(cmd: std::process::Command) -> Result<(String, String, Option<i32>, u64)> {
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
}
