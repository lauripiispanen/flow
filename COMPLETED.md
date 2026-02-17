# Flow - Completed Work

> Historical record of completed tasks and milestones. For current tasks, see [TODO.md](./TODO.md).

---

## Phase 1: MVP - Manual Single Cycle

**Goal**: Execute named cycles manually with basic observability. **Status: Complete.**

All Phase 1 work was completed between 2026-02-14 and 2026-02-16, culminating in two successful dogfood runs.

### Cycle Configuration
- [x] Define cycles.toml schema — `src/cycle/config.rs` (2026-02-14)
- [x] Implement TOML parser for cycle definitions (2026-02-14)
- [x] Implement permission resolver (global + per-cycle, additive) — `src/claude/permissions.rs` (2026-02-14)
- [x] Validate cycle configuration on load (2026-02-14)
- [x] Validate permission strings match `--allowedTools` syntax (2026-02-16)

### Cycle Executor
- [x] Implement CycleExecutor struct — `src/cycle/executor.rs` (2026-02-15)
- [x] Build Claude Code CLI command with --allowedTools flags — `src/claude/cli.rs` (2026-02-15)
- [x] Stream stdout/stderr to terminal (real-time) (2026-02-15)
- [x] Capture exit code and detect failures (2026-02-15)
- [x] Extract outcome summary from Claude's response (2026-02-15)

### JSONL Logger
- [x] Implement .flow/log.jsonl writer — `src/log/jsonl.rs` (2026-02-14)
- [x] Append-only log entries (2026-02-14)
- [x] Serialize cycle outcomes to JSON (2026-02-14)

### Cycle Rules Engine
- [x] Parse `after: [...]` from cycle config — `src/cycle/rules.rs` (2026-02-15)
- [x] Implement rule evaluator (2026-02-15)
- [x] Trigger dependent cycles automatically — `src/main.rs` (2026-02-15)

### CLI Interface
- [x] Implement `--cycle <name>` argument (2026-02-15)
- [x] Fail-fast error handling with anyhow (2026-02-15)
- [x] Define coding and gardening cycle prompts in cycles.toml (2026-02-15)

### Post-Dogfood: Rich Display & Runtime Safeguards
- [x] Fix cycles.toml permission strings (Write → Edit) (2026-02-16)
- [x] Implement stream-JSON parser — `src/claude/stream.rs` (2026-02-16)
- [x] Implement rich CLI display — `src/cli/display.rs` (2026-02-16)
- [x] Extend CycleOutcome with result blob data (2026-02-16)
- [x] Extend CycleResult with parsed stream-json fields (2026-02-16)
- [x] Implement execute_with_display() in CycleExecutor (2026-02-16)
- [x] Wire new execution path in main.rs (2026-02-16)
- [x] Add safeguard thresholds to GlobalConfig (2026-02-16)
- [x] Update integration tests for new CycleResult fields (2026-02-16)

### Testing & Validation
- [x] Integration test: Run coding cycle end-to-end (2026-02-15)
- [x] Test: Gardening auto-triggers after coding (2026-02-15)
- [x] Test: Permission resolution (additive) (2026-02-15)
- [x] Test: JSONL logging works correctly (2026-02-14)

---

## Phase 2: Automation (completed items)

### Status Bar
- [x] Implement persistent status bar with ANSI escape codes — `src/cli/display.rs` (2026-02-16)
- [x] Color-coded health (green/yellow/red based on error count) (2026-02-16)

### Cycle Frequency Constraints
- [x] Add `min_interval` config field — `src/cycle/config.rs` (2026-02-16)
- [x] Rules engine checks last-run time before triggering — `src/cycle/rules.rs` (2026-02-16)

### Flow Doctor
- [x] Implement diagnostic engine (D001–D006) — `src/doctor.rs` (2026-02-16)
- [x] Store full `permission_denials` list in CycleOutcome (2026-02-16)
- [x] Wire `flow doctor` subcommand into CLI (2026-02-17)

### Multi-Iteration Loop
- [x] Implement `--max-iterations` support (2026-02-17)
- [x] Iteration counter and loop control (2026-02-17)
- [x] Stop conditions (max iterations, errors, denial gate) (2026-02-17)

### Cycle Selector
- [x] Implement AI-driven cycle selector — `src/cycle/selector.rs` (2026-02-17)
- [x] JSONL log summarizer for selector context (2026-02-17)
- [x] TODO.md parser for task priorities (2026-02-17)
- [x] Selection logic (balance + context + priority) (2026-02-17)

### Review & Planning Cycles
- [x] Define review cycle prompt in cycles.toml (2026-02-16)
- [x] Define planning cycle prompt in cycles.toml (2026-02-16)
- [x] Design multi-step cycle config format — `plans/003-multi-step-cycles.md` (2026-02-16)

---

## Dogfood Runs

### Dogfood 1 (2026-02-15)
- Coding cycle: 53 turns, $2.15, 4m 13s — wrote 6 integration tests
- Gardening cycle: 70 turns, $2.37, 4m 15s — removed dead pipeline module
- 15 wasted turns on permission denials (~12% overhead)
- Learnings: Write→Edit permission fix, raw JSON unreadable, need runtime safeguards

### Dogfood 2 (2026-02-16)
- Coding cycle: 38 turns, $1.44, 3m 17s — validated permission strings (10 new tests)
- Gardening cycle: 65 turns, $2.65, 6m 20s — removed unused dep + dead code
- 0 permission denials (vs 15 in dogfood 1)
- Total cost: $4.09 (vs $4.52 in dogfood 1)
