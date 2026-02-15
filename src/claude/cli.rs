//! Claude Code CLI command builder
//!
//! Constructs `std::process::Command` for invoking Claude Code
//! with the appropriate prompt and permission flags.

use std::process::Command;

/// Build a `Command` to invoke Claude Code with the given prompt and permissions.
///
/// The command uses `-p` for non-interactive prompt execution,
/// `--output-format stream-json` for structured streaming output,
/// and `--allowedTools` for each resolved permission string.
#[must_use]
pub fn build_command(prompt: &str, permissions: &[String]) -> Command {
    let mut cmd = Command::new("claude");

    cmd.arg("-p").arg(prompt);
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
}
