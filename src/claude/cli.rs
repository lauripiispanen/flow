//! Claude Code CLI command builder
//!
//! Constructs `std::process::Command` for invoking Claude Code
//! with the appropriate prompt and permission flags.

use std::process::Command;

/// Build a `Command` to invoke Claude Code with the given prompt and permissions.
///
/// The command uses `-p` for non-interactive prompt execution,
/// `--verbose` and `--output-format stream-json` for structured streaming output,
/// and `--allowedTools` for each resolved permission string.
#[must_use]
pub fn build_command(prompt: &str, permissions: &[String]) -> Command {
    build_command_with_session(prompt, permissions, &[])
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
    let mut cmd = Command::new("claude");

    for arg in resume_args {
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

    cmd
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
}
