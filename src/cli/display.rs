//! Rich CLI display for cycle execution
//!
//! Renders stream-JSON events as human-readable terminal output.
//! All output goes to stderr so stdout remains clean for piping.

use colored::Colorize;

use crate::claude::stream::StreamEvent;

/// Truncate a string to at most `max_chars` Unicode characters, appending "..." if truncated.
fn truncate(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let collected: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{collected}...")
    } else {
        collected
    }
}

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
                eprintln!("  {}", truncate(text, 500));
            }
            StreamEvent::ToolUse { tool_name, input } => {
                let summary = summarize_tool_input(tool_name, input);
                eprintln!("  {} {}{}", "▶".blue(), tool_name.bold(), summary.dimmed());
            }
            StreamEvent::ToolResult {
                is_error: true,
                content,
            } => {
                eprintln!("  {} {}", "✗".red().bold(), truncate(content, 200).red());
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

        if !result_text.is_empty() {
            eprintln!("  {}", truncate(result_text, 500));
        }

        // Stats line
        let duration = format_duration(duration_ms / 1000);
        eprintln!(
            "  {} {num_turns} turns | ${total_cost_usd:.2} | {duration}",
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
            .map_or_else(String::new, |c| format!(" `{}`", truncate(c, 80))),
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

/// Render a diagnostic report as a human-readable string.
///
/// Formats findings by severity with codes, messages, and suggestions.
/// Returns a summary line at the end with counts.
#[must_use]
pub fn render_diagnostic_report(report: &crate::doctor::DiagnosticReport) -> String {
    use crate::doctor::Severity;

    if report.is_clean() {
        return "No issues found. Your Flow configuration looks healthy.".to_string();
    }

    let mut lines = Vec::new();

    for finding in &report.findings {
        let prefix = match finding.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN ",
            Severity::Info => "INFO ",
        };
        lines.push(format!("[{prefix}] {}: {}", finding.code, finding.message));
        if let Some(ref suggestion) = finding.suggestion {
            lines.push(format!("       Fix: {suggestion}"));
        }
    }

    // Summary line
    let errors = report.error_count();
    let warnings = report.warning_count();
    let infos = report.info_count();
    let mut summary_parts = Vec::new();
    if errors > 0 {
        summary_parts.push(format!(
            "{errors} error{}",
            if errors == 1 { "" } else { "s" }
        ));
    }
    if warnings > 0 {
        summary_parts.push(format!(
            "{warnings} warning{}",
            if warnings == 1 { "" } else { "s" }
        ));
    }
    if infos > 0 {
        summary_parts.push(format!("{infos} info"));
    }
    lines.push(String::new());
    lines.push(format!("Summary: {}", summary_parts.join(", ")));

    lines.join("\n")
}

/// Format a duration in seconds as a human-readable string (e.g. "2m 15s", "30s", "5m").
pub(crate) fn format_duration(secs: u64) -> String {
    let mins = secs / 60;
    let secs = secs % 60;
    if mins == 0 {
        format!("{secs}s")
    } else if secs == 0 {
        format!("{mins}m")
    } else {
        format!("{mins}m {secs}s")
    }
}

/// Render a periodic run summary as a compact multi-line block.
///
/// Displayed every N iterations during multi-iteration runs to give users
/// an aggregated view of progress, cost, cycle mix, and success rate.
#[must_use]
pub fn render_run_summary(
    iteration: u32,
    max_iterations: u32,
    total_cost_usd: f64,
    cycles: &std::collections::BTreeMap<String, u32>,
    successes: u32,
    failures: u32,
    duration_secs: u64,
) -> String {
    let total = successes + failures;
    let cycle_parts: Vec<String> = cycles
        .iter()
        .map(|(name, count)| format!("{name}\u{00d7}{count}"))
        .collect();
    let cycles_str = cycle_parts.join(", ");

    format!(
        "\u{2500}\u{2500}\u{2500} Run Summary ({iteration}/{max_iterations}) \u{2500}\u{2500}\u{2500}\n\
         Cycles: {cycles_str}\n\
         Results: {successes}/{total} succeeded\n\
         Cost: ${total_cost_usd:.2} | Duration: {}\n\
         \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        format_duration(duration_secs)
    )
}

/// Health color for the status bar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HealthColor {
    /// Healthy: 0 errors
    Green,
    /// Warning: 1-2 errors
    Yellow,
    /// Critical: 3+ errors
    Red,
}

/// Live status bar displayed at the bottom of the terminal during cycle execution.
///
/// Tracks turn count, cost, elapsed time, and error count from stream events.
/// Renders a single ANSI-formatted line using save/restore cursor positioning.
pub struct StatusLine {
    cycle_name: String,
    turn_count: u32,
    cost_usd: f64,
    error_count: u32,
    start: std::time::Instant,
    /// Optional `(current_iteration, max_iterations)` for multi-iteration runs.
    iteration_context: Option<(u32, u32)>,
}

impl StatusLine {
    /// Create a new status line for the given cycle
    #[must_use]
    pub fn new(cycle_name: &str) -> Self {
        Self {
            cycle_name: cycle_name.to_string(),
            turn_count: 0,
            cost_usd: 0.0,
            error_count: 0,
            start: std::time::Instant::now(),
            iteration_context: None,
        }
    }

    /// Create a status line with iteration context for multi-iteration runs.
    ///
    /// When `max > 1`, renders a `[current/max] ` prefix before the cycle name.
    #[must_use]
    pub fn with_iteration(cycle_name: &str, current: u32, max: u32) -> Self {
        Self {
            cycle_name: cycle_name.to_string(),
            turn_count: 0,
            cost_usd: 0.0,
            error_count: 0,
            start: std::time::Instant::now(),
            iteration_context: Some((current, max)),
        }
    }

    /// Create a status line with a specific start time (for testing)
    #[cfg(test)]
    fn with_start(cycle_name: &str, start: std::time::Instant) -> Self {
        Self {
            cycle_name: cycle_name.to_string(),
            turn_count: 0,
            cost_usd: 0.0,
            error_count: 0,
            start,
            iteration_context: None,
        }
    }

    /// Create a status line with iteration context and a specific start time (for testing)
    #[cfg(test)]
    fn with_iteration_and_start(
        cycle_name: &str,
        current: u32,
        max: u32,
        start: std::time::Instant,
    ) -> Self {
        Self {
            cycle_name: cycle_name.to_string(),
            turn_count: 0,
            cost_usd: 0.0,
            error_count: 0,
            start,
            iteration_context: Some((current, max)),
        }
    }

    /// Update the status line from a stream event
    pub const fn update(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::ToolUse { .. } => {
                self.turn_count += 1;
            }
            StreamEvent::ToolResult { is_error: true, .. } => {
                self.error_count += 1;
            }
            StreamEvent::Result {
                num_turns,
                total_cost_usd,
                ..
            } => {
                self.turn_count = *num_turns;
                self.cost_usd = *total_cost_usd;
            }
            _ => {}
        }
    }

    /// Render the status line content (without ANSI cursor positioning).
    ///
    /// Returns the formatted string like: `[coding] ▶ 12 turns | $1.23 | 2m 15s | 0 errors`
    #[must_use]
    pub fn render(&self) -> String {
        let elapsed = self.start.elapsed().as_secs();
        let mins = elapsed / 60;
        let secs = elapsed % 60;
        let prefix = match self.iteration_context {
            Some((current, max)) if max > 1 => format!("[{current}/{max}] "),
            _ => String::new(),
        };
        format!(
            "{prefix}[{}] \u{25b6} {} turns | ${:.2} | {}m {:02}s | {} errors",
            self.cycle_name, self.turn_count, self.cost_usd, mins, secs, self.error_count
        )
    }

    /// Determine the health level based on error count.
    ///
    /// Returns a color indicator: green (0 errors), yellow (1-2), red (3+).
    #[must_use]
    const fn health_color(&self) -> HealthColor {
        match self.error_count {
            0 => HealthColor::Green,
            1..=2 => HealthColor::Yellow,
            _ => HealthColor::Red,
        }
    }

    /// Render the status line with color-coded health.
    ///
    /// The entire line is colored based on error count:
    /// green (healthy), yellow (warning), red (critical).
    #[must_use]
    pub fn render_colored(&self) -> String {
        let content = self.render();
        match self.health_color() {
            HealthColor::Green => content.green().to_string(),
            HealthColor::Yellow => content.yellow().to_string(),
            HealthColor::Red => content.red().bold().to_string(),
        }
    }

    /// Print the status line to the terminal using ANSI escape codes.
    ///
    /// Uses save cursor → move to bottom → clear line → print → restore cursor.
    /// Color-coded based on health: green (0 errors), yellow (1-2), red (3+).
    pub fn print(&self) {
        let content = self.render_colored();
        // Save cursor, move to last row, clear line, print, restore cursor
        eprint!("\x1b[s\x1b[999;1H\x1b[2K{content}\x1b[u");
    }

    /// Clear the status line from the terminal.
    pub fn clear(&self) {
        // Save cursor, move to last row, clear line, restore cursor
        eprint!("\x1b[s\x1b[999;1H\x1b[2K\x1b[u");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- truncate helper tests ---

    #[test]
    fn test_truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_limit_unchanged() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_over_limit_appends_ellipsis() {
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn test_truncate_multibyte_chars_no_panic() {
        // '—' is 3 bytes; byte-slicing at index 197 would panic if it splits the char.
        // Place the em-dash at char position 5 so truncating at 4 drops it cleanly.
        let s = "aaaa—extra text";
        let result = truncate(s, 4);
        assert_eq!(result, "aaaa...");
        assert!(!result.contains('—'));
    }

    #[test]
    fn test_truncate_multibyte_within_limit() {
        let s = "café";
        assert_eq!(truncate(s, 10), "café");
    }

    #[test]
    fn test_truncate_emoji_no_panic() {
        let s = "hello 🎉 world";
        let result = truncate(s, 7);
        assert_eq!(result, "hello 🎉...");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 10), "");
    }

    // --- CycleDisplay tests ---

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
    fn test_summarize_write_tool() {
        let input = json!({"file_path": "src/new.rs", "content": "fn main() {}"});
        assert_eq!(summarize_tool_input("Write", &input), " src/new.rs");
    }

    #[test]
    fn test_summarize_bash_tool() {
        let input = json!({"command": "cargo test --lib"});
        assert_eq!(summarize_tool_input("Bash", &input), " `cargo test --lib`");
    }

    #[test]
    fn test_summarize_bash_long_command_truncated() {
        let long_cmd = "a".repeat(200);
        let input = json!({"command": long_cmd});
        let result = summarize_tool_input("Bash", &input);
        // " `" + 80 chars + "...`" = 87 chars
        assert!(result.len() <= 87);
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
            duration_ms: 120_000,
            permission_denials: vec!["Edit".to_string(), "Bash".to_string()],
        });
    }

    // --- StatusLine tests ---

    #[test]
    fn test_status_line_new() {
        let status = StatusLine::new("coding");
        assert_eq!(status.cycle_name, "coding");
        assert_eq!(status.turn_count, 0);
        assert_eq!(status.error_count, 0);
        assert!(status.cost_usd.abs() < f64::EPSILON);
    }

    #[test]
    fn test_status_line_render_initial() {
        let status = StatusLine::with_start("coding", std::time::Instant::now());
        let rendered = status.render();
        assert!(rendered.contains("[coding]"));
        assert!(rendered.contains("0 turns"));
        assert!(rendered.contains("$0.00"));
        assert!(rendered.contains("0 errors"));
    }

    #[test]
    fn test_status_line_update_tool_use_increments_turns() {
        let mut status = StatusLine::new("coding");
        status.update(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: json!({}),
        });
        assert_eq!(status.turn_count, 1);

        status.update(&StreamEvent::ToolUse {
            tool_name: "Bash".to_string(),
            input: json!({}),
        });
        assert_eq!(status.turn_count, 2);
    }

    #[test]
    fn test_status_line_update_tool_error_increments_errors() {
        let mut status = StatusLine::new("coding");
        status.update(&StreamEvent::ToolResult {
            is_error: true,
            content: "permission denied".to_string(),
        });
        assert_eq!(status.error_count, 1);
    }

    #[test]
    fn test_status_line_update_tool_success_no_error_increment() {
        let mut status = StatusLine::new("coding");
        status.update(&StreamEvent::ToolResult {
            is_error: false,
            content: "ok".to_string(),
        });
        assert_eq!(status.error_count, 0);
    }

    #[test]
    fn test_status_line_update_result_sets_final_stats() {
        let mut status = StatusLine::new("coding");
        // Simulate some tool uses first
        status.update(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: json!({}),
        });
        assert_eq!(status.turn_count, 1);

        // Result event overrides with authoritative values
        status.update(&StreamEvent::Result {
            is_error: false,
            result_text: "Done".to_string(),
            num_turns: 15,
            total_cost_usd: 2.50,
            duration_ms: 60000,
            permission_denials: vec![],
        });
        assert_eq!(status.turn_count, 15);
        assert!((status.cost_usd - 2.50).abs() < f64::EPSILON);
    }

    #[test]
    fn test_status_line_render_format() {
        let mut status = StatusLine::with_start("gardening", std::time::Instant::now());
        // Simulate 3 tool uses and 1 error
        for _ in 0..3 {
            status.update(&StreamEvent::ToolUse {
                tool_name: "Edit".to_string(),
                input: json!({}),
            });
        }
        status.update(&StreamEvent::ToolResult {
            is_error: true,
            content: "denied".to_string(),
        });

        let rendered = status.render();
        assert!(rendered.contains("[gardening]"));
        assert!(rendered.contains("3 turns"));
        assert!(rendered.contains("1 errors"));
        assert!(rendered.contains("\u{25b6}")); // ▶ character
    }

    #[test]
    fn test_status_line_health_green_no_errors() {
        let status = StatusLine::new("coding");
        assert_eq!(status.health_color(), HealthColor::Green);
    }

    #[test]
    fn test_status_line_health_yellow_few_errors() {
        let mut status = StatusLine::new("coding");
        status.update(&StreamEvent::ToolResult {
            is_error: true,
            content: "denied".to_string(),
        });
        assert_eq!(status.health_color(), HealthColor::Yellow);

        status.update(&StreamEvent::ToolResult {
            is_error: true,
            content: "denied again".to_string(),
        });
        assert_eq!(status.health_color(), HealthColor::Yellow);
    }

    #[test]
    fn test_status_line_health_red_many_errors() {
        let mut status = StatusLine::new("coding");
        for _ in 0..3 {
            status.update(&StreamEvent::ToolResult {
                is_error: true,
                content: "denied".to_string(),
            });
        }
        assert_eq!(status.health_color(), HealthColor::Red);
    }

    #[test]
    fn test_status_line_render_colored_no_panic() {
        let mut status = StatusLine::new("coding");
        // Green
        let _ = status.render_colored();
        // Yellow
        status.update(&StreamEvent::ToolResult {
            is_error: true,
            content: "denied".to_string(),
        });
        let _ = status.render_colored();
        // Red
        for _ in 0..3 {
            status.update(&StreamEvent::ToolResult {
                is_error: true,
                content: "denied".to_string(),
            });
        }
        let _ = status.render_colored();
    }

    // --- Doctor display tests ---

    #[test]
    fn test_render_diagnostic_report_clean() {
        use crate::doctor::DiagnosticReport;

        let report = DiagnosticReport { findings: vec![] };
        let output = render_diagnostic_report(&report);
        assert!(output.contains("No issues found"));
    }

    #[test]
    fn test_render_diagnostic_report_with_findings() {
        use crate::doctor::{DiagnosticReport, Finding, Severity};

        let report = DiagnosticReport {
            findings: vec![
                Finding {
                    severity: Severity::Error,
                    code: "D001".to_string(),
                    message: "Permission denied for Edit".to_string(),
                    suggestion: Some("Add Edit(./src/**) to permissions".to_string()),
                },
                Finding {
                    severity: Severity::Warning,
                    code: "D002".to_string(),
                    message: "Cycle 'coding' failed 3/4 times".to_string(),
                    suggestion: None,
                },
                Finding {
                    severity: Severity::Info,
                    code: "D004".to_string(),
                    message: "Consider setting min_interval".to_string(),
                    suggestion: Some("Add min_interval = 3".to_string()),
                },
            ],
        };
        let output = render_diagnostic_report(&report);
        assert!(output.contains("D001"));
        assert!(output.contains("Permission denied"));
        assert!(output.contains("D002"));
        assert!(output.contains("D004"));
        assert!(output.contains("Add Edit(./src/**) to permissions"));
    }

    #[test]
    fn test_render_diagnostic_report_summary_counts() {
        use crate::doctor::{DiagnosticReport, Finding, Severity};

        let report = DiagnosticReport {
            findings: vec![
                Finding {
                    severity: Severity::Error,
                    code: "E1".to_string(),
                    message: "err".to_string(),
                    suggestion: None,
                },
                Finding {
                    severity: Severity::Warning,
                    code: "W1".to_string(),
                    message: "warn".to_string(),
                    suggestion: None,
                },
            ],
        };
        let output = render_diagnostic_report(&report);
        assert!(output.contains("1 error"));
        assert!(output.contains("1 warning"));
    }

    // --- render_run_summary tests ---

    #[test]
    fn test_render_run_summary_basic() {
        let mut cycles = std::collections::BTreeMap::new();
        cycles.insert("coding".to_string(), 3u32);
        cycles.insert("gardening".to_string(), 2u32);

        let output = render_run_summary(5, 20, 3.45, &cycles, 4, 1, 510);
        assert!(output.contains("5/20"), "Should show iteration progress");
        assert!(output.contains("$3.45"), "Should show cost");
        assert!(
            output.contains("coding\u{00d7}3"),
            "Should show coding count"
        );
        assert!(
            output.contains("gardening\u{00d7}2"),
            "Should show gardening count"
        );
        assert!(output.contains("4/5 succeeded"), "Should show success rate");
        assert!(output.contains("8m 30s"), "Should show duration");
    }

    #[test]
    fn test_render_run_summary_all_success() {
        let mut cycles = std::collections::BTreeMap::new();
        cycles.insert("coding".to_string(), 5u32);

        let output = render_run_summary(5, 10, 1.00, &cycles, 5, 0, 300);
        assert!(output.contains("5/5 succeeded"));
    }

    #[test]
    fn test_render_run_summary_single_cycle_type() {
        let mut cycles = std::collections::BTreeMap::new();
        cycles.insert("coding".to_string(), 5u32);

        let output = render_run_summary(5, 10, 2.00, &cycles, 4, 1, 600);
        assert!(output.contains("coding\u{00d7}5"));
        // Should not contain a comma since there's only one cycle type
        let cycles_line = output
            .lines()
            .find(|l: &&str| l.contains("Cycles:"))
            .unwrap();
        assert!(
            !cycles_line.contains(", "),
            "Single cycle type should not have comma separator"
        );
    }

    #[test]
    fn test_render_run_summary_zero_cost() {
        let cycles = std::collections::BTreeMap::new();
        let output = render_run_summary(1, 5, 0.0, &cycles, 1, 0, 30);
        assert!(output.contains("$0.00"));
    }

    #[test]
    fn test_status_line_render_with_iteration_context() {
        let status = StatusLine::with_iteration("coding", 3, 10);
        let rendered = status.render();
        assert!(
            rendered.starts_with("[3/10] "),
            "Should start with iteration prefix: {rendered}"
        );
        assert!(
            rendered.contains("[coding]"),
            "Should contain cycle name: {rendered}"
        );
    }

    #[test]
    fn test_status_line_render_without_iteration_context() {
        let status = StatusLine::new("coding");
        let rendered = status.render();
        assert!(
            !rendered.contains("[3/10]"),
            "Should not have iteration prefix: {rendered}"
        );
        assert!(
            rendered.starts_with("[coding]"),
            "Should start with cycle name: {rendered}"
        );
    }

    #[test]
    fn test_status_line_single_iteration_no_prefix() {
        let status = StatusLine::with_iteration("coding", 1, 1);
        let rendered = status.render();
        assert!(
            rendered.starts_with("[coding]"),
            "Single iteration should not show [1/1] prefix: {rendered}"
        );
    }

    #[test]
    fn test_status_line_with_iteration_and_start() {
        let status =
            StatusLine::with_iteration_and_start("coding", 5, 20, std::time::Instant::now());
        let rendered = status.render();
        assert!(
            rendered.starts_with("[5/20] "),
            "Should start with iteration prefix: {rendered}"
        );
        assert!(
            rendered.contains("[coding]"),
            "Should contain cycle name: {rendered}"
        );
        assert!(
            rendered.contains("0 turns"),
            "Should show initial turn count: {rendered}"
        );
    }

    #[test]
    fn test_status_line_ignores_irrelevant_events() {
        let mut status = StatusLine::new("coding");
        status.update(&StreamEvent::SystemInit {
            model: "claude-opus-4-6".to_string(),
            session_id: "abc".to_string(),
        });
        status.update(&StreamEvent::AssistantText {
            text: "Hello".to_string(),
        });
        status.update(&StreamEvent::Unknown {
            event_type: "heartbeat".to_string(),
        });
        assert_eq!(status.turn_count, 0);
        assert_eq!(status.error_count, 0);
    }

    #[test]
    fn test_format_duration_over_one_hour() {
        assert_eq!(format_duration(3661), "61m 1s");
    }

    #[test]
    fn test_format_duration_boundary_59_seconds() {
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn test_format_duration_boundary_60_seconds() {
        assert_eq!(format_duration(60), "1m");
    }

    #[test]
    fn test_format_duration_boundary_61_seconds() {
        assert_eq!(format_duration(61), "1m 1s");
    }
}
