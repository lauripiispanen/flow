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
            StreamEvent::AssistantText { text } => {
                self.text_fragments.push(text.clone());
            }
            StreamEvent::ToolUse { tool_name, .. } => {
                self.tools_used.push(tool_name.clone());
            }
            StreamEvent::ToolResult {
                is_error: true,
                content,
            } => {
                self.tool_errors.push(content.clone());
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

        match event {
            StreamEvent::SystemInit { model, session_id } => {
                assert_eq!(model, "claude-opus-4-6");
                assert_eq!(session_id, "abc-123");
            }
            other => panic!("Expected SystemInit, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_assistant_text_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello! How can I help?"}]}}"#;
        let event = parse_event(line).unwrap();

        match event {
            StreamEvent::AssistantText { text } => {
                assert_eq!(text, "Hello! How can I help?");
            }
            other => panic!("Expected AssistantText, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_assistant_tool_use_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file":"test.rs"}}]}}"#;
        let event = parse_event(line).unwrap();

        match event {
            StreamEvent::ToolUse { tool_name, input } => {
                assert_eq!(tool_name, "Edit");
                assert_eq!(input["file"], "test.rs");
            }
            other => panic!("Expected ToolUse, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_success_event() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":5,"result":"Task completed","total_cost_usd":1.23,"duration_ms":45000,"permission_denials":[]}"#;
        let event = parse_event(line).unwrap();

        match event {
            StreamEvent::Result {
                is_error,
                result_text,
                num_turns,
                total_cost_usd,
                duration_ms,
                permission_denials,
            } => {
                assert!(!is_error);
                assert_eq!(result_text, "Task completed");
                assert_eq!(num_turns, 5);
                assert!((total_cost_usd - 1.23).abs() < f64::EPSILON);
                assert_eq!(duration_ms, 45000);
                assert!(permission_denials.is_empty());
            }
            other => panic!("Expected Result, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_with_permission_denials() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":10,"result":"Done","total_cost_usd":2.50,"duration_ms":60000,"permission_denials":["Edit","Bash"]}"#;
        let event = parse_event(line).unwrap();

        match event {
            StreamEvent::Result {
                permission_denials, ..
            } => {
                assert_eq!(permission_denials, vec!["Edit", "Bash"]);
            }
            other => panic!("Expected Result, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_error_event() {
        let line = r#"{"type":"result","subtype":"error","is_error":true,"num_turns":1,"result":"Error occurred","total_cost_usd":0.05,"duration_ms":1000,"permission_denials":[]}"#;
        let event = parse_event(line).unwrap();

        match event {
            StreamEvent::Result { is_error, .. } => {
                assert!(is_error);
            }
            other => panic!("Expected Result, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_unknown_event_type() {
        let line = r#"{"type":"heartbeat","data":"ping"}"#;
        let event = parse_event(line).unwrap();

        match event {
            StreamEvent::Unknown { event_type } => {
                assert_eq!(event_type, "heartbeat");
            }
            other => panic!("Expected Unknown, got {other:?}"),
        }
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

        match event {
            StreamEvent::SystemInit { model, session_id } => {
                assert_eq!(model, "claude-opus-4-6");
                assert_eq!(session_id, "f9c16ac1");
            }
            other => panic!("Expected SystemInit, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_real_world_result() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":2166,"duration_api_ms":2142,"num_turns":1,"result":"Hello! How can I help you today?","total_cost_usd":0.12109,"usage":{"input_tokens":3},"permission_denials":[]}"#;
        let event = parse_event(line).unwrap();

        match event {
            StreamEvent::Result {
                is_error,
                result_text,
                num_turns,
                total_cost_usd,
                duration_ms,
                permission_denials,
            } => {
                assert!(!is_error);
                assert_eq!(result_text, "Hello! How can I help you today?");
                assert_eq!(num_turns, 1);
                assert!((total_cost_usd - 0.12109).abs() < 0.00001);
                assert_eq!(duration_ms, 2166);
                assert!(permission_denials.is_empty());
            }
            other => panic!("Expected Result, got {other:?}"),
        }
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
}
