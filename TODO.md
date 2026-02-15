# Flow - Task Queue

> **Strategy**: We're building in phases to enable dogfooding ASAP. Phase 1 is the MVP that lets us run single cycles manually. See [plans/002-full-architecture.md](./plans/002-full-architecture.md) for the complete vision.

## 🎯 Phase 1: MVP - Manual Single Cycle (DOGFOOD TARGET)

**Goal**: Execute named cycles manually with basic observability

### 🌱 Meta: First Dogfooding Milestone

- [ ] Use Flow to run `/coding-iteration` as a cycle
  - Status: Not Started
  - Priority: P0 (Validates entire system)
  - Description: Create a "coding" cycle in cycles.toml that invokes /coding-iteration skill
  - Success: Flow successfully executes `flow --cycle coding`, completes a task, logs to JSONL
  - Dependencies: cycles.toml, config parser, executor, JSONL logger ✅, CLI
  - **Why First**: Proves Flow can build Flow - validates core concept

### Cycle Configuration
- [x] [Define cycles.toml schema](./plans/002-full-architecture.md#1-cycle-configuration-cyclestoml)
  - Status: Completed
  - Priority: P0
  - Files: `cycles.toml`, `src/cycle/config.rs`
  - Completed: 2026-02-14

- [x] Implement TOML parser for cycle definitions
  - Status: Completed
  - Priority: P0
  - Dependencies: cycles.toml schema
  - Completed: 2026-02-14

- [x] Implement permission resolver (global + per-cycle, additive)
  - Status: Completed
  - Priority: P0
  - Files: `src/claude/permissions.rs`
  - Completed: 2026-02-14

- [x] Validate cycle configuration on load
  - Status: Completed
  - Priority: P0
  - Completed: 2026-02-14

- [ ] Validate permission strings match `--allowedTools` syntax
  - Status: Not Started
  - Priority: P1
  - Files: `src/cycle/config.rs`
  - Description: Reject malformed permission strings at config parse time (must match `ToolName` or `ToolName(specifier)` pattern). Implement when building CLI builder.

### Cycle Executor
- [x] Implement CycleExecutor struct
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/executor.rs`
  - Completed: 2026-02-15

- [x] Build Claude Code CLI command with --allowedTools flags
  - Status: Completed
  - Priority: P0
  - Files: `src/claude/cli.rs`
  - Completed: 2026-02-15

- [x] Stream stdout/stderr to terminal (real-time)
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/executor.rs` (built into executor via async line-by-line streaming)
  - Completed: 2026-02-15

- [x] Capture exit code and detect failures
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/executor.rs` (CycleResult.exit_code and .success)
  - Completed: 2026-02-15

- [ ] Extract outcome summary from Claude's response
  - Status: Not Started
  - Priority: P0

### JSONL Logger
- [x] Implement .flow/log.jsonl writer
  - Status: Completed
  - Priority: P0
  - Files: `src/log/jsonl.rs`
  - Completed: 2026-02-14

- [x] Append-only log entries
  - Status: Completed
  - Priority: P0
  - Completed: 2026-02-14

- [x] Serialize cycle outcomes to JSON
  - Status: Completed
  - Priority: P0
  - Completed: 2026-02-14

### Cycle Rules Engine
- [ ] Parse `after: [...]` from cycle config
  - Status: Not Started
  - Priority: P0
  - Files: `src/cycle/rules.rs`

- [ ] Implement rule evaluator
  - Status: Not Started
  - Priority: P0

- [ ] Trigger dependent cycles automatically
  - Status: Not Started
  - Priority: P0

### CLI Interface
- [ ] Implement `--cycle <name>` argument
  - Status: Not Started
  - Priority: P0
  - Files: `src/main.rs`

- [ ] Fail-fast error handling
  - Status: Not Started
  - Priority: P0

- [ ] Pretty output formatting
  - Status: Not Started
  - Priority: P1

### Initial Cycles
- [ ] Define coding cycle prompt in cycles.toml
  - Status: Not Started
  - Priority: P0

- [ ] Define gardening cycle prompt in cycles.toml
  - Status: Not Started
  - Priority: P0

- [ ] Define review cycle prompt in cycles.toml
  - Status: Not Started
  - Priority: P1

- [ ] Define planning cycle prompt in cycles.toml
  - Status: Not Started
  - Priority: P1

### Testing & Validation
- [ ] Integration test: Run coding cycle end-to-end
  - Status: Not Started
  - Priority: P0
  - Files: `tests/integration_test.rs`

- [ ] Test: Gardening auto-triggers after coding
  - Status: Not Started
  - Priority: P0

- [x] Test: Permission resolution (additive)
  - Status: Completed
  - Priority: P0
  - Files: `src/claude/permissions.rs` (7 unit tests)
  - Completed: 2026-02-15

- [x] Test: JSONL logging works correctly
  - Status: Completed
  - Priority: P0
  - Completed: 2026-02-14

### 🌱 Dogfooding Milestone
- [ ] First dogfood: Run `/coding-iteration` as a cycle
  - Status: Not Started
  - Priority: P0
  - Description: See "Meta: First Dogfooding Milestone" at top of Phase 1
  - Success: `flow --cycle coding` completes one task end-to-end

- [ ] Second dogfood: Use Flow to implement next feature
  - Status: Not Started
  - Priority: P0
  - Description: Once basic MVP works, use `flow --cycle coding` to build Phase 2 features
  - Dependencies: First dogfood complete

- [ ] Document learnings and iterate
  - Status: Not Started
  - Priority: P0
  - Description: After dogfooding, update cycles, prompts, permissions based on what worked/didn't

---

## 🚀 Phase 2: Automation (After Dogfooding Phase 1)

**Goal**: Autonomous multi-iteration runs with smart cycle selection

### Cycle Selector
- [ ] Implement selector using Claude Sonnet API
  - Priority: P0

- [ ] Selection logic (balance + context + priority)
  - Priority: P0

- [ ] JSONL log summarizer for selector context
  - Priority: P0

- [ ] TODO.md parser for task priorities
  - Priority: P0

### Multi-Iteration Loop
- [ ] Implement `--max-iterations` support
  - Priority: P0

- [ ] Iteration counter and loop control
  - Priority: P0

- [ ] Stop conditions (max iterations, errors)
  - Priority: P0

### Iteration Context
- [ ] Implement context modes (full, summaries, none)
  - Priority: P0

- [ ] Optional CLI tool cycles can invoke for context
  - Priority: P1

- [ ] Context injection into cycle prompts
  - Priority: P0

### Enhanced Observability
- [ ] Implement .flow/progress.json writer
  - Priority: P0

- [ ] Periodic summary output (every N iterations)
  - Priority: P1

- [ ] Cycle balance statistics
  - Priority: P1

---

## 🔮 Phase 3: Advanced Features (Future)

- [ ] Template prompts with variables
- [ ] Multi-part prompts (system + user)
- [ ] Per-cycle timeout configuration
- [ ] Cost tracking (per cycle, per iteration, global)
- [ ] Parallel cycle execution
- [ ] Enhanced outcome capture (git, tests, lints)
- [ ] Recovery strategies

---

## ✅ Completed

### 2026-02-14 - Cycle Config Parser

**Completed:**
- [x] Define cycles.toml schema
- [x] Implement TOML parser for cycle definitions
- [x] Validate cycle configuration on load

**Implementation:**
- Files: `src/cycle/config.rs`, `src/cycle/mod.rs`, `src/lib.rs`
- Tests: 17 comprehensive tests passing
- Coverage: FlowConfig, GlobalConfig, CycleConfig structs, ContextMode enum, parse/validate/lookup
- Validation: duplicate names, unknown `after` references, empty names, missing fields, invalid TOML

**Notes:**
- Serde-based TOML deserialization with custom validation
- Default context mode is `none`, permissions default to empty vec
- Multiline TOML prompts supported
- `get_cycle()` lookup by name
- `from_path()` for file-based loading

### 2026-02-15 - Cycle Executor

**Completed:**
- [x] Implement CycleExecutor struct
- [x] Stream stdout/stderr to terminal (real-time)
- [x] Capture exit code and detect failures

**Implementation:**
- Files: `src/cycle/executor.rs`, `src/cycle/mod.rs`, `src/lib.rs`
- Tests: 12 comprehensive tests passing (6 prepare + 6 run_command)
- Coverage: `CycleExecutor` with `prepare()` and `execute()`, `PreparedCycle`, `CycleResult`, `run_command()` async subprocess runner

**Notes:**
- `prepare()` validates cycle exists and resolves permissions (testable without subprocess)
- `run_command()` async function handles subprocess execution with tokio
- Concurrent stdout/stderr reading with real-time terminal forwarding
- Line-by-line streaming via `tokio::io::BufReader`
- Duration tracking with `std::time::Instant`
- Re-exported `CycleExecutor`, `CycleResult`, `PreparedCycle` from `lib.rs`

### 2026-02-15 - Claude CLI Builder

**Completed:**
- [x] Build Claude Code CLI command with --allowedTools flags

**Implementation:**
- Files: `src/claude/cli.rs`, `src/claude/mod.rs`, `src/lib.rs`
- Tests: 7 comprehensive tests passing
- Coverage: `build_command()` function — constructs `std::process::Command` with `-p`, `--output-format stream-json`, and `--allowedTools` flags

**Notes:**
- Verified actual Claude Code CLI flags via documentation before implementing
- Each permission passed as separate arg after `--allowedTools`
- `--allowedTools` omitted when no permissions provided
- `--output-format stream-json` for structured streaming output
- Re-exported `build_command` from `lib.rs`

### 2026-02-14 - JSONL Logger

**Completed:**
- [x] Implement .flow/log.jsonl writer
- [x] Append-only log entries
- [x] Serialize cycle outcomes to JSON
- [x] Test: JSONL logging works correctly

**Implementation:**
- Files: `src/log/jsonl.rs`, `src/log/mod.rs`
- Tests: 6 comprehensive tests passing
- Coverage: CycleOutcome struct, JsonlLogger with create/append/read operations
- Commit: "Implement JSONL logger for cycle execution history"

**Notes:**
- Added chrono dependency for ISO 8601 timestamps
- Added toml dependency for future cycle config parsing
- Used tempfile crate for isolated test environments
- Full error handling with anyhow::Result
- Zero clippy warnings with strict linting
- Implements append-only JSONL format for cycle execution history

**Workflow Automation Added:**
- Pre-commit git hook for automatic validation
- `/coding-iteration` skill for structured TDD workflow
- `/reflect` skill for iteration retrospectives
- `/update-todos` skill for keeping TODO.md synchronized

---

## Task Management

This file serves as a simple task tracker. Each task links to a detailed plan in `plans/*.md`.

### Task States
- **Not Started**: Task defined but no work begun
- **In Progress**: Actively being worked on
- **Blocked**: Waiting on dependencies or decisions
- **Completed**: Fully implemented and tested

### Priority Levels
- **P0**: Critical for MVP
- **P1**: Important for v1.0
- **P2**: Nice to have
- **P3**: Future consideration
