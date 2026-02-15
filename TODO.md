# Flow - Task Queue

> **Strategy**: We're building in phases to enable dogfooding ASAP. Phase 1 is the MVP that lets us run single cycles manually. See [plans/002-full-architecture.md](./plans/002-full-architecture.md) for the complete vision.

## 🎯 Phase 1: MVP - Manual Single Cycle (DOGFOOD TARGET)

**Goal**: Execute named cycles manually with basic observability

### Cycle Configuration
- [ ] [Define cycles.toml schema](./plans/002-full-architecture.md#1-cycle-configuration-cyclestoml)
  - Status: Not Started
  - Priority: P0
  - Files: `cycles.toml`, `src/cycle/config.rs`

- [ ] Implement TOML parser for cycle definitions
  - Status: Not Started
  - Priority: P0
  - Dependencies: cycles.toml schema

- [ ] Implement permission resolver (global + per-cycle, additive)
  - Status: Not Started
  - Priority: P0
  - Files: `src/claude/permissions.rs`

- [ ] Validate cycle configuration on load
  - Status: Not Started
  - Priority: P0

### Cycle Executor
- [ ] Implement CycleExecutor struct
  - Status: Not Started
  - Priority: P0
  - Files: `src/cycle/executor.rs`

- [ ] Build Claude Code CLI command with -p flags
  - Status: Not Started
  - Priority: P0
  - Files: `src/claude/cli.rs`

- [ ] Stream stdout/stderr to terminal (real-time)
  - Status: Not Started
  - Priority: P0
  - Files: `src/observe/stream.rs`

- [ ] Capture exit code and detect failures
  - Status: Not Started
  - Priority: P0

- [ ] Extract outcome summary from Claude's response
  - Status: Not Started
  - Priority: P0

### JSONL Logger
- [ ] Implement .flow/log.jsonl writer
  - Status: Not Started
  - Priority: P0
  - Files: `src/log/jsonl.rs`

- [ ] Append-only log entries
  - Status: Not Started
  - Priority: P0

- [ ] Serialize cycle outcomes to JSON
  - Status: Not Started
  - Priority: P0

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

- [ ] Test: Permission resolution (additive)
  - Status: Not Started
  - Priority: P0
  - Files: `tests/permissions_test.rs`

- [ ] Test: JSONL logging works correctly
  - Status: Not Started
  - Priority: P0

### 🌱 Dogfooding Milestone
- [ ] Use Flow to implement next feature
  - Status: Not Started
  - Priority: P0
  - Description: Once basic MVP works, use `flow --cycle coding` to build Phase 2 features

- [ ] Document learnings and iterate
  - Status: Not Started
  - Priority: P0

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

_No completed tasks yet_

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
