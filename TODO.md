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

- [x] Second dogfood: Use Flow to implement next feature
  - Status: Completed
  - Priority: P0
  - Completed: 2026-02-16
  - **Result**: Coding cycle validated permission strings (38 turns, $1.44, 197s, 0 denials). Gardening auto-triggered and cleaned dead code, unused dep, added 4 tests (65 turns, $2.65, 380s, 0 denials). Tests: 120→124. Zero permission denials (vs 15 in dogfood 1).
  - **Learnings**:
    - Permission denial fix was the biggest win (15→0 denials, coding got cheaper/faster)
    - Gardening auto-triggers every time — needs frequency constraints to avoid redundant runs
    - `files_changed` and `tests_passed` fields are never populated (always `[]`/`0`)
    - Need clearer terminology hierarchy (step→cycle→iteration→run)

- [x] Document learnings and iterate
  - Status: Completed
  - Priority: P0
  - Description: First dogfood learnings captured and acted on. All issues fixed: permission strings corrected, rich display replaces raw JSON, runtime safeguards added (circuit breaker + between-cycle gate).
  - Completed: 2026-02-16

---

## 🚀 Phase 2: Automation (After Dogfooding Phase 1)

**Goal**: Multi-step cycles, frequency-aware triggering, autonomous multi-iteration runs with smart cycle selection

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

### Cycle Frequency Constraints
- [x] Add `min_interval` config field for cycle triggering rules
  - Status: Completed
  - Priority: P0
  - Description: Added `min_interval: Option<u32>` to `CycleConfig`. Specifies minimum iterations since last run before a cycle can be auto-triggered. None = no constraint (backward compatible).
  - Files: `src/cycle/config.rs`
  - Tests: 3 new tests (default none, parsed from config, zero is valid)
  - Completed: 2026-02-16

- [x] Rules engine checks last-run time before triggering
  - Status: Completed
  - Priority: P0
  - Description: `find_triggered_cycles()` now accepts a `&[CycleOutcome]` log slice and checks each candidate cycle's last-run iteration against its `min_interval` before triggering. Falls back to always-trigger if no constraint set (backward compatible).
  - Files: `src/cycle/rules.rs`, `src/main.rs`, `tests/integration_test.rs`
  - Tests: 6 new tests (blocks too recent, allows enough elapsed, allows never ran, no constraint always triggers, zero always triggers, boundary exact match)
  - Completed: 2026-02-16

### Multi-Step Cycles (Session Reuse)
- [x] Design multi-step cycle config format
  - Status: Completed
  - Priority: P0
  - Plan: [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md)
  - Description: Cycles can have multiple steps, each a separate Claude Code invocation. Steps with the same session tag reuse the same session (via `--continue`/`--resume`). Enables patterns like architect→coder→architect-review.
  - Completed: 2026-02-16

- [ ] Implement step executor with session affinity
  - Priority: P0
  - Description: Execute steps sequentially within a cycle. Track session IDs. Steps with matching session tags continue the same Claude Code session.

- [ ] Per-step permissions
  - Priority: P1
  - Description: Each step can have its own permissions (additive on top of cycle + global). Architect step may be read-only while coder step has write access.

### Outcome Data Completeness
- [ ] Populate `files_changed` from stream data or git diff
  - Priority: P1
  - Description: `files_changed` is always empty in log entries. Parse from stream events (tool use of Edit/Write) or run `git diff --name-only` after cycle completes.

- [ ] Populate `tests_passed` from stream data or cargo output
  - Priority: P1
  - Description: `tests_passed` is always 0. Parse test count from Bash tool results in stream or run `cargo test` count after cycle.

### Multi-Cycle Health Tracking (Safeguard Level 4)
- [ ] Track cumulative health across iterations
  - Priority: P0
  - Description: If N cycles in a row fail or have high denial rates, stop the whole run. Builds on Phase 1 safeguard thresholds.

### Status Bar (Live Run Display)
- [ ] Implement persistent status bar at terminal bottom during cycle execution
  - Priority: P0
  - Description: A single persistent line at the bottom of the terminal showing live stats while the scrolling event log continues above. Use ANSI escape codes (save/restore cursor position) — no full TUI framework needed for v1.
  - Display: `[cycle_name] ▶ 12 turns | $1.23 | 2m 15s | 0 errors`
  - Files: `src/cli/display.rs`, `src/cycle/executor.rs`
  - Implementation: Update the status line on each StreamEvent. On ToolUse increment turn proxy, on ToolResult(error) increment error count, on Result update final stats. Use `\x1b[s` / `\x1b[u` (save/restore cursor) + `\x1b[999;1H` (move to bottom) to write the bar, then restore cursor for normal scrolling.
  - Inspiration: GSD's statusline hook shows context window usage with color-coded severity bars.

- [ ] Color-code status bar based on health
  - Priority: P1
  - Description: Green = healthy (0 errors), yellow = warning (1-2 errors), red = critical (3+ errors or circuit breaker close). Show cost in yellow if exceeding expected range.

### `flow doctor` Command (Log Analysis & Diagnostics)
- [ ] Implement `flow doctor` subcommand
  - Priority: P0
  - Description: Analyze `.flow/log.jsonl` and `cycles.toml` to diagnose issues and suggest fixes. Returns structured report with categories: errors (must fix), warnings (should fix), info (suggestions).
  - Files: `src/doctor.rs` (new), `src/main.rs`
  - Checks:
    - Permission analysis: scan logs for denial counts > 0, suggest exact permission strings to add
    - Cycle health: flag cycles that consistently fail, have high turn counts, or cost anomalies
    - Config lint: warn about cycles with `after = []` that might want dependencies, missing `min_interval` on triggered cycles
    - Frequency tuning: suggest `min_interval` values based on actual run frequency
    - Stale state: warn if `.flow/log.jsonl` has entries older than N days with no recent runs
  - Inspiration: GSD's `/gsd:health` returns structured JSON with error codes (E001-E005), repairable flags, and auto-repair actions.

- [ ] Store full `permission_denials` list in `CycleOutcome` (not just count)
  - Priority: P0
  - Description: Currently `CycleOutcome` only stores `permission_denial_count: Option<u32>`. The stream result event contains `permission_denials: Vec<String>` with the actual denied tool names. Store the full list so `flow doctor` can suggest exact permission fixes.
  - Files: `src/log/jsonl.rs`, `src/main.rs`

- [ ] `flow doctor --repair` auto-fix mode
  - Priority: P1
  - Description: Auto-apply safe fixes (add missing permissions to cycles.toml, set recommended min_interval values). Only for non-destructive changes. Report what was changed.

### `flow plan` Command (Idea Capture & Deep Planning)
- [ ] Implement `flow plan` subcommand with two modes
  - Priority: P1
  - Description: Both modes invoke Claude Code to process input and update project plan artifacts (TODO.md, cycles.toml, plans/). This is a meta command — not a cycle, but uses Claude Code under the hood.
  - Files: `src/plan.rs` (new), `src/main.rs`

- [ ] Quick mode: `flow plan '<idea>'`
  - Priority: P1
  - Description: One-shot idea capture. Pass a short description as a positional argument. Claude Code reads current TODO.md and project context, then appends a well-scoped task (or set of tasks) to TODO.md. For rapid idea capture or small project edits without ceremony.
  - Usage: `flow plan 'add bookmarks for saved pages'` or `flow plan 'refactor auth to use JWT'`
  - Output: Updated TODO.md with new tasks scoped for coding cycles. Prints what was added.

- [ ] Interactive mode: `flow plan` (no arguments)
  - Priority: P1
  - Description: Deep idea rumination. Claude Code enters a conversational planning session — reads project context, asks clarifying questions about gray areas, explores tradeoffs, and captures decisions. Produces structured output: TODO.md tasks grouped into phases, dependency ordering, optional new cycle definitions, and optional plans/ spec files. Inspired by GSD's discuss-phase pattern (locked decisions, discretion areas, deferred ideas).
  - Usage: `flow plan` (launches interactive Claude Code session with planning prompt)
  - Output: Updated TODO.md, optionally new plans/*.md files, optionally suggested cycles.toml additions

- [ ] Plan decomposition with task scoping
  - Priority: P1
  - Description: Both modes should produce tasks completable in a single coding cycle. Estimate relative complexity. Identify dependencies between tasks. Group into phases when appropriate.

### `flow init` Command (Project Scaffolding)
- [ ] Implement `flow init` subcommand for new project setup
  - Priority: P1
  - Description: Scaffold a new Flow-managed project. Creates `cycles.toml` with sensible defaults, `.flow/` directory, and optionally a starter `TODO.md`. For v1, use a static template with coding + gardening cycles and reasonable global permissions.
  - Files: `src/init.rs` (new), `src/main.rs`
  - Usage: `flow init` (in project root)
  - Output: `cycles.toml`, `.flow/` dir, optional `TODO.md` scaffold
  - Creates: global permissions (Read, Glob, Grep), coding cycle (with TDD prompt), gardening cycle (with maintenance prompt)

- [ ] Interactive init with cycle selection (Phase 3)
  - Priority: P2
  - Description: `flow init` prompts user for what kinds of cycles to set up. Could detect project type (Rust/JS/Python) from existing files and suggest appropriate permissions and cycle prompts. Uses Claude Code to generate tailored cycle definitions based on the project's stack and structure.
  - Usage: `flow init --interactive` or just `flow init` detects no existing config and offers interactive setup

### Review & Planning Cycles in cycles.toml
- [x] Define review cycle prompt in cycles.toml
  - Status: Completed
  - Priority: P1
  - Description: Read-only goal-backward verification cycle. After coding, verifies: EXISTS (files/functions/tests exist), SUBSTANTIVE (real implementation, not stubs), WIRED (connected to entry points).
  - Completed: 2026-02-16
  - Inspiration: GSD's gsd-verifier starts from what SHOULD be true and works backwards through artifacts.

- [x] Define planning cycle prompt in cycles.toml
  - Status: Completed
  - Priority: P1
  - Description: Analyzes TODO.md, recent logs, project state. Re-prioritizes tasks, adds new discoveries, ensures tasks are scoped for single coding cycles.
  - Completed: 2026-02-16

---

## 🔮 Phase 3: Advanced Features (Future)

- [ ] Template prompts with variables
- [ ] Multi-part prompts (system + user)
- [ ] Per-cycle timeout configuration
- [ ] Cost tracking (per cycle, per iteration, global)
- [ ] Parallel cycle execution (wave-based: group independent cycles, run in parallel)
- [ ] Enhanced outcome capture (git, tests, lints)
- [ ] Recovery strategies (auto-retry with deviation rules, max 3 fix attempts per issue)
- [ ] Model profiles (different models for coding/review/planning — e.g., Opus for coding, Sonnet for review)
- [ ] State file (`.flow/state.md`) — compact living memory read/written by each cycle for cross-iteration continuity
- [ ] Pause/resume support — serialize run state to disk, allow `flow resume` to pick up where it left off
- [ ] Goal-backward gap closure loop — verify → plan gaps → execute gaps → re-verify as a first-class workflow
- [ ] Codebase mapping command — parallel analysis of existing codebase (stack, architecture, conventions, concerns) before first planning cycle
- [ ] Context window awareness — track approximate token usage across cycles, warn when approaching limits

---

## ✅ Completed

### 2026-02-16 - Cycle Frequency Constraints

**Completed:**
- [x] Add `min_interval` config field to `CycleConfig`
- [x] Rules engine checks log.jsonl before triggering dependent cycles
- [x] Wire frequency-aware triggering into main loop
- [x] Update integration tests for new `find_triggered_cycles` signature

**Implementation:**
- Files: `src/cycle/config.rs`, `src/cycle/rules.rs`, `src/main.rs`, `tests/integration_test.rs`, `cycles.toml`
- Tests: 9 new tests (3 config + 6 rules), 133 total (122 lib + 5 main + 6 integration)
- `find_triggered_cycles()` now accepts `&[CycleOutcome]` log history
- Gardening cycle configured with `min_interval = 3` and `after = ["coding"]` re-enabled

### 2026-02-16 - Second Dogfood Run

**Completed:**
- [x] Second dogfood: `flow --cycle coding` ran successfully
- [x] Coding cycle: validated permission strings at config parse time (10 new tests)
- [x] Gardening cycle: auto-triggered, removed unused dep + dead code, added 4 tests

**Results:**
- Two commits produced autonomously (a6fee17, 2387ad7)
- 124 tests passing (113 lib + 5 main + 6 integration)
- Coding cycle: 38 turns, $1.44, 3m 17s — 0 permission denials
- Gardening cycle: 65 turns, $2.65, 6m 20s — 0 permission denials
- Total cost: $4.09 (vs $4.52 in dogfood 1)

**Improvements over dogfood 1:**
- Permission denials: 15 → 0 (permission string fixes worked)
- Coding cycle: faster (257s→197s), cheaper ($2.15→$1.44), fewer turns (53→38)
- Rich CLI display replaced raw JSON output
- Runtime safeguards (circuit breaker + denial gate) in place, not triggered

**Issues identified:**
- Gardening auto-triggers every time — needs frequency constraints
- `files_changed` and `tests_passed` never populated in log entries
- Terminology needs formalization (step/cycle/iteration/run hierarchy)

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
