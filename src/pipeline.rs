//! Pipeline execution logic
//!
//! This module defines the core Pipeline that orchestrates the execution
//! of individual steps in sequence.

use anyhow::Result;

/// The main pipeline orchestrator
///
/// A Pipeline takes a task description and executes a sequence of steps
/// (Plan, Implement, Test) using Claude Code CLI.
#[allow(dead_code)] // TODO: Remove when implementing new cycle architecture
pub struct Pipeline {
    task: String,
}

impl Pipeline {
    /// Create a new pipeline for the given task
    ///
    /// # Arguments
    /// * `task` - A description of what should be accomplished
    ///
    /// # Example
    /// ```
    /// use flow::Pipeline;
    ///
    /// let pipeline = Pipeline::new("Create a hello world function");
    /// ```
    #[must_use]
    pub fn new(task: &str) -> Self {
        Self {
            task: task.to_string(),
        }
    }

    /// Execute the pipeline
    ///
    /// Runs all steps (Plan, Implement, Test) in sequence and returns
    /// the aggregated results.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The task is empty or invalid
    /// - Any step fails to execute
    /// - Claude Code CLI is not available
    pub fn run(&self) -> Result<PipelineResult> {
        // TODO: Implement pipeline execution
        // This is intentionally unimplemented to maintain RED state for TDD
        unimplemented!("Pipeline execution not yet implemented")
    }
}

/// The result of a complete pipeline execution
pub struct PipelineResult {
    steps_completed: usize,
    successful: bool,
}

impl PipelineResult {
    /// Returns the number of steps that were completed
    #[must_use]
    pub const fn steps_completed(&self) -> usize {
        self.steps_completed
    }

    /// Returns whether the pipeline completed successfully
    #[must_use]
    pub const fn was_successful(&self) -> bool {
        self.successful
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = Pipeline::new("test task");
        assert_eq!(pipeline.task, "test task");
    }
}
