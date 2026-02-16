//! Flow - Automated coding pipeline runner
//!
//! Flow orchestrates structured code production by invoking Claude Code CLI
//! in sequential iterations. Each iteration resets context to maintain focus.

// Allow multiple crate versions from dependencies (can't easily control)
#![allow(clippy::multiple_crate_versions)]

pub mod claude;
pub mod cli;
pub mod cycle;
pub mod doctor;
pub mod log;

// Re-export commonly used types
pub use claude::cli::build_command;
pub use claude::permissions::resolve_permissions;
pub use claude::stream::{parse_event, StreamAccumulator, StreamEvent};
pub use cli::{CycleDisplay, HealthColor, StatusLine};
pub use cycle::config::{CycleConfig, FlowConfig, GlobalConfig};
pub use cycle::executor::{CycleExecutor, CycleResult, PreparedCycle};
pub use cycle::rules::find_triggered_cycles;
pub use log::{CycleOutcome, JsonlLogger};
