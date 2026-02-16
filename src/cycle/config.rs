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
}

const fn default_max_permission_denials() -> u32 {
    10
}

const fn default_circuit_breaker_repeated() -> u32 {
    5
}

/// A single cycle definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CycleConfig {
    /// Unique name for this cycle
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// The prompt to send to Claude Code
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
}

const fn default_context() -> ContextMode {
    ContextMode::None
}

/// Top-level Flow configuration parsed from cycles.toml
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlowConfig {
    /// Global configuration
    pub global: GlobalConfig,
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
        // toml crate should report a parse error for missing required fields
        let msg = err.to_string();
        assert!(
            msg.contains("missing field") || msg.contains("Failed to parse"),
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
}
