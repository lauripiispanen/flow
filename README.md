# Flow

> The Makefile for AI development — encode your team's development process as code.

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

AI coding agents are good at executing tasks. What they lack is judgment about *what type of work to do next*. Left unmanaged, agents drift toward feature work and neglect maintenance, review, documentation, and code quality — the same pattern human teams fall into without process discipline.

Flow fixes this. You define your development process in a single `cycles.toml` — what types of work exist, how they're balanced, what permissions each phase gets, what quality gates must pass — and Flow enforces it across autonomous agent sessions. It's the policy layer between a company-level orchestrator that dispatches work and the AI agents that execute it.

```
Company orchestrator (Honk, GitHub Actions, custom)
  → dispatches work items to repos
    → Flow (cycles.toml — per-repo development policy)
      → enforces cycle balance, quality gates, permissions
        → Claude Code / Agent Teams (execution)
```

## What Makes Flow Different

**Development methodology as code.** `cycles.toml` is a declarative config that encodes your team's process: plan, implement, review, garden. No other tool treats the methodology itself as a versionable, shareable artifact.

**Investment balance.** The AI cycle selector ensures coding, gardening, review, and planning are balanced according to your configuration — not left to chance. Feature work doesn't crowd out maintenance. Quality work isn't an afterthought.

**Permission scoping per phase.** The architect can read but not edit. The coder can edit `src/` but not CI config. The gardener can update deps but not rewrite features. Permissions are hierarchical and additive — cycles can only add permissions, never remove the baseline.

**Guardrails for unattended operation.** Circuit breakers, denial gates, consecutive failure limits, and per-cycle cost caps make it safe to `flow --max-iterations 50` and walk away. When something goes wrong, Flow stops — it doesn't flail.

**Cross-run observability.** JSONL logs, progress tracking, `flow doctor` diagnostics, and periodic summaries give you a structured view across dozens of iterations. Know what happened, what it cost, and what went wrong — without reading raw transcripts.

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

### Initialize a project

```bash
flow init
```

This creates a `cycles.toml` with coding and gardening cycles and a `.flow/` directory for logs. Edit `cycles.toml` to define your team's development process.

### Usage

```bash
# Run a single named cycle
flow --cycle coding

# Run 10 iterations with AI-driven cycle selection
flow --max-iterations 10

# Run diagnostics on config and history
flow doctor

# Auto-fix permission and config issues
flow doctor --repair
```

### CLI Reference

| Flag | Default | Description |
|------|---------|-------------|
| `--cycle <name>` | — | Cycle to execute (AI selector chooses if omitted) |
| `--config <path>` | `cycles.toml` | Path to configuration file |
| `--log-dir <path>` | `.flow` | Directory for log output |
| `--max-iterations <n>` | `1` | Number of iterations to run |
| `--todo <path>` | `TODO.md` | Path to TODO.md for cycle selector context |

| Subcommand | Description |
|------------|-------------|
| `doctor` | Analyze config and log history, report findings by severity |
| `doctor --repair` | Auto-fix safe issues (missing permissions, missing `min_interval`) |
| `init` | Scaffold a new project with `cycles.toml` and `.flow/` directory |

## Configuration

Flow is configured via `cycles.toml` — the "Makefile" that encodes your development process.

### Defining cycles

Each cycle represents a type of work. Together, they describe your complete development methodology.

```toml
[global]
permissions = ["Read", "Glob", "Grep", "Edit(./src/**)", "Bash(cargo *)"]
max_permission_denials = 10        # Stop cycle after this many total denials
circuit_breaker_repeated = 5       # Kill cycle after N consecutive tool errors
max_consecutive_failures = 3       # Stop run after N cycles in a row fail
summary_interval = 5               # Print run summary every N iterations

[[cycle]]
name = "coding"
description = "Implement features from TODO.md using TDD"
prompt = "Your coding prompt here..."
permissions = ["Edit(./TODO.md)", "Bash(git *)"]

[[cycle]]
name = "gardening"
description = "Refactoring, cleanup, dependency updates"
prompt = "Your gardening prompt here..."
permissions = ["Edit(./Cargo.toml)", "Bash(git *)"]
after = ["coding"]                   # Auto-triggers after coding cycles
min_interval = 5                     # But only if 5+ iterations since last run
context = "summaries"                # Inject summarized history into prompt
```

### Multi-step cycles

Cycles can have multiple sequential steps with session affinity — e.g., plan, review the plan, then implement.

```toml
[[cycle]]
name = "coding"
description = "Plan, review plan, implement, then reflect"

[[cycle.step]]
name = "plan"
session = "architect"                  # Session tag — shared with plan-review
prompt = "Read TODO.md, write an implementation plan to .flow/current-plan.md"
permissions = ["Edit(./.flow/current-plan.md)"]

[[cycle.step]]
name = "plan-review"
session = "architect"                  # Continues the same conversation
router = "llm"                         # LLM decides: proceed or loop back
max_visits = 2                         # Prevent infinite loops
prompt = "Review the plan. Write APPROVED or REJECTED."
permissions = ["Edit(./.flow/plan-review.md)"]

[[cycle.step]]
name = "implement"
session = "coder"                      # Fresh session for implementation
prompt = "Read the approved plan. Implement with TDD. Commit when done."
permissions = ["Edit(./TODO.md)", "Bash(git *)"]
```

**Session affinity**: Steps with the same `session` tag continue the same Claude Code conversation (via `--resume`). Different tags start fresh sessions. Sessions do not persist across iterations.

**Step routing**: By default, steps execute sequentially. Set `router = "llm"` to let an LLM decide the next step — it can jump to any step or declare the cycle done. Use `max_visits` to cap revisits.

### Selector customization

When running without `--cycle`, Flow uses an AI selector to pick the best cycle each iteration. Customize its priorities:

```toml
[selector]
prompt = "Prefer coding cycles for TODO items. Only run gardening after 3+ coding cycles."
```

### Permission format

Permissions use `ToolName` or `ToolName(specifier)` syntax, matching Claude Code's `--allowedTools` format:

- Bare tool: `"Read"`, `"Glob"`, `"Grep"`, `"Edit"`, `"Write"`, `"Bash"`
- With specifier: `"Edit(./src/**)"`, `"Bash(cargo test *)"`, `"Write(./out.txt)"`

Permissions are **hierarchical and additive**: global + cycle + per-step permissions are merged. Permissions can only be added, never removed — a safety property that ensures baseline protections always apply.

### Context modes

The `context` field controls how much execution history is injected into cycle prompts:

| Mode | Behavior |
|------|----------|
| `"full"` | Full JSONL log history included |
| `"summaries"` | Summarized history (recommended for most cycles) |
| `"none"` | No history context (default) |

## How It Works

1. **Load config** — parse `cycles.toml`, validate cycles, steps, and permissions
2. **Select cycle** — either the fixed `--cycle` name, or AI-selected based on log history, TODO.md state, and configured balance priorities
3. **Resolve permissions** — merge global + cycle + step-specific, deduplicate
4. **Execute steps** — spawn `claude` CLI with prompt, permissions, and session affinity; route between steps
5. **Log** — append outcome to `.flow/log.jsonl` (cost, turns, denials, files changed, tests passed)
6. **Gate** — check denial threshold and consecutive failure count; abort if exceeded
7. **Trigger** — find dependent cycles (via `after` + `min_interval` rules)
8. **Repeat** — loop back to step 2 until `--max-iterations` reached or circuit breaker trips

### Observability

**Log file** (`.flow/log.jsonl`): Append-only JSONL with one entry per cycle. Each entry includes iteration number, cycle name, outcome, duration, turn count, cost, permission denials, files changed, tests passed, and optional per-step breakdowns.

**Progress file** (`.flow/progress.json`): Written during multi-iteration runs. Contains run state, current iteration, cycle breakdown, costs. External tools can poll this to monitor progress.

**Periodic summaries**: Compact summary every `summary_interval` iterations showing cycle breakdown, success rate, cumulative cost, and elapsed time.

**Diagnostics** (`flow doctor`):

| Code | Severity | What it checks | `--repair` |
|------|----------|----------------|------------|
| D001 | Error | Permission denials in log history | Auto-fix |
| D002 | Warning | Repeated cycle failures (>50% failure rate) | — |
| D003 | Warning | High-cost runs (>$5 per cycle) | — |
| D004 | Info | Triggered cycles missing `min_interval` | Auto-fix |
| D005 | Warning | Cycles with no permissions at all | — |
| D006 | Info | Frequency tuning suggestions | — |

## Project Structure

```
flow/
├── src/
│   ├── main.rs              # CLI entry point, iteration loop, signal handling
│   ├── lib.rs               # Public library re-exports
│   ├── init.rs              # flow init scaffolding
│   ├── doctor.rs            # Diagnostic engine (D001-D006)
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
├── cycles.toml              # Development process configuration
├── AGENTS.md                # Agent context and architecture index
└── TODO.md                  # Task queue
```

## Development

```bash
cargo build                  # Build
cargo test-all               # Run all tests (lib + main + integration)
cargo clippy-all             # Lint with strict warnings
cargo fmt                    # Format
```

Custom cargo aliases are defined in `.cargo/config.toml` for `test-all`, `clippy-all`, `check-all`, and `fmt-check`.

## License

MIT License - see [LICENSE](LICENSE) file for details
