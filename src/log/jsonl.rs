//! JSONL (JSON Lines) logging for cycle execution history
//!
//! Provides append-only logging of cycle outcomes to `.flow/log.jsonl`

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

/// Represents the outcome of a single cycle execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
        };

        let outcome2 = CycleOutcome {
            iteration: 2,
            cycle: "gardening".to_string(),
            timestamp: Utc::now(),
            outcome: "Updated dependencies".to_string(),
            files_changed: vec!["Cargo.toml".to_string()],
            tests_passed: 3,
            duration_secs: 45,
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
        };

        let outcome2 = CycleOutcome {
            iteration: 2,
            cycle: "gardening".to_string(),
            timestamp: Utc::now(),
            outcome: "Updated dependencies".to_string(),
            files_changed: vec!["Cargo.toml".to_string()],
            tests_passed: 3,
            duration_secs: 45,
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
}
