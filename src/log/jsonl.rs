//! JSONL (JSON Lines) logging for cycle execution history
//!
//! Provides append-only logging of cycle outcomes to `.flow/log.jsonl`

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Per-step outcome data for multi-step cycles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepOutcome {
    /// Step name (e.g., "plan", "implement", "review")
    pub name: String,
    /// Session tag used for this step (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Duration of this step in seconds
    pub duration_secs: u64,
    /// Number of conversation turns in this step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_turns: Option<u32>,
    /// Cost of this step in USD
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Represents the outcome of a single cycle execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CycleOutcome {
    /// The iteration number (1-indexed)
    pub iteration: u32,
    /// The name of the cycle that was executed
    pub cycle: String,
    /// ISO 8601 timestamp of when the cycle completed
    pub timestamp: DateTime<Utc>,
    /// Human-readable summary of what was accomplished
    pub outcome: String,
    /// List of files that were modified
    pub files_changed: Vec<String>,
    /// Number of tests that passed
    pub tests_passed: u32,
    /// Duration of the cycle in seconds
    pub duration_secs: u64,
    /// Number of conversation turns
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_turns: Option<u32>,
    /// Total cost in USD
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    /// Number of permission denials during the cycle
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_denial_count: Option<u32>,
    /// List of denied tool names (e.g., `["Edit", "Bash"]`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_denials: Option<Vec<String>>,
    /// Per-step outcome data for multi-step cycles (omitted for single-step cycles)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<StepOutcome>>,
}

/// JSONL logger for cycle execution history
///
/// Provides append-only logging to `.flow/log.jsonl`.
/// Each line is a JSON object representing a single cycle outcome.
pub struct JsonlLogger {
    log_path: PathBuf,
}

impl JsonlLogger {
    /// Create a new JSONL logger
    ///
    /// # Arguments
    /// * `log_dir` - Directory where log.jsonl will be stored (typically `.flow`)
    ///
    /// # Errors
    /// Returns an error if the log directory cannot be created
    pub fn new<P: AsRef<Path>>(log_dir: P) -> Result<Self> {
        let log_dir = log_dir.as_ref();

        // Create the log directory if it doesn't exist
        fs::create_dir_all(log_dir)
            .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;

        let log_path = log_dir.join("log.jsonl");

        Ok(Self { log_path })
    }

    /// Append a cycle outcome to the log
    ///
    /// # Arguments
    /// * `outcome` - The cycle outcome to log
    ///
    /// # Errors
    /// Returns an error if:
    /// - The log file cannot be opened or created
    /// - The outcome cannot be serialized to JSON
    /// - Writing to the file fails
    pub fn append(&self, outcome: &CycleOutcome) -> Result<()> {
        // Open file in append mode, create if it doesn't exist
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("Failed to open log file: {}", self.log_path.display()))?;

        // Serialize to JSON
        let json =
            serde_json::to_string(outcome).context("Failed to serialize cycle outcome to JSON")?;

        // Write JSON line
        writeln!(file, "{json}").context("Failed to write to log file")?;

        Ok(())
    }

    /// Read all cycle outcomes from the log
    ///
    /// # Returns
    /// A vector of all cycle outcomes, in chronological order
    ///
    /// # Errors
    /// Returns an error if:
    /// - The log file cannot be read
    /// - Any line cannot be parsed as valid JSON
    pub fn read_all(&self) -> Result<Vec<CycleOutcome>> {
        // If log file doesn't exist yet, return empty vector
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.log_path)
            .with_context(|| format!("Failed to read log file: {}", self.log_path.display()))?;

        let mut outcomes = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            let outcome: CycleOutcome = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse line {} as JSON", line_num + 1))?;

            outcomes.push(outcome);
        }

        Ok(outcomes)
    }

    /// Get the path to the log file
    #[must_use]
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_logger_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join(".flow");

        let logger = JsonlLogger::new(&log_dir).unwrap();

        assert!(log_dir.exists());
        assert_eq!(logger.log_path(), log_dir.join("log.jsonl"));
    }

    #[test]
    fn test_append_creates_file_and_writes_json() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "Implemented basic feature".to_string(),
            files_changed: vec!["src/main.rs".to_string()],
            tests_passed: 3,
            duration_secs: 180,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        logger.append(&outcome).unwrap();

        assert!(logger.log_path().exists());
    }

    #[test]
    fn test_append_multiple_outcomes() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome1 = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "First task".to_string(),
            files_changed: vec!["src/main.rs".to_string()],
            tests_passed: 3,
            duration_secs: 180,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        let outcome2 = CycleOutcome {
            iteration: 2,
            cycle: "gardening".to_string(),
            timestamp: Utc::now(),
            outcome: "Updated dependencies".to_string(),
            files_changed: vec!["Cargo.toml".to_string()],
            tests_passed: 3,
            duration_secs: 45,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        logger.append(&outcome1).unwrap();
        logger.append(&outcome2).unwrap();

        // Read the file and verify it has two lines
        let content = fs::read_to_string(logger.log_path()).unwrap();
        assert_eq!(content.lines().count(), 2);
    }

    #[test]
    fn test_read_all_empty_log() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcomes = logger.read_all().unwrap();
        assert!(outcomes.is_empty());
    }

    #[test]
    fn test_read_all_returns_outcomes() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome1 = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "First task".to_string(),
            files_changed: vec!["src/main.rs".to_string()],
            tests_passed: 3,
            duration_secs: 180,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        let outcome2 = CycleOutcome {
            iteration: 2,
            cycle: "gardening".to_string(),
            timestamp: Utc::now(),
            outcome: "Updated dependencies".to_string(),
            files_changed: vec!["Cargo.toml".to_string()],
            tests_passed: 3,
            duration_secs: 45,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        logger.append(&outcome1).unwrap();
        logger.append(&outcome2).unwrap();

        let results = logger.read_all().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].iteration, 1);
        assert_eq!(results[0].cycle, "coding");
        assert_eq!(results[1].iteration, 2);
        assert_eq!(results[1].cycle, "gardening");
    }

    #[test]
    fn test_round_trip_serialization() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let original = CycleOutcome {
            iteration: 42,
            cycle: "testing".to_string(),
            timestamp: Utc::now(),
            outcome: "All tests pass".to_string(),
            files_changed: vec![
                "src/main.rs".to_string(),
                "tests/integration.rs".to_string(),
            ],
            tests_passed: 15,
            duration_secs: 300,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        logger.append(&original).unwrap();

        let outcomes = logger.read_all().unwrap();
        assert_eq!(outcomes.len(), 1);

        let recovered = &outcomes[0];
        assert_eq!(recovered.iteration, original.iteration);
        assert_eq!(recovered.cycle, original.cycle);
        assert_eq!(recovered.outcome, original.outcome);
        assert_eq!(recovered.files_changed, original.files_changed);
        assert_eq!(recovered.tests_passed, original.tests_passed);
        assert_eq!(recovered.duration_secs, original.duration_secs);
        // Note: timestamp might have minor precision differences
    }

    #[test]
    fn test_cycle_outcome_with_rich_fields() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "Implemented feature X with 5 new tests".to_string(),
            files_changed: vec!["src/main.rs".to_string()],
            tests_passed: 5,
            duration_secs: 253,
            num_turns: Some(53),
            total_cost_usd: Some(2.15),
            permission_denial_count: Some(3),
            permission_denials: Some(vec![
                "Edit".to_string(),
                "Bash".to_string(),
                "Edit".to_string(),
            ]),
            steps: None,
        };

        logger.append(&outcome).unwrap();

        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].num_turns, Some(53));
        assert_eq!(entries[0].total_cost_usd, Some(2.15));
        assert_eq!(entries[0].permission_denial_count, Some(3));
        assert_eq!(entries[0].permission_denials.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_cycle_outcome_rich_fields_default_for_backward_compat() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        // Simulate old-format JSONL without the new fields
        let old_json = r#"{"iteration":1,"cycle":"coding","timestamp":"2026-02-15T00:00:00Z","outcome":"done","files_changed":[],"tests_passed":0,"duration_secs":60}"#;
        std::fs::write(logger.log_path(), format!("{old_json}\n")).unwrap();

        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].num_turns, None);
        assert_eq!(entries[0].total_cost_usd, None);
        assert_eq!(entries[0].permission_denial_count, None);
        assert_eq!(entries[0].permission_denials, None);
    }

    #[test]
    fn test_cycle_outcome_with_permission_denials_list() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "Completed with denials".to_string(),
            files_changed: vec![],
            tests_passed: 0,
            duration_secs: 120,
            num_turns: Some(10),
            total_cost_usd: Some(1.50),
            permission_denial_count: Some(2),
            permission_denials: Some(vec!["Edit".to_string(), "Bash".to_string()]),
            steps: None,
        };

        logger.append(&outcome).unwrap();

        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].permission_denials,
            Some(vec!["Edit".to_string(), "Bash".to_string()])
        );
    }

    #[test]
    fn test_cycle_outcome_permission_denials_omitted_when_none() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "done".to_string(),
            files_changed: vec![],
            tests_passed: 0,
            duration_secs: 60,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        logger.append(&outcome).unwrap();

        // Read raw JSON to verify the field is not present
        let content = fs::read_to_string(logger.log_path()).unwrap();
        assert!(!content.contains("permission_denials"));
    }

    #[test]
    fn test_cycle_outcome_steps_field_omitted_when_none() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "done".to_string(),
            files_changed: vec![],
            tests_passed: 0,
            duration_secs: 60,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };

        logger.append(&outcome).unwrap();
        let content = fs::read_to_string(logger.log_path()).unwrap();
        assert!(
            !content.contains("\"steps\""),
            "steps should not appear when None"
        );
    }

    #[test]
    fn test_cycle_outcome_steps_field_round_trips() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        let outcome = CycleOutcome {
            iteration: 1,
            cycle: "coding".to_string(),
            timestamp: Utc::now(),
            outcome: "Multi-step complete".to_string(),
            files_changed: vec![],
            tests_passed: 0,
            duration_secs: 300,
            num_turns: Some(30),
            total_cost_usd: Some(1.5),
            permission_denial_count: None,
            permission_denials: None,
            steps: Some(vec![
                StepOutcome {
                    name: "plan".to_string(),
                    session: Some("architect".to_string()),
                    duration_secs: 120,
                    num_turns: Some(10),
                    cost_usd: Some(0.5),
                },
                StepOutcome {
                    name: "implement".to_string(),
                    session: None,
                    duration_secs: 180,
                    num_turns: Some(20),
                    cost_usd: Some(1.0),
                },
            ]),
        };

        logger.append(&outcome).unwrap();
        let entries = logger.read_all().unwrap();
        let steps = entries[0].steps.as_ref().unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name, "plan");
        assert_eq!(steps[0].session, Some("architect".to_string()));
        assert_eq!(steps[1].name, "implement");
        assert_eq!(steps[1].session, None);
    }

    #[test]
    fn test_cycle_outcome_backward_compat_without_steps_field() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        // Old format without steps field
        let old_json = r#"{"iteration":1,"cycle":"coding","timestamp":"2026-02-15T00:00:00Z","outcome":"done","files_changed":[],"tests_passed":0,"duration_secs":60}"#;
        std::fs::write(logger.log_path(), format!("{old_json}\n")).unwrap();

        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].steps.is_none());
    }

    #[test]
    fn test_cycle_outcome_backward_compat_with_count_but_no_list() {
        let temp_dir = TempDir::new().unwrap();
        let logger = JsonlLogger::new(temp_dir.path()).unwrap();

        // Simulate format that has permission_denial_count but not permission_denials
        let json = r#"{"iteration":1,"cycle":"coding","timestamp":"2026-02-15T00:00:00Z","outcome":"done","files_changed":[],"tests_passed":0,"duration_secs":60,"permission_denial_count":3}"#;
        std::fs::write(logger.log_path(), format!("{json}\n")).unwrap();

        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].permission_denial_count, Some(3));
        assert_eq!(entries[0].permission_denials, None);
    }
}
