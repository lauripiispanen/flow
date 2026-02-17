# Flow

> An automated coding pipeline runner that orchestrates structured code production using Claude Code

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Overview

Flow is a Rust-based orchestrator that enables automated, iterative software development by coordinating Claude Code CLI invocations. Each iteration executes a named **cycle** (e.g., coding, gardening, review) in a fresh context, allowing for focused, incremental progress.

### Key Features

- **Named Cycles**: Define purpose-specific cycles (coding, gardening, review, planning) in a single TOML config
- **Multi-Iteration Loops**: Run many iterations back-to-back with `--max-iterations`
- **AI Cycle Selection**: When no fixed cycle is specified, an AI selector picks the best cycle each iteration based on log history and TODO.md state
- **Dependency Triggers**: Cycles can auto-trigger after others (e.g., gardening after coding) with configurable minimum intervals
- **Additive Permissions**: Global + cycle-specific permissions merged and passed to Claude Code as `--allowedTools`
- **Live Status Bar**: Color-coded health display during execution showing turns, cost, and errors
- **JSONL Logging**: Every cycle outcome is logged to `.flow/log.jsonl` with cost, turns, denials, and timing
- **Diagnostics**: `flow doctor` analyzes config and log history for permission issues, high costs, and configuration lint
- **Circuit Breaker**: Kills runaway cycles after repeated consecutive tool errors

## Quick Start

### Prerequisites

- Rust 1.70+ (2021 edition)
- Claude Code CLI installed and in PATH

### Installation

```bash
git clone https://github.com/yourusername/flow.git
cd flow
cargo build --release
```

### Usage

```bash
# Run a single named cycle
flow --cycle coding

# Run 10 iterations of the same cycle
flow --cycle coding --max-iterations 10

# Run 10 iterations with AI-driven cycle selection
flow --max-iterations 10

# Run diagnostics on config and history
flow doctor

# Use a custom config path
flow --cycle coding --config my-cycles.toml
```

### CLI Reference

| Flag | Default | Description |
|------|---------|-------------|
| `--cycle <name>` | — | Cycle to execute (optional with `--max-iterations > 1`) |
| `--config <path>` | `cycles.toml` | Path to configuration file |
| `--log-dir <path>` | `.flow` | Directory for JSONL log output |
| `--max-iterations <n>` | `1` | Number of iterations to run |
| `--todo <path>` | `TODO.md` | Path to TODO.md for AI cycle selector context |

| Subcommand | Description |
|------------|-------------|
| `doctor` | Analyze config and log history, report findings by severity |

## Configuration

Flow is configured via a TOML file (default: `cycles.toml`).

```toml
[global]
permissions = ["Read", "Glob", "Grep", "Edit(./src/**)", "Bash(cargo *)"]
max_permission_denials = 10        # Kill cycle after this many total denials
circuit_breaker_repeated = 5       # Kill cycle after N consecutive tool errors

[[cycle]]
name = "coding"
description = "Pick a task from TODO.md, implement with TDD"
prompt = """
Your coding prompt here...
"""
permissions = ["Edit(./TODO.md)", "Bash(git *)"]   # Added to global permissions
after = []                                          # No auto-trigger dependencies
context = "summaries"                               # Log context: "full", "summaries", or "none"

[[cycle]]
name = "gardening"
description = "Refactoring, cleanup, dependency updates"
prompt = """
Your gardening prompt here...
"""
permissions = ["Edit(./Cargo.toml)", "Bash(git *)"]
after = ["coding"]                                  # Auto-triggers after coding
min_interval = 3                                    # But only if 3+ iterations since last run
context = "none"
```

### Permission Format

Permissions use `ToolName` or `ToolName(specifier)` syntax:

- Bare tool: `"Read"`, `"Glob"`, `"Grep"`, `"Edit"`, `"Write"`, `"Bash"`
- With specifier: `"Edit(./src/**)"`, `"Bash(cargo test *)"`, `"Write(./out.txt)"`

Cycle permissions are additive — they merge with global permissions and are deduplicated.

## How It Works

1. **Load config** — parse `cycles.toml`, validate cycles and permissions
2. **Select cycle** — either the fixed `--cycle` name, or AI-selected from log history + TODO.md
3. **Resolve permissions** — merge global + cycle-specific, deduplicate
4. **Execute** — spawn `claude` CLI with prompt, stream-JSON output, and allowed tools
5. **Display** — render live status bar with turn count, cost, and health color
6. **Log** — append `CycleOutcome` to `.flow/log.jsonl`
7. **Gate** — check permission denial threshold; abort if exceeded
8. **Trigger** — find dependent cycles (via `after` + `min_interval` rules)
9. **Repeat** — loop back to step 2 until `--max-iterations` reached or circuit breaker trips

## Diagnostics

`flow doctor` runs read-only analysis and reports findings at three severity levels:

| Code | Severity | What it checks |
|------|----------|----------------|
| D001 | Error | Permission denials in log history |
| D002 | Warning | Repeated cycle failures |
| D003 | Warning | High-cost runs (>$5 per cycle) |
| D004 | Info | Triggered cycles missing `min_interval` |
| D005 | Warning | Cycles with no permissions at all |
| D006 | Info | Frequency tuning suggestions |

## Project Structure

```
flow/
├── src/
│   ├── main.rs              # CLI entry point and iteration loop
│   ├── lib.rs               # Public library re-exports
│   ├── cycle/
│   │   ├── config.rs        # TOML config parsing and validation
│   │   ├── executor.rs      # Cycle execution and stream processing
│   │   ├── rules.rs         # Dependency triggers and min_interval logic
│   │   └── selector.rs      # AI-driven cycle selection
│   ├── claude/
│   │   ├── cli.rs           # Claude Code command builder
│   │   ├── permissions.rs   # Permission resolution and merging
│   │   └── stream.rs        # Stream-JSON event parser
│   ├── cli/
│   │   └── display.rs       # Terminal display and status bar
│   ├── doctor.rs            # Diagnostic engine (D001-D006)
│   └── log/
│       └── jsonl.rs         # JSONL logger
├── tests/                   # Integration tests
├── cycles.toml              # Cycle configuration
├── TODO.md                  # Task queue
└── AGENTS.md                # Agent context
```

## Development

```bash
cargo build          # Build
cargo test           # Run all tests
cargo clippy         # Lint
cargo fmt            # Format
```

## License

MIT License - see [LICENSE](LICENSE) file for details
