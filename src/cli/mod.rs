//! CLI output formatting
//!
//! Provides human-readable terminal display for cycle execution,
//! replacing raw JSON output with formatted, colored output.

pub mod display;

pub use display::render_diagnostic_report;
pub use display::render_run_summary;
pub use display::CycleDisplay;
pub use display::StatusLine;
