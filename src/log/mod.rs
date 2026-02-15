//! Logging and observability
//!
//! This module provides logging functionality for Flow, including
//! JSONL logging for cycle execution history.

pub mod jsonl;

pub use jsonl::{CycleOutcome, JsonlLogger};
