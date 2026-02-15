//! Rich CLI display for cycle execution
//!
//! Renders stream-JSON events as human-readable terminal output.
//! All output goes to stderr so stdout remains clean for piping.

use colored::Colorize;

use crate::claude::stream::StreamEvent;

/// Display handler for cycle execution output
pub struct CycleDisplay {
    cycle_name: String,
}

impl CycleDisplay {
    /// Create a new display handler for the given cycle
    #[must_use]
    pub fn new(cycle_name: &str) -> Self {
        Self {
            cycle_name: cycle_name.to_string(),
        }
    }

    /// Print the cycle header at the start of execution
    pub fn print_header(&self) {
        eprintln!(
            "\n{} {}",
            "===".bold().cyan(),
            format!("Cycle: {}", self.cycle_name).bold().cyan()
        );
        eprintln!("{}", "─".repeat(50).dimmed());
    }

    /// Render a stream event to stderr
    pub fn render_event(&self, event: &StreamEvent) {
        match event {
            StreamEvent::SystemInit { model, .. } => {
                eprintln!("  {} {}", "Model:".dimmed(), model);
            }
            StreamEvent::AssistantText { text } => {
                // Show assistant text, truncated if very long
                let display_text = if text.len() > 200 {
                    format!("{}...", &text[..197])
                } else {
                    text.clone()
                };
                eprintln!("  {display_text}");
            }
            StreamEvent::ToolUse { tool_name, input } => {
                let summary = summarize_tool_input(tool_name, input);
                eprintln!("  {} {}{}", "▶".blue(), tool_name.bold(), summary.dimmed());
            }
            StreamEvent::ToolResult {
                is_error: true,
                content,
            } => {
                let short = if content.len() > 100 {
                    format!("{}...", &content[..97])
                } else {
                    content.clone()
                };
                eprintln!("  {} {}", "✗".red().bold(), short.red());
            }
            StreamEvent::Result {
                is_error,
                result_text,
                num_turns,
                total_cost_usd,
                duration_ms,
                permission_denials,
            } => {
                self.render_result_summary(
                    *is_error,
                    result_text,
                    *num_turns,
                    *total_cost_usd,
                    *duration_ms,
                    permission_denials,
                );
            }
            // Successful tool results and unknown events are not displayed
            StreamEvent::ToolResult { .. } | StreamEvent::Unknown { .. } => {}
        }
    }

    /// Render the post-cycle summary
    fn render_result_summary(
        &self,
        is_error: bool,
        result_text: &str,
        num_turns: u32,
        total_cost_usd: f64,
        duration_ms: u64,
        permission_denials: &[String],
    ) {
        eprintln!("{}", "─".repeat(50).dimmed());

        let status = if is_error {
            "FAILED".red().bold().to_string()
        } else {
            "COMPLETED".green().bold().to_string()
        };
        eprintln!("  {} {}", status, self.cycle_name.bold());

        // Result text (first 200 chars)
        if !result_text.is_empty() {
            let display = if result_text.len() > 200 {
                format!("{}...", &result_text[..197])
            } else {
                result_text.to_string()
            };
            eprintln!("  {display}");
        }

        // Stats line
        let duration_secs = duration_ms / 1000;
        let mins = duration_secs / 60;
        let secs = duration_secs % 60;
        eprintln!(
            "  {} {num_turns} turns | ${total_cost_usd:.2} | {mins}m {secs}s",
            "Stats:".dimmed()
        );

        // Permission denials
        if !permission_denials.is_empty() {
            eprintln!(
                "  {} {} permission denial(s)",
                "⚠".yellow().bold(),
                permission_denials.len()
            );
        }

        eprintln!();
    }
}

/// Summarize tool input as a short one-line string
fn summarize_tool_input(tool_name: &str, input: &serde_json::Value) -> String {
    match tool_name {
        "Edit" | "Read" | "Write" => input
            .get("file_path")
            .and_then(serde_json::Value::as_str)
            .map_or_else(String::new, |p| format!(" {p}")),
        "Bash" => input
            .get("command")
            .and_then(serde_json::Value::as_str)
            .map_or_else(String::new, |c| {
                let short = if c.len() > 60 {
                    format!("{}...", &c[..57])
                } else {
                    c.to_string()
                };
                format!(" `{short}`")
            }),
        "Glob" => input
            .get("pattern")
            .and_then(serde_json::Value::as_str)
            .map_or_else(String::new, |p| format!(" {p}")),
        "Grep" => input
            .get("pattern")
            .and_then(serde_json::Value::as_str)
            .map_or_else(String::new, |p| format!(" /{p}/")),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_display() {
        let display = CycleDisplay::new("coding");
        assert_eq!(display.cycle_name, "coding");
    }

    #[test]
    fn test_summarize_edit_tool() {
        let input = json!({"file_path": "src/main.rs", "old_string": "foo", "new_string": "bar"});
        assert_eq!(summarize_tool_input("Edit", &input), " src/main.rs");
    }

    #[test]
    fn test_summarize_read_tool() {
        let input = json!({"file_path": "Cargo.toml"});
        assert_eq!(summarize_tool_input("Read", &input), " Cargo.toml");
    }

    #[test]
    fn test_summarize_bash_tool() {
        let input = json!({"command": "cargo test --lib"});
        assert_eq!(summarize_tool_input("Bash", &input), " `cargo test --lib`");
    }

    #[test]
    fn test_summarize_bash_long_command_truncated() {
        let long_cmd = "a".repeat(100);
        let input = json!({"command": long_cmd});
        let result = summarize_tool_input("Bash", &input);
        assert!(result.len() < 70);
        assert!(result.ends_with("...`"));
    }

    #[test]
    fn test_summarize_glob_tool() {
        let input = json!({"pattern": "**/*.rs"});
        assert_eq!(summarize_tool_input("Glob", &input), " **/*.rs");
    }

    #[test]
    fn test_summarize_grep_tool() {
        let input = json!({"pattern": "fn main"});
        assert_eq!(summarize_tool_input("Grep", &input), " /fn main/");
    }

    #[test]
    fn test_summarize_unknown_tool() {
        let input = json!({"data": "whatever"});
        assert_eq!(summarize_tool_input("WebSearch", &input), "");
    }

    #[test]
    fn test_summarize_missing_field() {
        let input = json!({});
        assert_eq!(summarize_tool_input("Edit", &input), "");
        assert_eq!(summarize_tool_input("Bash", &input), "");
    }

    // Test that render_event doesn't panic for any event type
    #[test]
    fn test_render_all_event_types_no_panic() {
        let display = CycleDisplay::new("test");

        display.render_event(&StreamEvent::SystemInit {
            model: "claude-opus-4-6".to_string(),
            session_id: "abc".to_string(),
        });
        display.render_event(&StreamEvent::AssistantText {
            text: "Hello".to_string(),
        });
        display.render_event(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: json!({"file_path": "test.rs"}),
        });
        display.render_event(&StreamEvent::ToolResult {
            is_error: false,
            content: "ok".to_string(),
        });
        display.render_event(&StreamEvent::ToolResult {
            is_error: true,
            content: "denied".to_string(),
        });
        display.render_event(&StreamEvent::Result {
            is_error: false,
            result_text: "Done".to_string(),
            num_turns: 5,
            total_cost_usd: 1.23,
            duration_ms: 30000,
            permission_denials: vec![],
        });
        display.render_event(&StreamEvent::Unknown {
            event_type: "other".to_string(),
        });
    }

    #[test]
    fn test_render_long_assistant_text_truncated_no_panic() {
        let display = CycleDisplay::new("test");
        let long_text = "x".repeat(500);
        display.render_event(&StreamEvent::AssistantText { text: long_text });
    }

    #[test]
    fn test_render_result_with_permission_denials_no_panic() {
        let display = CycleDisplay::new("test");
        display.render_event(&StreamEvent::Result {
            is_error: true,
            result_text: "Failed".to_string(),
            num_turns: 10,
            total_cost_usd: 2.50,
            duration_ms: 120000,
            permission_denials: vec!["Edit".to_string(), "Bash".to_string()],
        });
    }
}
