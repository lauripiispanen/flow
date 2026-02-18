//! Progress file writer for external observability
//!
//! Manages `.flow/progress.json` — a single JSON file that reflects the current
//! state of a running Flow session. External tools can poll this file to monitor
//! progress without parsing JSONL or terminal output.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Current status of a Flow run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    /// Run is currently executing
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed due to a cycle failure or error
    Failed,
    /// Run was stopped by a health/denial gate
    Stopped,
}

/// Snapshot of the current run state, written to `.flow/progress.json`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunProgress {
    /// When the run started (ISO 8601)
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Current iteration number (1-indexed)
    pub current_iteration: u32,
    /// Maximum iterations configured for this run
    pub max_iterations: u32,
    /// Name of the cycle currently executing
    pub current_cycle: String,
    /// Current status of the run
    pub current_status: RunStatus,
    /// Count of executions per cycle name
    pub cycles_executed: BTreeMap<String, u32>,
    /// Total duration of all completed cycles in seconds
    pub total_duration_secs: u64,
    /// Cumulative cost of all completed cycles in USD
    #[serde(default)]
    pub total_cost_usd: f64,
    /// Outcome text from the most recent cycle (None if no cycle has completed yet)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_outcome: Option<String>,
}

impl RunProgress {
    /// Create a new `RunProgress` for the start of a run.
    #[must_use]
    pub fn new(max_iterations: u32) -> Self {
        Self {
            started_at: chrono::Utc::now(),
            current_iteration: 1,
            max_iterations,
            current_cycle: String::new(),
            current_status: RunStatus::Running,
            cycles_executed: BTreeMap::new(),
            total_duration_secs: 0,
            total_cost_usd: 0.0,
            last_outcome: None,
        }
    }
}

/// Manages reading and writing `.flow/progress.json`
pub struct ProgressWriter {
    path: PathBuf,
}

impl ProgressWriter {
    /// Create a new `ProgressWriter` targeting `<log_dir>/progress.json`.
    pub fn new(log_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(log_dir)
            .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;
        Ok(Self {
            path: log_dir.join("progress.json"),
        })
    }

    /// Atomically write progress to the file (write to temp, then rename).
    pub fn write(&self, progress: &RunProgress) -> Result<()> {
        let json =
            serde_json::to_string_pretty(progress).context("Failed to serialize progress")?;
        let tmp_path = self.path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json.as_bytes())
            .with_context(|| format!("Failed to write temp file: {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &self.path).with_context(|| {
            format!(
                "Failed to rename {} -> {}",
                tmp_path.display(),
                self.path.display()
            )
        })?;
        Ok(())
    }

    /// Read the current progress from the file, or `None` if it doesn't exist.
    pub fn read(&self) -> Result<Option<RunProgress>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read {}", self.path.display()))?;
        let progress: RunProgress =
            serde_json::from_str(&content).context("Failed to parse progress.json")?;
        Ok(Some(progress))
    }

    /// Delete the progress file. No-op if it doesn't exist.
    pub fn delete(&self) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)
                .with_context(|| format!("Failed to delete {}", self.path.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn sample_progress() -> RunProgress {
        let mut cycles = BTreeMap::new();
        cycles.insert("coding".to_string(), 2);
        cycles.insert("gardening".to_string(), 1);

        RunProgress {
            started_at: Utc::now(),
            current_iteration: 3,
            max_iterations: 20,
            current_cycle: "coding".to_string(),
            current_status: RunStatus::Running,
            cycles_executed: cycles,
            total_duration_secs: 445,
            total_cost_usd: 3.45,
            last_outcome: Some("Added ClaudeClient implementation".to_string()),
        }
    }

    #[test]
    fn test_run_progress_serializes_to_expected_json() {
        let progress = sample_progress();
        let json = serde_json::to_value(&progress).unwrap();

        assert_eq!(json["current_iteration"], 3);
        assert_eq!(json["max_iterations"], 20);
        assert_eq!(json["current_cycle"], "coding");
        assert_eq!(json["current_status"], "running");
        assert_eq!(json["cycles_executed"]["coding"], 2);
        assert_eq!(json["cycles_executed"]["gardening"], 1);
        assert_eq!(json["total_duration_secs"], 445);
        assert_eq!(json["last_outcome"], "Added ClaudeClient implementation");
        // started_at should be present
        assert!(json["started_at"].is_string());
    }

    #[test]
    fn test_run_progress_round_trip() {
        let progress = sample_progress();
        let json = serde_json::to_string(&progress).unwrap();
        let recovered: RunProgress = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.current_iteration, progress.current_iteration);
        assert_eq!(recovered.max_iterations, progress.max_iterations);
        assert_eq!(recovered.current_cycle, progress.current_cycle);
        assert_eq!(recovered.current_status, progress.current_status);
        assert_eq!(recovered.cycles_executed, progress.cycles_executed);
        assert_eq!(recovered.total_duration_secs, progress.total_duration_secs);
        assert_eq!(recovered.last_outcome, progress.last_outcome);
    }

    #[test]
    fn test_run_status_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&RunStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&RunStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&RunStatus::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&RunStatus::Stopped).unwrap(),
            "\"stopped\""
        );
    }

    #[test]
    fn test_progress_writer_creates_file() {
        let tmp = TempDir::new().unwrap();
        let writer = ProgressWriter::new(tmp.path()).unwrap();

        writer.write(&sample_progress()).unwrap();

        assert!(tmp.path().join("progress.json").exists());
    }

    #[test]
    fn test_progress_writer_overwrites_on_update() {
        let tmp = TempDir::new().unwrap();
        let writer = ProgressWriter::new(tmp.path()).unwrap();

        let mut progress = sample_progress();
        writer.write(&progress).unwrap();

        progress.current_iteration = 5;
        progress.current_cycle = "gardening".to_string();
        writer.write(&progress).unwrap();

        let read_back = writer.read().unwrap().unwrap();
        assert_eq!(read_back.current_iteration, 5);
        assert_eq!(read_back.current_cycle, "gardening");
    }

    #[test]
    fn test_progress_writer_read_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let writer = ProgressWriter::new(tmp.path()).unwrap();

        assert!(writer.read().unwrap().is_none());
    }

    #[test]
    fn test_progress_writer_read_returns_written_data() {
        let tmp = TempDir::new().unwrap();
        let writer = ProgressWriter::new(tmp.path()).unwrap();

        let progress = sample_progress();
        writer.write(&progress).unwrap();

        let read_back = writer.read().unwrap().unwrap();
        assert_eq!(read_back.current_iteration, progress.current_iteration);
        assert_eq!(read_back.current_cycle, progress.current_cycle);
    }

    #[test]
    fn test_progress_writer_delete_removes_file() {
        let tmp = TempDir::new().unwrap();
        let writer = ProgressWriter::new(tmp.path()).unwrap();

        writer.write(&sample_progress()).unwrap();
        assert!(tmp.path().join("progress.json").exists());

        writer.delete().unwrap();
        assert!(!tmp.path().join("progress.json").exists());
    }

    #[test]
    fn test_progress_writer_delete_ok_when_missing() {
        let tmp = TempDir::new().unwrap();
        let writer = ProgressWriter::new(tmp.path()).unwrap();

        // Should not error even though no file was written
        writer.delete().unwrap();
    }

    #[test]
    fn test_progress_writer_atomic_write() {
        let tmp = TempDir::new().unwrap();
        let writer = ProgressWriter::new(tmp.path()).unwrap();

        writer.write(&sample_progress()).unwrap();

        // Verify no temp file remains
        assert!(!tmp.path().join("progress.json.tmp").exists());
        // But the actual file does exist
        assert!(tmp.path().join("progress.json").exists());
    }

    #[test]
    fn test_total_cost_usd_serializes() {
        let mut progress = sample_progress();
        progress.total_cost_usd = 3.45;
        let json = serde_json::to_value(&progress).unwrap();
        let cost = json["total_cost_usd"].as_f64().unwrap();
        assert!((cost - 3.45).abs() < f64::EPSILON);
    }

    #[test]
    fn test_total_cost_usd_defaults_to_zero_on_deserialize() {
        // Simulate a progress.json from an older version without total_cost_usd
        let json = r#"{
            "started_at": "2026-01-15T10:00:00Z",
            "current_iteration": 1,
            "max_iterations": 5,
            "current_cycle": "coding",
            "current_status": "running",
            "cycles_executed": {},
            "total_duration_secs": 0
        }"#;
        let progress: RunProgress = serde_json::from_str(json).unwrap();
        assert!((progress.total_cost_usd - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_last_outcome_omitted_when_none() {
        let progress = RunProgress {
            started_at: Utc::now(),
            current_iteration: 1,
            max_iterations: 5,
            current_cycle: "coding".to_string(),
            current_status: RunStatus::Running,
            cycles_executed: BTreeMap::new(),
            total_duration_secs: 0,
            total_cost_usd: 0.0,
            last_outcome: None,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(
            !json.contains("last_outcome"),
            "last_outcome should be omitted when None"
        );
    }
}
