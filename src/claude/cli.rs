//! Claude Code CLI command builder
//!
//! Constructs `std::process::Command` for invoking Claude Code
//! with the appropriate prompt and permission flags. Also provides
//! `run_for_result` to spawn a command and collect the final result text.

use anyhow::{bail, Context, Result};
use std::process::Command;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command as TokioCommand;

use super::stream::{parse_event, StreamAccumulator, StreamEvent};

/// Options for building a Claude Code command beyond prompt and permissions.
#[derive(Debug, Clone, Default)]
pub struct CommandOptions {
    /// Resume args (e.g., `["--resume", "<session_id>"]`). Empty means new session.
    pub resume_args: Vec<String>,
    /// Maximum agentic turns (maps to `--max-turns`).
    pub max_turns: Option<u32>,
    /// Maximum cost in USD (maps to `--max-budget-usd`).
    pub max_cost_usd: Option<f64>,
}

/// Build a `Command` to invoke Claude Code with the given prompt and permissions.
///
/// The command uses `-p` for non-interactive prompt execution,
/// `--verbose` and `--output-format stream-json` for structured streaming output,
/// and `--allowedTools` for each resolved permission string.
#[must_use]
pub fn build_command(prompt: &str, permissions: &[String]) -> Command {
    build_command_with_options(prompt, permissions, &CommandOptions::default())
}

/// Build a `Command` to invoke Claude Code, optionally resuming an existing session.
///
/// Like `build_command` but prepends `resume_args` (e.g., `["--resume", "<session_id>"]`)
/// to continue a prior Claude Code session. An empty `resume_args` slice produces the
/// same command as `build_command`.
#[must_use]
pub fn build_command_with_session(
    prompt: &str,
    permissions: &[String],
    resume_args: &[String],
) -> Command {
    build_command_with_options(
        prompt,
        permissions,
        &CommandOptions {
            resume_args: resume_args.to_vec(),
            ..Default::default()
        },
    )
}

/// Build a `Command` to invoke Claude Code with full options.
///
/// This is the core builder that all other `build_command*` functions delegate to.
/// Supports resume args, max turns, and max budget in addition to prompt and permissions.
#[must_use]
pub fn build_command_with_options(
    prompt: &str,
    permissions: &[String],
    options: &CommandOptions,
) -> Command {
    let mut cmd = Command::new("claude");

    for arg in &options.resume_args {
        cmd.arg(arg);
    }

    cmd.arg("-p").arg(prompt);
    cmd.arg("--verbose");
    cmd.arg("--output-format").arg("stream-json");

    if !permissions.is_empty() {
        cmd.arg("--allowedTools");
        for perm in permissions {
            cmd.arg(perm);
        }
    }

    if let Some(max_turns) = options.max_turns {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    if let Some(max_cost) = options.max_cost_usd {
        cmd.arg("--max-budget-usd").arg(max_cost.to_string());
    }

    cmd
}

/// Spawn a Claude Code command, stream-parse the output, and return the result text.
///
/// Used by the cycle selector and step router — both invoke Claude with no tool
/// permissions and only need the final result text from the response.
pub async fn run_for_result(cmd: Command) -> Result<String> {
    let mut child = TokioCommand::from(cmd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn claude")?;

    let stdout = child.stdout.take().context("No stdout from claude")?;
    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut accumulator = StreamAccumulator::new();

    while let Some(line) = lines
        .next_line()
        .await
        .context("Failed to read claude output")?
    {
        if let Some(event) = parse_event(&line) {
            accumulator.process(&event);
            if matches!(event, StreamEvent::Result { .. }) {
                break;
            }
        }
    }

    let _ = child.wait().await;

    let result_text = match &accumulator.result {
        Some(StreamEvent::Result { result_text, .. }) => result_text.clone(),
        _ => accumulator.text_fragments.join(""),
    };

    if result_text.is_empty() {
        bail!("Claude returned empty response");
    }

    Ok(result_text)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_build_sets_claude_binary() {
        let cmd = super::build_command("Do something", &[]);
        let program = cmd.get_program().to_str().unwrap();
        assert_eq!(program, "claude");
    }

    #[test]
    fn test_build_passes_prompt_with_print_flag() {
        let cmd = super::build_command("Fix the bug", &[]);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        assert!(args.contains(&"-p"), "Expected -p flag, got: {args:?}");
        assert!(
            args.contains(&"Fix the bug"),
            "Expected prompt in args, got: {args:?}"
        );
    }

    #[test]
    fn test_build_with_permissions_adds_allowed_tools() {
        let perms = vec!["Read".to_string(), "Edit(./src/**)".to_string()];
        let cmd = super::build_command("Code", &perms);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        assert!(
            args.contains(&"--allowedTools"),
            "Expected --allowedTools flag, got: {args:?}"
        );
        assert!(
            args.contains(&"Read"),
            "Expected 'Read' permission, got: {args:?}"
        );
        assert!(
            args.contains(&"Edit(./src/**)"),
            "Expected 'Edit(./src/**)' permission, got: {args:?}"
        );
    }

    #[test]
    fn test_build_without_permissions_omits_allowed_tools() {
        let cmd = super::build_command("Code", &[]);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        assert!(
            !args.contains(&"--allowedTools"),
            "Should not include --allowedTools when no permissions, got: {args:?}"
        );
    }

    #[test]
    fn test_build_args_order_is_prompt_then_tools() {
        let perms = vec!["Read".to_string()];
        let cmd = super::build_command("My prompt", &perms);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        let p_pos = args.iter().position(|a| *a == "-p").unwrap();
        let tools_pos = args.iter().position(|a| *a == "--allowedTools").unwrap();
        assert!(
            p_pos < tools_pos,
            "Expected -p before --allowedTools, got: {args:?}"
        );
    }

    #[test]
    fn test_build_with_multiple_permissions() {
        let perms = vec![
            "Read".to_string(),
            "Edit(./src/**)".to_string(),
            "Bash(cargo test *)".to_string(),
        ];
        let cmd = super::build_command("Test", &perms);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        // Each permission should be a separate arg after --allowedTools
        let tools_pos = args.iter().position(|a| *a == "--allowedTools").unwrap();
        assert_eq!(args[tools_pos + 1], "Read");
        assert_eq!(args[tools_pos + 2], "Edit(./src/**)");
        assert_eq!(args[tools_pos + 3], "Bash(cargo test *)");
    }

    #[test]
    fn test_build_sets_json_output_format() {
        let cmd = super::build_command("Code", &[]);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        assert!(
            args.contains(&"--output-format"),
            "Expected --output-format flag, got: {args:?}"
        );
        assert!(
            args.contains(&"stream-json"),
            "Expected stream-json output format, got: {args:?}"
        );
    }

    #[test]
    fn test_build_includes_verbose_flag() {
        let cmd = super::build_command("Code", &[]);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        assert!(
            args.contains(&"--verbose"),
            "Expected --verbose flag (required for stream-json with -p), got: {args:?}"
        );
    }

    #[test]
    fn test_build_with_resume_args_includes_resume_flag() {
        let resume = vec!["--resume".to_string(), "abc-123".to_string()];
        let cmd = super::build_command_with_session("Code", &[], &resume);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

        assert!(
            args.contains(&"--resume"),
            "Expected --resume flag, got: {args:?}"
        );
        assert!(
            args.contains(&"abc-123"),
            "Expected session ID, got: {args:?}"
        );
    }

    #[test]
    fn test_build_with_empty_resume_args_behaves_like_build_command() {
        let cmd1 = super::build_command("Code", &["Read".to_string()]);
        let cmd2 = super::build_command_with_session("Code", &["Read".to_string()], &[]);
        let args1: Vec<&str> = cmd1.get_args().map(|a| a.to_str().unwrap()).collect();
        let args2: Vec<&str> = cmd2.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(args1, args2);
    }

    // --- CommandOptions tests ---

    #[test]
    fn test_command_options_default_has_no_limits() {
        let opts = super::CommandOptions::default();
        assert!(opts.resume_args.is_empty());
        assert!(opts.max_turns.is_none());
        assert!(opts.max_cost_usd.is_none());
    }

    #[test]
    fn test_build_with_max_turns_adds_flag() {
        let opts = super::CommandOptions {
            max_turns: Some(50),
            ..Default::default()
        };
        let cmd = super::build_command_with_options("Code", &[], &opts);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert!(
            args.contains(&"--max-turns"),
            "Expected --max-turns flag, got: {args:?}"
        );
        assert!(
            args.contains(&"50"),
            "Expected max-turns value '50', got: {args:?}"
        );
    }

    #[test]
    fn test_build_with_max_cost_adds_flag() {
        let opts = super::CommandOptions {
            max_cost_usd: Some(5.0),
            ..Default::default()
        };
        let cmd = super::build_command_with_options("Code", &[], &opts);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert!(
            args.contains(&"--max-budget-usd"),
            "Expected --max-budget-usd flag, got: {args:?}"
        );
        assert!(
            args.contains(&"5"),
            "Expected max-budget-usd value '5', got: {args:?}"
        );
    }

    #[test]
    fn test_build_with_both_limits() {
        let opts = super::CommandOptions {
            max_turns: Some(100),
            max_cost_usd: Some(10.0),
            ..Default::default()
        };
        let cmd = super::build_command_with_options("Code", &[], &opts);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert!(args.contains(&"--max-turns"));
        assert!(args.contains(&"100"));
        assert!(args.contains(&"--max-budget-usd"));
        assert!(args.contains(&"10"));
    }

    #[test]
    fn test_build_with_resume_and_limits() {
        let opts = super::CommandOptions {
            resume_args: vec!["--resume".to_string(), "abc-123".to_string()],
            max_turns: Some(30),
            max_cost_usd: Some(2.5),
        };
        let cmd = super::build_command_with_options("Code", &[], &opts);
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert!(args.contains(&"--resume"));
        assert!(args.contains(&"abc-123"));
        assert!(args.contains(&"--max-turns"));
        assert!(args.contains(&"30"));
        assert!(args.contains(&"--max-budget-usd"));
    }

    #[test]
    fn test_build_command_delegates_to_default_options() {
        let cmd1 = super::build_command("Code", &["Read".to_string()]);
        let cmd2 = super::build_command_with_options(
            "Code",
            &["Read".to_string()],
            &super::CommandOptions::default(),
        );
        let args1: Vec<&str> = cmd1.get_args().map(|a| a.to_str().unwrap()).collect();
        let args2: Vec<&str> = cmd2.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(args1, args2);
    }
}
