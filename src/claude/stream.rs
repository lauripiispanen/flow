//! Stream-JSON parser for Claude Code's `--output-format stream-json` output
//!
//! Parses newline-delimited JSON events from Claude Code into structured
//! `StreamEvent` variants for display and data extraction.

use serde_json::Value;

/// A parsed event from Claude Code's stream-json output
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// System initialization with session metadata
    SystemInit {
        /// The model being used
        model: String,
        /// Session ID
        session_id: String,
    },
    /// Assistant text output
    AssistantText {
        /// The text content
        text: String,
    },
    /// Tool use request by the assistant
    ToolUse {
        /// Tool name (e.g., "Edit", "Bash")
        tool_name: String,
        /// Tool input as raw JSON
        input: Value,
    },
    /// Tool execution result
    ToolResult {
        /// Whether the tool succeeded
        is_error: bool,
        /// Content of the result (may be truncated)
        content: String,
    },
    /// Final result of the entire session
    Result {
        /// Whether this was a success
        is_error: bool,
        /// Human-readable result text
        result_text: String,
        /// Number of conversation turns
        num_turns: u32,
        /// Total cost in USD
        total_cost_usd: f64,
        /// Duration in milliseconds
        duration_ms: u64,
        /// Permission denial details
        permission_denials: Vec<String>,
    },
    /// Unrecognized event type
    Unknown {
        /// The raw event type string
        event_type: String,
    },
}

/// Parse a single line of stream-json output into a `StreamEvent`.
///
/// Returns `None` if the line is empty or not valid JSON.
#[must_use]
pub fn parse_event(line: &str) -> Option<StreamEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let value: Value = serde_json::from_str(line).ok()?;
    let event_type = value.get("type")?.as_str()?;

    match event_type {
        "system" => Some(parse_system_event(&value)),
        "assistant" => parse_assistant_event(&value),
        "result" => Some(parse_result_event(&value)),
        other => Some(StreamEvent::Unknown {
            event_type: other.to_string(),
        }),
    }
}

fn parse_system_event(value: &Value) -> StreamEvent {
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let session_id = value
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    StreamEvent::SystemInit { model, session_id }
}

fn parse_assistant_event(value: &Value) -> Option<StreamEvent> {
    let message = value.get("message")?;
    let content = message.get("content")?.as_array()?;

    // Extract first meaningful content block
    for block in content {
        let block_type = block.get("type")?.as_str()?;
        match block_type {
            "text" => {
                let text = block.get("text")?.as_str()?.to_string();
                return Some(StreamEvent::AssistantText { text });
            }
            "tool_use" => {
                let tool_name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let input = block.get("input").cloned().unwrap_or(Value::Null);
                return Some(StreamEvent::ToolUse { tool_name, input });
            }
            "tool_result" => {
                let is_error = block
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let content = block
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                return Some(StreamEvent::ToolResult { is_error, content });
            }
            _ => {}
        }
    }

    None
}

fn parse_result_event(value: &Value) -> StreamEvent {
    let is_error = value
        .get("is_error")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let result_text = value
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let num_turns = value
        .get("num_turns")
        .and_then(Value::as_u64)
        .map_or(0, |v| u32::try_from(v).unwrap_or(u32::MAX));
    let total_cost_usd = value
        .get("total_cost_usd")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let duration_ms = value
        .get("duration_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let permission_denials = value
        .get("permission_denials")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    StreamEvent::Result {
        is_error,
        result_text,
        num_turns,
        total_cost_usd,
        duration_ms,
        permission_denials,
    }
}

/// Suggest a permission fix for a denied tool.
///
/// Maps common tool names to their `--allowedTools` permission string
/// that should be added to cycles.toml.
#[must_use]
pub fn suggest_permission_fix(tool_name: &str) -> String {
    match tool_name {
        "Read" => "Read".to_string(),
        "Glob" => "Glob".to_string(),
        "Grep" => "Grep".to_string(),
        name if name.starts_with("Edit") => "Edit(./**) or Edit(./src/**)".to_string(),
        name if name.starts_with("Write") => "Write(./**) or Write(./src/**)".to_string(),
        name if name.starts_with("Bash") => "Bash(*) or Bash(cargo *)".to_string(),
        _ => tool_name.to_string(),
    }
}

/// Parse the number of passed tests from a cargo test output line.
///
/// Recognizes the pattern `test result: ... N passed;` produced by `cargo test`.
/// Returns `None` if the content does not contain a recognized cargo test summary.
fn parse_tests_passed(content: &str) -> Option<u32> {
    // Look for "N passed" in cargo test output (e.g. "test result: ok. 42 passed; 0 failed")
    let passed_idx = content.find(" passed")?;
    // Walk backwards from "passed" to find the start of the number
    let before = &content[..passed_idx];
    let number_start = before.rfind(|c: char| !c.is_ascii_digit())?;
    let number_str = &before[number_start + 1..];
    if number_str.is_empty() {
        return None;
    }
    // Only parse if this looks like cargo test output (contains "test result")
    if !content.contains("test result") {
        return None;
    }
    number_str.parse().ok()
}

/// Accumulator for stream events — collects data across events for final summary.
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    /// Text fragments collected from assistant events
    pub text_fragments: Vec<String>,
    /// Tool names used during the session
    pub tools_used: Vec<String>,
    /// Tool errors encountered
    pub tool_errors: Vec<String>,
    /// Final result (populated from Result event)
    pub result: Option<StreamEvent>,
    /// Session ID from `SystemInit` event (used for session affinity in multi-step cycles)
    pub session_id: Option<String>,
    /// Files modified during the session (from `Edit`/`Write` `ToolUse` events, deduplicated)
    pub files_changed: Vec<String>,
    /// Total number of tests passed, parsed from cargo test output in `ToolResult` content
    pub tests_passed: u32,
}

impl StreamAccumulator {
    /// Create a new empty accumulator
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream event and accumulate relevant data
    pub fn process(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::SystemInit { session_id, .. } => {
                self.session_id = Some(session_id.clone());
            }
            StreamEvent::AssistantText { text } => {
                self.text_fragments.push(text.clone());
            }
            StreamEvent::ToolUse { tool_name, input } => {
                self.tools_used.push(tool_name.clone());
                if matches!(tool_name.as_str(), "Edit" | "Write") {
                    if let Some(path) = input.get("file_path").and_then(Value::as_str) {
                        if !self.files_changed.contains(&path.to_string()) {
                            self.files_changed.push(path.to_string());
                        }
                    }
                }
            }
            StreamEvent::ToolResult {
                is_error: true,
                content,
            } => {
                self.tool_errors.push(content.clone());
            }
            StreamEvent::ToolResult {
                is_error: false,
                content,
            } => {
                if let Some(count) = parse_tests_passed(content) {
                    self.tests_passed = self.tests_passed.saturating_add(count);
                }
            }
            StreamEvent::Result { .. } => {
                self.result = Some(event.clone());
            }
            _ => {}
        }
    }

    /// Extract the number of permission denials from the result
    #[must_use]
    pub fn permission_denial_count(&self) -> u32 {
        match &self.result {
            Some(StreamEvent::Result {
                permission_denials, ..
            }) => u32::try_from(permission_denials.len()).unwrap_or(u32::MAX),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_event tests ---

    #[test]
    fn test_parse_empty_line_returns_none() {
        assert!(parse_event("").is_none());
        assert!(parse_event("   ").is_none());
    }

    #[test]
    fn test_parse_invalid_json_returns_none() {
        assert!(parse_event("not json").is_none());
        assert!(parse_event("{invalid").is_none());
    }

    #[test]
    fn test_parse_system_init_event() {
        let line = r#"{"type":"system","subtype":"init","model":"claude-opus-4-6","session_id":"abc-123","tools":["Read","Edit"]}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::SystemInit { model, session_id } = event else {
            panic!("Expected SystemInit, got {event:?}");
        };
        assert_eq!(model, "claude-opus-4-6");
        assert_eq!(session_id, "abc-123");
    }

    #[test]
    fn test_parse_assistant_text_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello! How can I help?"}]}}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::AssistantText { text } = event else {
            panic!("Expected AssistantText, got {event:?}");
        };
        assert_eq!(text, "Hello! How can I help?");
    }

    #[test]
    fn test_parse_assistant_tool_use_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file":"test.rs"}}]}}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::ToolUse { tool_name, input } = event else {
            panic!("Expected ToolUse, got {event:?}");
        };
        assert_eq!(tool_name, "Edit");
        assert_eq!(input["file"], "test.rs");
    }

    #[test]
    fn test_parse_result_success_event() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":5,"result":"Task completed","total_cost_usd":1.23,"duration_ms":45000,"permission_denials":[]}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::Result {
            is_error,
            result_text,
            num_turns,
            total_cost_usd,
            duration_ms,
            permission_denials,
        } = event
        else {
            panic!("Expected Result, got {event:?}");
        };
        assert!(!is_error);
        assert_eq!(result_text, "Task completed");
        assert_eq!(num_turns, 5);
        assert!((total_cost_usd - 1.23).abs() < f64::EPSILON);
        assert_eq!(duration_ms, 45000);
        assert!(permission_denials.is_empty());
    }

    #[test]
    fn test_parse_result_with_permission_denials() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":10,"result":"Done","total_cost_usd":2.50,"duration_ms":60000,"permission_denials":["Edit","Bash"]}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::Result {
            permission_denials, ..
        } = event
        else {
            panic!("Expected Result, got {event:?}");
        };
        assert_eq!(permission_denials, vec!["Edit", "Bash"]);
    }

    #[test]
    fn test_parse_result_error_event() {
        let line = r#"{"type":"result","subtype":"error","is_error":true,"num_turns":1,"result":"Error occurred","total_cost_usd":0.05,"duration_ms":1000,"permission_denials":[]}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::Result { is_error, .. } = event else {
            panic!("Expected Result, got {event:?}");
        };
        assert!(is_error);
    }

    #[test]
    fn test_parse_unknown_event_type() {
        let line = r#"{"type":"heartbeat","data":"ping"}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::Unknown { event_type } = event else {
            panic!("Expected Unknown, got {event:?}");
        };
        assert_eq!(event_type, "heartbeat");
    }

    #[test]
    fn test_parse_missing_type_returns_none() {
        let line = r#"{"data":"no type field"}"#;
        assert!(parse_event(line).is_none());
    }

    // --- Real-world format test ---

    #[test]
    fn test_parse_real_world_system_init() {
        let line = r#"{"type":"system","subtype":"init","cwd":"/Users/test/project","session_id":"f9c16ac1","tools":["Read","Edit"],"model":"claude-opus-4-6","permissionMode":"default"}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::SystemInit { model, session_id } = event else {
            panic!("Expected SystemInit, got {event:?}");
        };
        assert_eq!(model, "claude-opus-4-6");
        assert_eq!(session_id, "f9c16ac1");
    }

    #[test]
    fn test_parse_real_world_result() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":2166,"duration_api_ms":2142,"num_turns":1,"result":"Hello! How can I help you today?","total_cost_usd":0.12109,"usage":{"input_tokens":3},"permission_denials":[]}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::Result {
            is_error,
            result_text,
            num_turns,
            total_cost_usd,
            duration_ms,
            permission_denials,
        } = event
        else {
            panic!("Expected Result, got {event:?}");
        };
        assert!(!is_error);
        assert_eq!(result_text, "Hello! How can I help you today?");
        assert_eq!(num_turns, 1);
        assert!((total_cost_usd - 0.12109).abs() < 0.00001);
        assert_eq!(duration_ms, 2166);
        assert!(permission_denials.is_empty());
    }

    #[test]
    fn test_parse_assistant_tool_result_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_result","is_error":true,"content":"permission denied"}]}}"#;
        let event = parse_event(line).unwrap();

        let StreamEvent::ToolResult { is_error, content } = event else {
            panic!("Expected ToolResult, got {event:?}");
        };
        assert!(is_error);
        assert_eq!(content, "permission denied");
    }

    #[test]
    fn test_parse_assistant_empty_content_returns_none() {
        let line = r#"{"type":"assistant","message":{"content":[]}}"#;
        assert!(parse_event(line).is_none());
    }

    #[test]
    fn test_parse_assistant_skips_unknown_block_to_find_known() {
        // First block has unknown type, second has a recognized text type
        let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","text":"hmm"},{"type":"text","text":"Hello"}]}}"#;
        let event = parse_event(line).unwrap();
        match event {
            StreamEvent::AssistantText { text } => assert_eq!(text, "Hello"),
            other => panic!("Expected AssistantText, got {other:?}"),
        }
    }

    #[test]
    fn test_suggest_permission_fix_write() {
        assert_eq!(
            suggest_permission_fix("Write"),
            "Write(./**) or Write(./src/**)"
        );
    }

    // --- suggest_permission_fix tests ---

    #[test]
    fn test_suggest_permission_fix_simple_tools() {
        assert_eq!(suggest_permission_fix("Read"), "Read");
        assert_eq!(suggest_permission_fix("Glob"), "Glob");
        assert_eq!(suggest_permission_fix("Grep"), "Grep");
    }

    #[test]
    fn test_suggest_permission_fix_edit() {
        assert_eq!(
            suggest_permission_fix("Edit"),
            "Edit(./**) or Edit(./src/**)"
        );
    }

    #[test]
    fn test_suggest_permission_fix_bash() {
        assert_eq!(suggest_permission_fix("Bash"), "Bash(*) or Bash(cargo *)");
    }

    #[test]
    fn test_suggest_permission_fix_unknown_tool() {
        assert_eq!(suggest_permission_fix("WebSearch"), "WebSearch");
    }

    // --- StreamAccumulator tests ---

    #[test]
    fn test_accumulator_collects_text() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::AssistantText {
            text: "Hello".to_string(),
        });
        acc.process(&StreamEvent::AssistantText {
            text: "World".to_string(),
        });
        assert_eq!(acc.text_fragments, vec!["Hello", "World"]);
    }

    #[test]
    fn test_accumulator_collects_tool_names() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: Value::Null,
        });
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Bash".to_string(),
            input: Value::Null,
        });
        assert_eq!(acc.tools_used, vec!["Edit", "Bash"]);
    }

    #[test]
    fn test_accumulator_collects_tool_errors() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolResult {
            is_error: true,
            content: "permission denied".to_string(),
        });
        acc.process(&StreamEvent::ToolResult {
            is_error: false,
            content: "success".to_string(),
        });
        assert_eq!(acc.tool_errors.len(), 1);
        assert_eq!(acc.tool_errors[0], "permission denied");
    }

    #[test]
    fn test_accumulator_captures_result() {
        let mut acc = StreamAccumulator::new();
        let result = StreamEvent::Result {
            is_error: false,
            result_text: "Done".to_string(),
            num_turns: 5,
            total_cost_usd: 1.0,
            duration_ms: 30000,
            permission_denials: vec!["Edit".to_string()],
        };
        acc.process(&result);
        assert!(acc.result.is_some());
        assert_eq!(acc.permission_denial_count(), 1);
    }

    #[test]
    fn test_accumulator_permission_denial_count_no_result() {
        let acc = StreamAccumulator::new();
        assert_eq!(acc.permission_denial_count(), 0);
    }

    #[test]
    fn test_accumulator_captures_session_id() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::SystemInit {
            model: "claude-opus-4-6".to_string(),
            session_id: "test-session-abc".to_string(),
        });
        assert_eq!(acc.session_id.as_deref(), Some("test-session-abc"));
    }

    #[test]
    fn test_accumulator_session_id_default_is_none() {
        let acc = StreamAccumulator::new();
        assert!(acc.session_id.is_none());
    }

    // --- files_changed tracking tests ---

    #[test]
    fn test_accumulator_tracks_edit_file_path() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
        });
        assert_eq!(acc.files_changed, vec!["src/main.rs"]);
    }

    #[test]
    fn test_accumulator_tracks_write_file_path() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Write".to_string(),
            input: serde_json::json!({"file_path": "src/lib.rs"}),
        });
        assert_eq!(acc.files_changed, vec!["src/lib.rs"]);
    }

    #[test]
    fn test_accumulator_deduplicates_files_changed() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
        });
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
        });
        assert_eq!(acc.files_changed, vec!["src/main.rs"]);
    }

    #[test]
    fn test_accumulator_does_not_track_read_tool() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Read".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
        });
        assert!(acc.files_changed.is_empty());
    }

    #[test]
    fn test_accumulator_does_not_track_bash_tool() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "cargo test"}),
        });
        assert!(acc.files_changed.is_empty());
    }

    #[test]
    fn test_accumulator_files_changed_default_is_empty() {
        let acc = StreamAccumulator::new();
        assert!(acc.files_changed.is_empty());
    }

    #[test]
    fn test_accumulator_tracks_multiple_different_files() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
        });
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Write".to_string(),
            input: serde_json::json!({"file_path": "src/lib.rs"}),
        });
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: serde_json::json!({"file_path": "tests/integration_test.rs"}),
        });
        assert_eq!(
            acc.files_changed,
            vec!["src/main.rs", "src/lib.rs", "tests/integration_test.rs"]
        );
    }

    #[test]
    fn test_accumulator_ignores_edit_without_file_path() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolUse {
            tool_name: "Edit".to_string(),
            input: serde_json::json!({}),
        });
        assert!(acc.files_changed.is_empty());
    }

    // --- tests_passed tracking tests ---

    #[test]
    fn test_accumulator_tracks_tests_passed_from_cargo_output() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolResult {
            is_error: false,
            content: "test result: ok. 42 passed; 0 failed; 0 ignored".to_string(),
        });
        assert_eq!(acc.tests_passed, 42);
    }

    #[test]
    fn test_accumulator_accumulates_tests_passed_across_multiple_results() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolResult {
            is_error: false,
            content: "test result: ok. 10 passed; 0 failed; 0 ignored".to_string(),
        });
        acc.process(&StreamEvent::ToolResult {
            is_error: false,
            content: "test result: ok. 5 passed; 0 failed; 0 ignored".to_string(),
        });
        assert_eq!(acc.tests_passed, 15);
    }

    #[test]
    fn test_accumulator_tests_passed_default_is_zero() {
        let acc = StreamAccumulator::new();
        assert_eq!(acc.tests_passed, 0);
    }

    #[test]
    fn test_accumulator_ignores_non_cargo_tool_result_content() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolResult {
            is_error: false,
            content: "File saved successfully".to_string(),
        });
        assert_eq!(acc.tests_passed, 0);
    }

    #[test]
    fn test_accumulator_ignores_cargo_failure_results() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolResult {
            is_error: false,
            content: "test result: FAILED. 5 passed; 2 failed; 0 ignored".to_string(),
        });
        // Even for failures, count the passed tests
        assert_eq!(acc.tests_passed, 5);
    }

    #[test]
    fn test_accumulator_ignores_error_tool_results_for_tests_passed() {
        let mut acc = StreamAccumulator::new();
        acc.process(&StreamEvent::ToolResult {
            is_error: true,
            content: "test result: ok. 10 passed; 0 failed".to_string(),
        });
        // Error results are not counted for tests_passed (they're permission denials)
        assert_eq!(acc.tests_passed, 0);
    }
}
