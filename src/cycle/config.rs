//! Cycle configuration parser
//!
//! Parses `cycles.toml` into structured cycle definitions.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Context mode for a cycle - controls how much history is provided
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContextMode {
    /// Full JSONL history
    Full,
    /// Summarized history
    Summaries,
    /// No history context
    None,
}

/// Global configuration shared across all cycles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalConfig {
    /// Permissions applied to all cycles
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Max permission denials before stopping between cycles (default: 10)
    #[serde(default = "default_max_permission_denials")]
    pub max_permission_denials: u32,
    /// Kill subprocess if same tool denied N times in a row (default: 5)
    #[serde(default = "default_circuit_breaker_repeated")]
    pub circuit_breaker_repeated: u32,
    /// Stop the entire run if this many consecutive cycles fail (default: 3)
    #[serde(default = "default_max_consecutive_failures")]
    pub max_consecutive_failures: u32,
    /// Print a periodic run summary every N iterations (default: 5, 0 = disabled)
    #[serde(default = "default_summary_interval")]
    pub summary_interval: u32,
}

const fn default_max_permission_denials() -> u32 {
    10
}

const fn default_circuit_breaker_repeated() -> u32 {
    5
}

const fn default_max_consecutive_failures() -> u32 {
    3
}

const fn default_summary_interval() -> u32 {
    5
}

/// Router mode for determining the next step after a step completes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepRouter {
    /// Proceed to the next step in TOML order (default)
    Sequential,
    /// Use an LLM call to determine the next step based on the completed step's output
    Llm,
}

const fn default_step_router() -> StepRouter {
    StepRouter::Sequential
}

const fn default_max_visits() -> u32 {
    3
}

/// A single step within a multi-step cycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepConfig {
    /// Unique name for this step within the cycle
    pub name: String,
    /// Optional session tag — steps sharing the same tag continue the same Claude session
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The prompt to send to Claude Code for this step
    pub prompt: String,
    /// Additional permissions for this step (additive to global + cycle)
    #[serde(default)]
    pub permissions: Vec<String>,
    /// How to determine the next step after this one completes.
    /// `sequential` (default): proceed to the next step in TOML order.
    /// `llm`: invoke a model to choose the next step based on this step's output.
    #[serde(default = "default_step_router")]
    pub router: StepRouter,
    /// Maximum number of times this step can be visited in one cycle execution.
    /// Prevents infinite loops when using LLM routing. Default: 3.
    #[serde(default = "default_max_visits")]
    pub max_visits: u32,
    /// Maximum number of agentic turns for this step (maps to --max-turns).
    /// Overrides the cycle-level value when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    /// Maximum cost in USD for this step (maps to --max-budget-usd).
    /// Overrides the cycle-level value when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cost_usd: Option<f64>,
}

/// A single cycle definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CycleConfig {
    /// Unique name for this cycle
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// The prompt to send to Claude Code (used for single-step cycles; empty for multi-step)
    #[serde(default)]
    pub prompt: String,
    /// Additional permissions for this cycle (additive to global)
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Cycles that must complete before this one triggers
    #[serde(default)]
    pub after: Vec<String>,
    /// How much context to provide
    #[serde(default = "default_context")]
    pub context: ContextMode,
    /// Minimum iterations since last run before this cycle can be auto-triggered.
    /// None means no constraint (always eligible).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_interval: Option<u32>,
    /// Maximum number of agentic turns per invocation (maps to `--max-turns`).
    /// Used as fallback for steps that don't set their own `max_turns`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    /// Maximum cost in USD per invocation (maps to `--max-budget-usd`).
    /// Used as fallback for steps that don't set their own `max_cost_usd`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cost_usd: Option<f64>,
    /// Steps for multi-step cycles. Empty means single-step (uses top-level `prompt`).
    #[serde(default, rename = "step")]
    pub steps: Vec<StepConfig>,
}

const fn default_context() -> ContextMode {
    ContextMode::None
}

impl CycleConfig {
    /// Returns `true` if this cycle has explicit steps (multi-step cycle).
    ///
    /// Single-step cycles use the top-level `prompt` field. Multi-step cycles
    /// define `[[cycle.step]]` entries and have an empty `prompt`.
    #[must_use]
    pub const fn is_multi_step(&self) -> bool {
        !self.steps.is_empty()
    }
}

/// Configuration for the AI cycle selector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectorConfig {
    /// Custom prompt/guidance for the selector (replaces the default selection criteria)
    #[serde(default)]
    pub prompt: String,
}

/// Top-level Flow configuration parsed from cycles.toml
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowConfig {
    /// Global configuration
    pub global: GlobalConfig,
    /// Optional selector configuration
    #[serde(default)]
    pub selector: Option<SelectorConfig>,
    /// Cycle definitions
    #[serde(rename = "cycle")]
    pub cycles: Vec<CycleConfig>,
}

impl FlowConfig {
    /// Parse a cycles.toml file from a path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        Self::parse(&content)
    }

    /// Parse cycles.toml content from a string
    pub fn parse(content: &str) -> Result<Self> {
        let config: Self = toml::from_str(content).context("Failed to parse cycles.toml")?;
        config.validate()?;
        Ok(config)
    }

    /// Find a cycle by name
    #[must_use]
    pub fn get_cycle(&self, name: &str) -> Option<&CycleConfig> {
        self.cycles.iter().find(|c| c.name == name)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        // Check for duplicate cycle names
        let mut seen = HashSet::new();
        for cycle in &self.cycles {
            if !seen.insert(&cycle.name) {
                bail!("Duplicate cycle name: '{}'", cycle.name);
            }
        }

        // Check that `after` references exist
        let names: HashSet<&str> = self.cycles.iter().map(|c| c.name.as_str()).collect();
        for cycle in &self.cycles {
            for dep in &cycle.after {
                if !names.contains(dep.as_str()) {
                    bail!(
                        "Cycle '{}' references unknown cycle '{}' in 'after'",
                        cycle.name,
                        dep
                    );
                }
            }
        }

        // Check that cycle names are non-empty
        for cycle in &self.cycles {
            if cycle.name.trim().is_empty() {
                bail!("Cycle name cannot be empty");
            }
        }

        // Validate permission strings in global config
        for perm in &self.global.permissions {
            validate_permission(perm)?;
        }

        // Validate permission strings in each cycle
        for cycle in &self.cycles {
            for perm in &cycle.permissions {
                validate_permission(perm).with_context(|| format!("in cycle '{}'", cycle.name))?;
            }
        }

        // Validate max_turns and max_cost_usd on cycles and steps
        for cycle in &self.cycles {
            validate_limits(cycle.max_turns, cycle.max_cost_usd, &cycle.name, None)?;
            for step in &cycle.steps {
                validate_limits(
                    step.max_turns,
                    step.max_cost_usd,
                    &cycle.name,
                    Some(&step.name),
                )?;
            }
        }

        // Validate that every cycle has either a prompt (single-step) or steps (multi-step)
        for cycle in &self.cycles {
            if cycle.steps.is_empty() && cycle.prompt.is_empty() {
                bail!(
                    "Cycle '{}' must have a 'prompt' (single-step) or '[[cycle.step]]' entries (multi-step)",
                    cycle.name
                );
            }
        }

        // Validate multi-step cycle constraints
        for cycle in &self.cycles {
            if !cycle.steps.is_empty() {
                // Multi-step cycle: prompt must not also be set
                if !cycle.prompt.is_empty() {
                    bail!(
                        "Cycle '{}' cannot have both a top-level 'prompt' and '[[cycle.step]]' entries",
                        cycle.name
                    );
                }

                // Step names must be unique and non-empty
                let mut step_names = HashSet::new();
                for step in &cycle.steps {
                    if step.name.trim().is_empty() {
                        bail!("Step name cannot be empty in cycle '{}'", cycle.name);
                    }
                    if !step_names.insert(step.name.as_str()) {
                        bail!(
                            "Duplicate step name '{}' in cycle '{}'",
                            step.name,
                            cycle.name
                        );
                    }
                }

                // Validate step permissions
                for step in &cycle.steps {
                    for perm in &step.permissions {
                        validate_permission(perm).with_context(|| {
                            format!("in step '{}' of cycle '{}'", step.name, cycle.name)
                        })?;
                    }
                }
            }
        }

        Ok(())
    }
}

/// Validate that a permission string matches `--allowedTools` syntax:
/// either `ToolName` (bare) or `ToolName(specifier)`.
///
/// Tool names must start with an uppercase ASCII letter and contain only
/// ASCII alphanumeric characters.
fn validate_permission(perm: &str) -> Result<()> {
    if perm.is_empty() {
        bail!("Invalid permission '': permission string cannot be empty");
    }

    // Find where the tool name ends
    let tool_end = perm
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(perm.len());

    let tool_name = &perm[..tool_end];

    // Tool name must be non-empty and start with uppercase
    if tool_name.is_empty() || !tool_name.starts_with(|c: char| c.is_ascii_uppercase()) {
        bail!("Invalid permission '{perm}': tool name must start with an uppercase letter");
    }

    let rest = &perm[tool_end..];

    if rest.is_empty() {
        // Bare tool name like "Read" — valid
        return Ok(());
    }

    // Must be ToolName(specifier)
    if !rest.starts_with('(') || !rest.ends_with(')') {
        bail!("Invalid permission '{perm}': expected format 'ToolName' or 'ToolName(specifier)'");
    }

    // Extract specifier (strip outer parens)
    let specifier = &rest[1..rest.len() - 1];
    if specifier.is_empty() {
        bail!("Invalid permission '{perm}': specifier inside parentheses cannot be empty");
    }

    Ok(())
}

/// Validate `max_turns` and `max_cost_usd` for a cycle or step.
fn validate_limits(
    max_turns: Option<u32>,
    max_cost_usd: Option<f64>,
    cycle_name: &str,
    step_name: Option<&str>,
) -> Result<()> {
    let prefix = step_name.map_or_else(
        || format!("Cycle '{cycle_name}'"),
        |s| format!("Step '{s}' in cycle '{cycle_name}'"),
    );
    if max_turns == Some(0) {
        bail!("{prefix}: max_turns must be greater than 0");
    }
    if let Some(cost) = max_cost_usd {
        if cost <= 0.0 {
            bail!("{prefix}: max_cost_usd must be greater than 0");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CONFIG: &str = r#"
[global]
permissions = ["Read", "Edit(./src/**)", "Bash(cargo *)"]

[[cycle]]
name = "coding"
description = "Pick a task and implement with TDD"
prompt = "You are Flow's coding cycle."
permissions = ["Edit(./tests/**)", "Bash(cargo test *)"]
after = []
context = "summaries"

[[cycle]]
name = "gardening"
description = "Deps, refactoring, docs"
prompt = "You are Flow's gardening cycle."
permissions = ["Edit(./Cargo.toml)", "Bash(cargo update *)"]
after = ["coding"]
context = "none"
"#;

    #[test]
    fn test_parse_valid_config() {
        let config = FlowConfig::parse(VALID_CONFIG).unwrap();

        assert_eq!(config.global.permissions.len(), 3);
        assert_eq!(config.global.permissions[0], "Read");
        assert_eq!(config.cycles.len(), 2);
    }

    #[test]
    fn test_parse_cycle_fields() {
        let config = FlowConfig::parse(VALID_CONFIG).unwrap();
        let coding = config.get_cycle("coding").unwrap();

        assert_eq!(coding.name, "coding");
        assert_eq!(coding.description, "Pick a task and implement with TDD");
        assert_eq!(coding.prompt, "You are Flow's coding cycle.");
        assert_eq!(
            coding.permissions,
            vec!["Edit(./tests/**)", "Bash(cargo test *)"]
        );
        assert!(coding.after.is_empty());
        assert_eq!(coding.context, ContextMode::Summaries);
    }

    #[test]
    fn test_parse_after_dependencies() {
        let config = FlowConfig::parse(VALID_CONFIG).unwrap();
        let gardening = config.get_cycle("gardening").unwrap();

        assert_eq!(gardening.after, vec!["coding"]);
    }

    #[test]
    fn test_parse_context_modes() {
        let config = FlowConfig::parse(VALID_CONFIG).unwrap();

        assert_eq!(
            config.get_cycle("coding").unwrap().context,
            ContextMode::Summaries
        );
        assert_eq!(
            config.get_cycle("gardening").unwrap().context,
            ContextMode::None
        );
    }

    #[test]
    fn test_context_mode_full() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "review"
description = "Code review"
prompt = "Review code"
context = "full"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        assert_eq!(
            config.get_cycle("review").unwrap().context,
            ContextMode::Full
        );
    }

    #[test]
    fn test_default_context_is_none() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "review"
description = "Code review"
prompt = "Review code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        assert_eq!(
            config.get_cycle("review").unwrap().context,
            ContextMode::None
        );
    }

    #[test]
    fn test_default_empty_permissions() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "review"
description = "Code review"
prompt = "Review code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let review = config.get_cycle("review").unwrap();

        assert!(review.permissions.is_empty());
        assert!(review.after.is_empty());
    }

    #[test]
    fn test_get_cycle_not_found() {
        let config = FlowConfig::parse(VALID_CONFIG).unwrap();
        assert!(config.get_cycle("nonexistent").is_none());
    }

    #[test]
    fn test_reject_duplicate_cycle_names() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "First"
prompt = "First"

[[cycle]]
name = "coding"
description = "Duplicate"
prompt = "Duplicate"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Duplicate cycle name"),
            "Expected 'Duplicate cycle name' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_unknown_after_reference() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
after = ["nonexistent"]
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("unknown cycle"),
            "Expected 'unknown cycle' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_empty_cycle_name() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = ""
description = "Empty"
prompt = "Empty"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("empty"),
            "Expected 'empty' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_missing_required_fields() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        // Either TOML parse error (missing description) or our validation error (no prompt/steps)
        let msg = err.to_string();
        assert!(
            msg.contains("missing field")
                || msg.contains("Failed to parse")
                || msg.contains("must have"),
            "Expected parse error for missing fields, got: {msg}"
        );
    }

    #[test]
    fn test_reject_invalid_toml() {
        let err = FlowConfig::parse("not valid toml {{{").unwrap_err();
        assert!(err.to_string().contains("Failed to parse"));
    }

    #[test]
    fn test_from_path_missing_file() {
        let err = FlowConfig::from_path("/nonexistent/cycles.toml").unwrap_err();
        assert!(err.to_string().contains("Failed to read"));
    }

    #[test]
    fn test_from_path_valid_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("cycles.toml");
        std::fs::write(&config_path, VALID_CONFIG).unwrap();

        let config = FlowConfig::from_path(&config_path).unwrap();
        assert_eq!(config.cycles.len(), 2);
    }

    #[test]
    fn test_multiline_prompt() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding cycle"
prompt = """
Line one.
Line two.
Line three.
"""
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert!(coding.prompt.contains("Line one."));
        assert!(coding.prompt.contains("Line three."));
    }

    #[test]
    fn test_global_safeguard_defaults() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        assert_eq!(config.global.max_permission_denials, 10);
        assert_eq!(config.global.circuit_breaker_repeated, 5);
    }

    #[test]
    fn test_global_safeguard_custom_values() {
        let toml = r#"
[global]
permissions = []
max_permission_denials = 20
circuit_breaker_repeated = 3

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        assert_eq!(config.global.max_permission_denials, 20);
        assert_eq!(config.global.circuit_breaker_repeated, 3);
    }

    #[test]
    fn test_global_permissions_preserved() {
        let config = FlowConfig::parse(VALID_CONFIG).unwrap();
        assert_eq!(
            config.global.permissions,
            vec!["Read", "Edit(./src/**)", "Bash(cargo *)"]
        );
    }

    // --- min_interval config field tests ---

    #[test]
    fn test_min_interval_default_is_none() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"

[[cycle]]
name = "gardening"
description = "Gardening"
prompt = "Garden"
after = ["coding"]
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let gardening = config.get_cycle("gardening").unwrap();
        assert_eq!(gardening.min_interval, None);
    }

    #[test]
    fn test_min_interval_parsed_from_config() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"

[[cycle]]
name = "gardening"
description = "Gardening"
prompt = "Garden"
after = ["coding"]
min_interval = 3
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let gardening = config.get_cycle("gardening").unwrap();
        assert_eq!(gardening.min_interval, Some(3));
    }

    #[test]
    fn test_min_interval_zero_is_valid() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
min_interval = 0
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert_eq!(coding.min_interval, Some(0));
    }

    // --- Permission string validation tests ---

    #[test]
    fn test_valid_bare_tool_name() {
        let toml = r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        assert!(FlowConfig::parse(toml).is_ok());
    }

    #[test]
    fn test_valid_tool_with_specifier() {
        let toml = r#"
[global]
permissions = ["Edit(./src/**)", "Bash(cargo test *)"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        assert!(FlowConfig::parse(toml).is_ok());
    }

    #[test]
    fn test_reject_lowercase_tool_name() {
        let toml = r#"
[global]
permissions = ["read"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid permission"),
            "Expected 'Invalid permission' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_empty_permission_string() {
        let toml = r#"
[global]
permissions = [""]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid permission"),
            "Expected 'Invalid permission' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_permission_with_spaces_in_tool_name() {
        let toml = r#"
[global]
permissions = ["Read Write"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid permission"),
            "Expected 'Invalid permission' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_unbalanced_parens() {
        let toml = r#"
[global]
permissions = ["Edit(./src/**"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid permission"),
            "Expected 'Invalid permission' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_empty_specifier() {
        let toml = r#"
[global]
permissions = ["Edit()"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid permission"),
            "Expected 'Invalid permission' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_permission_with_trailing_text() {
        let toml = r#"
[global]
permissions = ["Edit(./src/**)extra"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Invalid permission"),
            "Expected 'Invalid permission' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_invalid_cycle_permission() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
permissions = ["not-valid!"]
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Invalid permission"),
            "Expected 'Invalid permission' error, got: {msg}"
        );
        assert!(
            msg.contains("in cycle 'test'"),
            "Expected cycle context, got: {msg}"
        );
    }

    #[test]
    fn test_valid_known_tool_names() {
        let toml = r#"
[global]
permissions = ["Read", "Glob", "Grep", "Edit(./src/**)", "Write(./out.txt)", "Bash(cargo *)", "WebFetch", "WebSearch", "NotebookEdit(./nb.ipynb)", "Task", "TodoWrite"]

[[cycle]]
name = "test"
description = "Test"
prompt = "Test"
"#;
        assert!(FlowConfig::parse(toml).is_ok());
    }

    // --- Multi-step cycle config tests ---

    #[test]
    fn test_parse_multi_step_cycle() {
        let toml = r#"
[global]
permissions = ["Read"]

[[cycle]]
name = "coding"
description = "Multi-step coding cycle"
after = []

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
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert_eq!(coding.steps.len(), 2);
        assert_eq!(coding.steps[0].name, "plan");
        assert_eq!(coding.steps[0].session, Some("architect".to_string()));
        assert_eq!(coding.steps[0].prompt, "Read TODO.md and write a plan.");
        assert_eq!(
            coding.steps[0].permissions,
            vec!["Edit(./.flow/current-plan.md)"]
        );
        assert_eq!(coding.steps[1].name, "implement");
    }

    #[test]
    fn test_single_step_cycle_has_empty_steps() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "gardening"
description = "Gardening"
prompt = "You are gardening."
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let gardening = config.get_cycle("gardening").unwrap();
        assert!(gardening.steps.is_empty());
        assert_eq!(gardening.prompt, "You are gardening.");
    }

    #[test]
    fn test_step_without_session_tag_is_valid() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "implement"
prompt = "Implement the task."
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert_eq!(coding.steps[0].session, None);
    }

    #[test]
    fn test_reject_multi_step_cycle_with_top_level_prompt() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "This should not be here alongside steps."

[[cycle.step]]
name = "plan"
prompt = "Plan."
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("cannot have both"),
            "Expected 'cannot have both' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_cycle_without_prompt_and_without_steps() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must have") || msg.contains("missing field") || msg.contains("prompt"),
            "Expected error about missing prompt or steps, got: {msg}"
        );
    }

    #[test]
    fn test_reject_duplicate_step_names_within_cycle() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."

[[cycle.step]]
name = "plan"
prompt = "Also plan."
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Duplicate step name"),
            "Expected 'Duplicate step name' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_empty_step_name() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = ""
prompt = "Plan."
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("empty"),
            "Expected 'empty' error for step name, got: {err}"
        );
    }

    #[test]
    fn test_multi_step_cycle_has_no_top_level_prompt() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert!(coding.prompt.is_empty());
    }

    /// Verify the actual cycles.toml in the project root parses and that the
    /// coding cycle is a multi-step cycle with plan / plan-review / implement steps.
    #[test]
    fn test_actual_cycles_toml_coding_is_multi_step() {
        let config = FlowConfig::from_path("cycles.toml").expect("cycles.toml must be parseable");
        let coding = config.get_cycle("coding").expect("coding cycle must exist");

        assert!(
            coding.is_multi_step(),
            "coding cycle should be multi-step (using [[cycle.step]] entries)"
        );
        assert!(
            coding.prompt.is_empty(),
            "multi-step cycle must not have a top-level prompt"
        );

        let step_names: Vec<&str> = coding.steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            step_names,
            vec!["plan", "plan-review", "implement", "reflect"],
            "coding cycle should have plan, plan-review, implement, reflect steps"
        );

        // plan step: architect session, read-only + plan file write
        let plan = &coding.steps[0];
        assert_eq!(plan.session.as_deref(), Some("architect"));
        assert!(
            plan.permissions
                .iter()
                .any(|p| p.starts_with("Edit(./.flow/")),
            "plan step should have edit permission for .flow/ artifacts"
        );

        // plan-review step: architect continues (same session), reads plan, can exit 1
        let review = &coding.steps[1];
        assert_eq!(review.name, "plan-review");
        assert_eq!(
            review.session.as_deref(),
            Some("architect"),
            "plan-review should continue the architect session"
        );

        // implement step: coder session with full write permissions
        let implement = &coding.steps[2];
        assert_eq!(implement.session.as_deref(), Some("coder"));
        assert!(
            implement.permissions.iter().any(|p| p == "Bash(git *)"),
            "implement step should have git permissions for committing"
        );
    }

    // --- StepConfig router and max_visits tests ---

    #[test]
    fn test_step_router_default_is_sequential() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let step = &config.get_cycle("coding").unwrap().steps[0];
        assert_eq!(step.router, StepRouter::Sequential);
    }

    #[test]
    fn test_step_router_llm_parsed() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan-review"
prompt = "Review the plan."
router = "llm"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let step = &config.get_cycle("coding").unwrap().steps[0];
        assert_eq!(step.router, StepRouter::Llm);
    }

    #[test]
    fn test_step_router_sequential_explicit() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
router = "sequential"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let step = &config.get_cycle("coding").unwrap().steps[0];
        assert_eq!(step.router, StepRouter::Sequential);
    }

    #[test]
    fn test_step_max_visits_default_is_3() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let step = &config.get_cycle("coding").unwrap().steps[0];
        assert_eq!(step.max_visits, 3);
    }

    #[test]
    fn test_step_max_visits_custom_value() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
max_visits = 5
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let step = &config.get_cycle("coding").unwrap().steps[0];
        assert_eq!(step.max_visits, 5);
    }

    #[test]
    fn test_reject_invalid_router_value() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
router = "invalid"
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("Failed to parse"),
            "Expected parse error for invalid router, got: {err}"
        );
    }

    // --- SelectorConfig tests ---

    #[test]
    fn test_selector_config_parsed() {
        let toml = r#"
[global]
permissions = []

[selector]
prompt = "Read TODO.md for priorities. Focus on P0 tasks first."

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let selector = config.selector.as_ref().expect("selector should be Some");
        assert_eq!(
            selector.prompt,
            "Read TODO.md for priorities. Focus on P0 tasks first."
        );
    }

    #[test]
    fn test_selector_config_absent_is_none() {
        let config = FlowConfig::parse(VALID_CONFIG).unwrap();
        assert!(
            config.selector.is_none(),
            "config without [selector] should have selector = None"
        );
    }

    /// Verify the actual cycles.toml parses correctly with the optional [selector] field.
    /// When [selector] is added to cycles.toml, this test validates it has a non-empty prompt.
    #[test]
    fn test_actual_cycles_toml_parses_with_optional_selector() {
        let config = FlowConfig::from_path("cycles.toml").expect("cycles.toml must be parseable");
        // selector is optional — just verify the config parses without error
        if let Some(selector) = &config.selector {
            assert!(
                !selector.prompt.is_empty(),
                "if [selector] is present, its prompt should be non-empty"
            );
        }
    }

    #[test]
    fn test_selector_config_empty_prompt() {
        let toml = r#"
[global]
permissions = []

[selector]
prompt = ""

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let selector = config.selector.as_ref().expect("selector should be Some");
        assert!(selector.prompt.is_empty());
    }

    // --- summary_interval config field tests ---

    #[test]
    fn test_summary_interval_defaults_to_five() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        assert_eq!(config.global.summary_interval, 5);
    }

    #[test]
    fn test_summary_interval_custom_value() {
        let toml = r#"
[global]
permissions = []
summary_interval = 10

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        assert_eq!(config.global.summary_interval, 10);
    }

    #[test]
    fn test_summary_interval_zero_is_valid() {
        let toml = r#"
[global]
permissions = []
summary_interval = 0

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        assert_eq!(config.global.summary_interval, 0);
    }

    // --- max_turns / max_cost_usd config field tests ---

    #[test]
    fn test_max_turns_default_is_none() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert_eq!(coding.max_turns, None);
    }

    #[test]
    fn test_max_turns_parsed_from_config() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
max_turns = 50
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert_eq!(coding.max_turns, Some(50));
    }

    #[test]
    fn test_max_cost_usd_default_is_none() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert_eq!(coding.max_cost_usd, None);
    }

    #[test]
    fn test_max_cost_usd_parsed_from_config() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
max_cost_usd = 5.0
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let coding = config.get_cycle("coding").unwrap();
        assert!((coding.max_cost_usd.unwrap() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_step_max_turns_parsed() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
max_turns = 30
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let step = &config.get_cycle("coding").unwrap().steps[0];
        assert_eq!(step.max_turns, Some(30));
    }

    #[test]
    fn test_step_max_cost_usd_parsed() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
max_cost_usd = 2.0
"#;
        let config = FlowConfig::parse(toml).unwrap();
        let step = &config.get_cycle("coding").unwrap().steps[0];
        assert!((step.max_cost_usd.unwrap() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reject_max_turns_zero() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
max_turns = 0
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("max_turns"),
            "Expected 'max_turns' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_max_cost_usd_zero() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
max_cost_usd = 0.0
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("max_cost_usd"),
            "Expected 'max_cost_usd' error, got: {err}"
        );
    }

    #[test]
    fn test_reject_max_cost_usd_negative() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
prompt = "Code"
max_cost_usd = -1.0
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        assert!(
            err.to_string().contains("max_cost_usd"),
            "Expected 'max_cost_usd' error, got: {err}"
        );
    }

    #[test]
    fn test_step_permissions_validated() {
        let toml = r#"
[global]
permissions = []

[[cycle]]
name = "coding"
description = "Coding"
after = []

[[cycle.step]]
name = "plan"
prompt = "Plan."
permissions = ["not-valid"]
"#;
        let err = FlowConfig::parse(toml).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Invalid permission"),
            "Expected 'Invalid permission' error for step permission, got: {msg}"
        );
        assert!(
            msg.contains("in step 'plan'"),
            "Expected step context in error, got: {msg}"
        );
    }
}
