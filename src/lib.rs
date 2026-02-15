//! Flow - Automated coding pipeline runner
//!
//! Flow orchestrates structured code production by invoking Claude Code CLI
//! in sequential iterations. Each iteration resets context to maintain focus.

// Allow multiple crate versions from dependencies (can't easily control)
#![allow(clippy::multiple_crate_versions)]

pub mod claude;
pub mod cycle;
pub mod log;
pub mod pipeline;

// Re-export commonly used types
pub use claude::cli::build_command;
pub use claude::permissions::resolve_permissions;
pub use cycle::config::{CycleConfig, FlowConfig, GlobalConfig};
pub use log::{CycleOutcome, JsonlLogger};
pub use pipeline::{Pipeline, PipelineResult};
