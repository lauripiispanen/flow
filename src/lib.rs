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
pub mod init;
pub mod log;
#[cfg(test)]
pub mod testutil;

// Re-export commonly used types
pub use claude::cli::{
    build_command, build_command_with_options, build_command_with_session, run_for_result,
    CommandOptions,
};
pub use claude::permissions::{resolve_permissions, resolve_step_permissions};
pub use claude::stream::{parse_event, StreamAccumulator, StreamEvent};
pub use cli::{render_diagnostic_report, CycleDisplay, StatusLine};
pub use cycle::config::{CycleConfig, FlowConfig, GlobalConfig, StepConfig, StepRouter};
pub use cycle::executor::{CycleExecutor, CycleResult};
pub use cycle::rules::find_triggered_cycles;
pub use cycle::selector::select_cycle;
pub use cycle::template::{build_template_vars, expand_template};
pub use log::{CycleOutcome, JsonlLogger, ProgressWriter, RunProgress, RunStatus};
