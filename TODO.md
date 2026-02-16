# Flow - Task Queue

> **Strategy**: We're building in phases to enable dogfooding ASAP. Phase 1 is the MVP that lets us run single cycles manually. See [plans/002-full-architecture.md](./plans/002-full-architecture.md) for the complete vision.

## 🎯 Phase 1: MVP - Manual Single Cycle (DOGFOOD TARGET)

**Goal**: Execute named cycles manually with basic observability

### 🌱 Meta: First Dogfooding Milestone

- [x] Use Flow to run `/coding-iteration` as a cycle
  - Status: Completed
  - Priority: P0 (Validates entire system)
  - Description: Create a "coding" cycle in cycles.toml that invokes /coding-iteration skill
  - Success: Flow successfully executes `flow --cycle coding`, completes a task, logs to JSONL
  - Dependencies: cycles.toml ✅, config parser ✅, executor ✅, JSONL logger ✅, CLI ✅
  - Completed: 2026-02-15
  - **Result**: First dogfood succeeded — coding cycle wrote 6 integration tests, gardening cycle cleaned up dead pipeline module. Two commits, 68 tests passing.

### 🔧 Post-Dogfood: CLI Output & Runtime Safeguards

> Plan: [wiggly-wandering-corbato.md](../.claude/plans/wiggly-wandering-corbato.md)
> Discovered during first dogfood: raw JSON output, permission issues invisible, result data discarded.

- [x] Fix cycles.toml permission strings (Write → Edit)
  - Status: Completed
  - Priority: P0
  - Completed: 2026-02-16

- [x] Implement stream-JSON parser
  - Status: Completed
  - Priority: P0
  - Files: `src/claude/stream.rs`, `src/claude/mod.rs`
  - Completed: 2026-02-16

- [x] Implement rich CLI display
  - Status: Completed
  - Priority: P0
  - Files: `src/cli/display.rs`, `src/cli/mod.rs`, `Cargo.toml`
  - Completed: 2026-02-16

- [x] Extend CycleOutcome with result blob data
  - Status: Completed
  - Priority: P0
  - Files: `src/log/jsonl.rs`
  - Completed: 2026-02-16

- [x] Extend CycleResult with parsed stream-json fields
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/executor.rs`
  - Completed: 2026-02-16

- [x] Implement execute_with_display() in CycleExecutor
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/executor.rs`
  - Completed: 2026-02-16

- [x] Wire new execution path in main.rs
  - Status: Completed
  - Priority: P0
  - Files: `src/main.rs`
  - Completed: 2026-02-16

- [x] Add safeguard thresholds to GlobalConfig
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/config.rs`
  - Completed: 2026-02-16

- [x] Update integration tests for new CycleResult fields
  - Status: Completed
  - Priority: P0
  - Files: `tests/integration_test.rs`, `src/main.rs`
  - Completed: 2026-02-16

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

- [x] Validate permission strings match `--allowedTools` syntax
  - Status: Completed
  - Priority: P1
  - Files: `src/cycle/config.rs`
  - Description: Reject malformed permission strings at config parse time (must match `ToolName` or `ToolName(specifier)` pattern).
  - Completed: 2026-02-16

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

- [x] Extract outcome summary from Claude's response
  - Status: Completed (basic: exit-code-based success/failure for MVP)
  - Priority: P0
  - Completed: 2026-02-15

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
- [x] Parse `after: [...]` from cycle config
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/rules.rs`
  - Completed: 2026-02-15

- [x] Implement rule evaluator
  - Status: Completed
  - Priority: P0
  - Files: `src/cycle/rules.rs`
  - Completed: 2026-02-15

- [x] Trigger dependent cycles automatically
  - Status: Completed
  - Priority: P0
  - Description: Wired find_triggered_cycles into CLI main loop to auto-run dependent cycles
  - Files: `src/main.rs`
  - Completed: 2026-02-15

### CLI Interface
- [x] Implement `--cycle <name>` argument
  - Status: Completed
  - Priority: P0
  - Files: `src/main.rs`
  - Completed: 2026-02-15

- [x] Fail-fast error handling
  - Status: Completed
  - Priority: P0
  - Files: `src/main.rs` (anyhow with context on all fallible operations)
  - Completed: 2026-02-15

- [ ] Pretty output formatting
  - Status: Superseded by "Implement rich CLI display" in Post-Dogfood section
  - Priority: P1

### Initial Cycles
- [x] Define coding cycle prompt in cycles.toml
  - Status: Completed
  - Priority: P0
  - Files: `cycles.toml`
  - Completed: 2026-02-15

- [x] Define gardening cycle prompt in cycles.toml
  - Status: Completed
  - Priority: P0
  - Files: `cycles.toml`
  - Completed: 2026-02-15

- [ ] Define review cycle prompt in cycles.toml
  - Status: Not Started
  - Priority: P1

- [ ] Define planning cycle prompt in cycles.toml
  - Status: Not Started
  - Priority: P1

### Testing & Validation
- [x] Integration test: Run coding cycle end-to-end
  - Status: Completed (by first dogfood coding cycle)
  - Priority: P0
  - Files: `tests/integration_test.rs` (6 tests)
  - Completed: 2026-02-15

- [x] Test: Gardening auto-triggers after coding
  - Status: Completed (by first dogfood coding cycle)
  - Priority: P0
  - Files: `tests/integration_test.rs`
  - Completed: 2026-02-15

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
- [x] First dogfood: Run `/coding-iteration` as a cycle
  - Status: Completed
  - Priority: P0
  - Success: `flow --cycle coding` completed two tasks, gardening auto-triggered
  - Completed: 2026-02-15

- [ ] Second dogfood: Use Flow to implement next feature
  - Status: Ready
  - Priority: P0
  - Description: Use `flow --cycle coding` to build the next set of improvements. All post-dogfood prerequisites (stream-JSON parser, rich display, safeguards) now complete.
  - Dependencies: All met

- [x] Document learnings and iterate
  - Status: Completed
  - Priority: P0
  - Description: First dogfood learnings captured and acted on. All issues fixed: permission strings corrected, rich display replaces raw JSON, runtime safeguards added (circuit breaker + between-cycle gate).
  - Completed: 2026-02-16

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

### Multi-Cycle Health Tracking (Safeguard Level 4)
- [ ] Track cumulative health across iterations
  - Priority: P0
  - Description: If N cycles in a row fail or have high denial rates, stop the whole run. Builds on Phase 1 safeguard thresholds.

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

### 2026-02-16 - Permission String Validation

**Completed:**
- [x] Validate permission strings match `--allowedTools` syntax

**Implementation:**
- Files: `src/cycle/config.rs`
- Tests: 10 new tests (7 rejection + 3 valid), 109 lib tests total
- Validates at config parse time: tool name must start uppercase, specifier in parens must be non-empty, no trailing text

### 2026-02-16 - Post-Dogfood: Rich Display & Runtime Safeguards

**Completed (all 9 P0 tasks in one iteration):**
- [x] Fix cycles.toml permission strings (Write → Edit)
- [x] Add safeguard thresholds to GlobalConfig (max_permission_denials, circuit_breaker_repeated)
- [x] Extend CycleResult with rich fields (result_text, num_turns, total_cost_usd, permission_denial_count)
- [x] Extend CycleOutcome with rich fields + backward-compatible serde defaults
- [x] Implement stream-JSON parser (21 tests)
- [x] Implement rich CLI display with colored output (12 tests)
- [x] Implement execute_with_display() with circuit breaker (2 tests)
- [x] Wire new execution path in main.rs with between-cycle gate
- [x] Update all integration tests for new fields

**Results:**
- 6 commits, 110 tests passing (99 lib + 5 main + 6 integration)
- New components: stream-JSON parser, CLI display, execute_with_display
- Runtime safeguards: mid-cycle circuit breaker, between-cycle denial gate
- All output now human-readable with colored formatting

### 2026-02-15 - First Dogfood Run

**Completed:**
- [x] First dogfood: `flow --cycle coding` ran successfully
- [x] Coding cycle: wrote 6 integration tests, made run_command() public
- [x] Gardening cycle: auto-triggered, removed dead pipeline module, cleaned up code
- [x] Integration tests completed by Flow itself

**Results:**
- Two commits produced autonomously (e4c192f, afc075f)
- 68 tests passing (58 lib + 4 main + 6 integration)
- Coding cycle: 53 turns, $2.15, 4m 13s
- Gardening cycle: 70 turns, $2.37, 4m 15s
- 15 wasted turns on permission denials (~12% overhead)

**Learnings:**
- `Write(./TODO.md)` should be `Edit(./TODO.md)` — Edit is for modifying existing files
- Raw stream-json stdout is unreadable for humans
- Result blob contains rich data (summary, cost, turns, permission_denials) that was discarded
- Need runtime safeguards for unattended operation (circuit breaker, between-cycle gate)

### 2026-02-15 - CLI Interface & Dogfooding Prerequisites

**Completed:**
- [x] Implement `--cycle <name>` CLI argument (clap derive)
- [x] Fail-fast error handling (anyhow with context)
- [x] Define coding cycle prompt in cycles.toml
- [x] Define gardening cycle prompt in cycles.toml
- [x] Wire auto-trigger of dependent cycles
- [x] Basic outcome extraction (exit-code-based)
- [x] JSONL logging integration in main loop

**Implementation:**
- Files: `src/main.rs`, `cycles.toml`
- Tests: 4 new tests (build_outcome success/failure/signal, available_cycle_names)
- Total: 62 tests passing (58 lib + 4 main)

### 2026-02-15 - Cycle Executor

**Completed:**
- [x] Implement CycleExecutor struct
- [x] Stream stdout/stderr to terminal (real-time)
- [x] Capture exit code and detect failures

**Implementation:**
- Files: `src/cycle/executor.rs`, `src/cycle/mod.rs`, `src/lib.rs`
- Tests: 12 comprehensive tests passing (6 prepare + 6 run_command)

### 2026-02-15 - Claude CLI Builder

**Completed:**
- [x] Build Claude Code CLI command with --allowedTools flags

**Implementation:**
- Files: `src/claude/cli.rs`, `src/claude/mod.rs`, `src/lib.rs`
- Tests: 8 comprehensive tests passing (includes --verbose flag)

### 2026-02-15 - Cycle Rules Engine

**Completed:**
- [x] Parse `after: [...]` from cycle config
- [x] Implement rule evaluator

**Implementation:**
- Files: `src/cycle/rules.rs`, `src/cycle/mod.rs`, `src/lib.rs`
- Tests: 8 comprehensive tests passing

### 2026-02-14 - Cycle Config Parser

**Completed:**
- [x] Define cycles.toml schema
- [x] Implement TOML parser for cycle definitions
- [x] Validate cycle configuration on load

**Implementation:**
- Files: `src/cycle/config.rs`, `src/cycle/mod.rs`, `src/lib.rs`
- Tests: 17 comprehensive tests passing

### 2026-02-14 - JSONL Logger

**Completed:**
- [x] Implement .flow/log.jsonl writer
- [x] Append-only log entries
- [x] Serialize cycle outcomes to JSON
- [x] Test: JSONL logging works correctly

**Implementation:**
- Files: `src/log/jsonl.rs`, `src/log/mod.rs`
- Tests: 6 comprehensive tests passing

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
