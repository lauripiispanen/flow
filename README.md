# Flow

> "The outer loop that turns Claude Code from a single-shot tool into an autonomous development pipeline."

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Flow is a loop runner for Claude Code. You define named cycles — coding, gardening, review, docs, planning — each with its own prompt and permission set, and Flow executes them as fresh Claude Code invocations. Point it at a `cycles.toml` and a `TODO.md`, run `flow --max-iterations 20`, and it picks the most useful cycle each iteration, spawns Claude Code with scoped permissions, logs the outcome, auto-triggers follow-up cycles, and repeats — autonomous software development in a loop, with guardrails.

## Features

- **Named Cycles**: Define purpose-specific cycles (coding, gardening, review, docs, planning) in a single TOML config
- **Multi-Step Cycles**: Break cycles into sequential steps with session affinity — e.g., plan → review → implement in the same conversation
- **LLM Step Routing**: Steps can use `router = "llm"` for conditional branching (e.g., loop back to planning if review rejects)
- **Multi-Iteration Loops**: Run many iterations back-to-back with `--max-iterations`
- **AI Cycle Selection**: When no fixed cycle is specified, an AI selector picks the best cycle each iteration based on log history and TODO.md state, with customizable selection criteria via `[selector]` config
- **Dependency Triggers**: Cycles can auto-trigger after others (e.g., gardening after coding) with configurable minimum intervals
- **Additive Permissions**: Global + cycle + per-step permissions merged and passed to Claude Code as `--allowedTools`
- **Live Status Bar**: Color-coded health display during execution showing turns, cost, elapsed time, errors, and iteration progress (`[3/10]`) for multi-iteration runs
- **JSONL Logging**: Every cycle outcome is logged to `.flow/log.jsonl` with cost, turns, denials, files changed, tests passed, and timing
- **Progress Tracking**: `.flow/progress.json` provides real-time run status for external monitoring
- **Diagnostics**: `flow doctor` analyzes config and log history for permission issues, high costs, and configuration lint
- **Project Scaffolding**: `flow init` creates a starter `cycles.toml` and `.flow/` directory for new projects
- **Graceful Shutdown**: Ctrl+C cleanly stops execution, kills child processes, and writes final progress status
- **Periodic Run Summaries**: Configurable summary output every N iterations showing cycle breakdown, success rate, and cost
- **Circuit Breakers**: Consecutive tool errors kill runaway cycles; consecutive cycle failures stop the entire run

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

### Initialize a new project

```bash
flow init
```

This creates a `cycles.toml` with coding and gardening cycles and a `.flow/` directory for logs. Edit `cycles.toml` to customize prompts and permissions for your project.

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
| `--cycle <name>` | — | Cycle to execute (optional — AI selector chooses if omitted) |
| `--config <path>` | `cycles.toml` | Path to configuration file |
| `--log-dir <path>` | `.flow` | Directory for log output |
| `--max-iterations <n>` | `1` | Number of iterations to run |
| `--todo <path>` | `TODO.md` | Path to TODO.md for AI cycle selector context |

| Subcommand | Description |
|------------|-------------|
| `doctor` | Analyze config and log history, report findings by severity |
| `init` | Scaffold a new project with `cycles.toml` and `.flow/` directory |

## Configuration

Flow is configured via a TOML file (default: `cycles.toml`).

### Single-step cycle

```toml
[global]
permissions = ["Read", "Glob", "Grep", "Edit(./src/**)", "Bash(cargo *)"]
max_permission_denials = 10        # Stop cycle after this many total denials
circuit_breaker_repeated = 5       # Kill cycle after N consecutive tool errors
max_consecutive_failures = 3       # Stop run after N cycles in a row fail
summary_interval = 5               # Print run summary every N iterations (0 = disabled)

[[cycle]]
name = "gardening"
description = "Refactoring, cleanup, dependency updates"
prompt = """
Your gardening prompt here...
"""
permissions = ["Edit(./Cargo.toml)", "Bash(git *)"]   # Added to global permissions
after = ["coding"]                                      # Auto-triggers after coding
min_interval = 5                                        # But only if 5+ iterations since last run
context = "summaries"                                   # Log context: "full", "summaries", or "none"
```

### Multi-step cycle

Cycles can have multiple steps. Steps within the same cycle can share a Claude Code session via session tags.

```toml
[[cycle]]
name = "coding"
description = "Plan, review plan, then implement"
after = []
context = "summaries"

[[cycle.step]]
name = "plan"
session = "architect"                  # Session tag — shared with plan-review
prompt = "Read TODO.md, write an implementation plan to .flow/current-plan.md"
permissions = ["Edit(./.flow/current-plan.md)"]

[[cycle.step]]
name = "plan-review"
session = "architect"                  # Continues the same conversation
router = "llm"                         # LLM decides: proceed to implement, or loop back to plan
max_visits = 2                         # Prevent infinite loops (default: 3)
prompt = "Review the plan. Write APPROVED or REJECTED to .flow/plan-review.md"
permissions = ["Edit(./.flow/plan-review.md)"]

[[cycle.step]]
name = "implement"
session = "coder"                      # Fresh session for implementation
prompt = "Read the approved plan. Implement with TDD. Commit when done."
permissions = ["Edit(./TODO.md)", "Bash(git *)"]
```

**Session affinity**: Steps with the same `session` tag continue the same Claude Code conversation (via `--resume`). Different tags or no tag start fresh sessions. Sessions do not persist across iterations.

**Step routing**: By default, steps execute sequentially. Set `router = "llm"` on a step to let an LLM decide the next step after it completes — it can jump to any step or declare the cycle done. Use `max_visits` to cap how many times any step can be visited.

### Permission format

Permissions use `ToolName` or `ToolName(specifier)` syntax, matching Claude Code's `--allowedTools` format:

- Bare tool: `"Read"`, `"Glob"`, `"Grep"`, `"Edit"`, `"Write"`, `"Bash"`
- With specifier: `"Edit(./src/**)"`, `"Bash(cargo test *)"`, `"Write(./out.txt)"`

Permissions are **hierarchical and additive**: global + cycle + per-step permissions are merged and deduplicated. Permissions can only be added, never removed.

### Context modes

The `context` field controls how much execution history is injected into cycle prompts:

| Mode | Behavior |
|------|----------|
| `"full"` | Full JSONL log history included |
| `"summaries"` | Summarized history (recommended for most cycles) |
| `"none"` | No history context (default) |

### Selector customization

When running without `--cycle`, Flow uses an AI selector to pick the best cycle each iteration. You can customize the selector's decision criteria with a `[selector]` section:

```toml
[selector]
prompt = "Prefer coding cycles for TODO items. Only run gardening after 3+ coding cycles."
```

The custom prompt replaces the default selection criteria. When omitted or empty, built-in defaults are used.

## How It Works

1. **Load config** — parse `cycles.toml`, validate cycles, steps, and permissions
2. **Select cycle** — either the fixed `--cycle` name, or AI-selected from log history + TODO.md
3. **Resolve permissions** — merge global + cycle + step-specific, deduplicate
4. **Execute steps** — for each step: spawn `claude` CLI with prompt, stream-JSON output, and allowed tools; route to next step
5. **Display** — render live status bar with turn count, cost, elapsed time, and health color
6. **Log** — append `CycleOutcome` to `.flow/log.jsonl` (includes per-step outcomes, files changed, tests passed)
7. **Update progress** — write current run state to `.flow/progress.json`
8. **Gate** — check denial threshold and consecutive failure count; abort if exceeded
9. **Trigger** — find dependent cycles (via `after` + `min_interval` rules)
10. **Repeat** — loop back to step 2 until `--max-iterations` reached, circuit breaker trips, or Ctrl+C

### Observability

**Log file** (`.flow/log.jsonl`): Append-only JSONL with one entry per cycle execution. Each entry includes iteration number, cycle name, outcome, duration, turn count, cost, permission denials, files changed, tests passed, and optional per-step breakdowns.

**Progress file** (`.flow/progress.json`): Written during multi-iteration runs. Contains `started_at`, `current_iteration`, `max_iterations`, `current_cycle`, `current_status` (running/completed/failed/stopped), `cycles_executed` counts, `total_duration_secs`, `total_cost_usd`, and `last_outcome`. Deleted on normal completion. External tools can poll this file to monitor run progress.

**Periodic summaries**: During multi-iteration runs, Flow prints a compact summary every `summary_interval` iterations (default: 5) showing cycle breakdown, success rate, cumulative cost, and elapsed time.

## Diagnostics

`flow doctor` runs read-only analysis and reports findings at three severity levels:

| Code | Severity | What it checks |
|------|----------|----------------|
| D001 | Error | Permission denials in log history (with fix suggestions) |
| D002 | Warning | Repeated cycle failures (>50% failure rate) |
| D003 | Warning | High-cost runs (>$5 per cycle, aggregated) |
| D004 | Info | Triggered cycles missing `min_interval` |
| D005 | Warning | Cycles with no permissions at all |
| D006 | Info | Frequency tuning suggestions |

## Project Structure

```
flow/
├── src/
│   ├── main.rs              # CLI entry point, iteration loop, signal handling
│   ├── lib.rs               # Public library re-exports
│   ├── init.rs              # flow init scaffolding
│   ├── doctor.rs            # Diagnostic engine (D001-D006)
│   ├── testutil.rs          # Shared test helpers
│   ├── cycle/
│   │   ├── config.rs        # TOML config parsing and validation
│   │   ├── executor.rs      # Single-step and multi-step cycle execution
│   │   ├── rules.rs         # Dependency triggers and min_interval logic
│   │   ├── selector.rs      # AI-driven cycle selection
│   │   ├── router.rs        # Step routing (sequential + LLM-driven)
│   │   └── context.rs       # Iteration context injection
│   ├── claude/
│   │   ├── cli.rs           # Claude Code command builder
│   │   ├── permissions.rs   # Permission resolution and merging
│   │   ├── session.rs       # Session manager (tag → ID mapping)
│   │   └── stream.rs        # Stream-JSON event parser
│   ├── cli/
│   │   └── display.rs       # Terminal display, status bar, doctor report
│   └── log/
│       ├── jsonl.rs         # Append-only JSONL logger
│       └── progress.rs      # Real-time progress.json writer
├── tests/
│   └── integration_test.rs  # End-to-end tests
├── cycles.toml              # Cycle configuration
├── AGENTS.md                # Agent context and architecture index
└── TODO.md                  # Task queue
```

## Development

```bash
cargo build                  # Build
cargo test-all               # Run all tests (lib + main + integration)
cargo clippy-all             # Lint with strict warnings
cargo fmt                    # Format

# Quick iteration (lib only)
cargo test --lib && cargo clippy --lib && cargo fmt
```

Custom cargo aliases are defined in `.cargo/config.toml` for `test-all`, `clippy-all`, `check-all`, and `fmt-check`.

## License

MIT License - see [LICENSE](LICENSE) file for details
