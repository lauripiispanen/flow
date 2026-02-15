# Plan 002: Full Flow Architecture

**Status**: Planning Complete
**Priority**: P0
**Created**: 2026-02-14
**Based On**: PLANNING_QUESTIONS.md responses

## Vision

Flow is an automated coding pipeline runner that orchestrates Claude Code CLI in iterative cycles. Each cycle executes a complete workflow (coding, gardening, planning, review) with controlled permissions, and the system intelligently selects the next cycle based on context, balance, and priorities.

## Design Principles

1. **Start Simple, Iterate Fast**: MVP focuses on manual single-cycle execution, then automate
2. **Dogfood ASAP**: Use Flow to build Flow as soon as basic functionality works
3. **Fail-Fast Learning**: Stop on errors in MVP to understand failure modes before building recovery
4. **Additive Permissions**: Hierarchical allowlists that can only add, never remove permissions
5. **Configurable Everything**: Build extensibility from the start, even if not all features are used in MVP

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Flow CLI                              │
│  cargo run -- --cycle coding --max-iterations 20            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │   Cycle Selector        │
        │  (Phase 2: Automated)   │
        │  - Balance cycles       │
        │  - Context-aware        │
        │  - Priority-aware       │
        └────────┬───────────────┘
                 │
                 ▼
        ┌────────────────────────┐
        │   Cycle Executor        │
        │  - Load cycle config    │
        │  - Apply permissions    │
        │  - Invoke Claude Code   │
        │  - Capture outcome      │
        └────────┬───────────────┘
                 │
                 ▼
        ┌────────────────────────┐
        │  Observation Layer      │
        │  - Real-time output     │
        │  - Progress file        │
        │  - Periodic summaries   │
        │  - JSONL log            │
        └────────────────────────┘
```

### Data Flow

```
User: flow --cycle coding
  ↓
Load cycles.toml config
  ↓
Resolve permissions (global + cycle-specific, additive)
  ↓
Execute cycle:
  - Construct Claude Code CLI command with --allowedTools flags
  - Stream output to terminal (real-time observability)
  - Update .flow/progress.json
  ↓
Capture outcome (ask Claude for summary)
  ↓
Append to .flow/log.jsonl
  ↓
Apply cycle rules (e.g., "after: [coding]" → trigger gardening)
  ↓
Update TODO.md with results
  ↓
Done (or continue to next iteration)
```

---

## Phase Breakdown

### Phase 1: MVP - Manual Single Cycle (DOGFOOD TARGET)

**Goal**: Execute a single named cycle manually with basic observability

**Features**:
- ✅ Manual cycle selection via CLI (`--cycle coding`)
- ✅ Load cycle config from `cycles.toml`
- ✅ Simple string prompts (no templates yet)
- ✅ Hierarchical additive permissions (global + per-cycle)
- ✅ Invoke Claude Code CLI with `-p` flags
- ✅ Real-time terminal output streaming
- ✅ Capture outcome via Claude summary
- ✅ JSONL logging to `.flow/log.jsonl`
- ✅ Fail-fast error handling (no retry/recovery)
- ✅ Basic cycle rules (e.g., `after: ["coding"]`)

**Success Criteria**:
- Can run: `flow --cycle coding`
- Coding cycle completes successfully
- Gardening auto-runs after coding (rule-based)
- JSONL log captures outcomes
- Can dogfood: Use Flow to build the next feature

**Non-Goals (Deferred to Phase 2+)**:
- ❌ Automated cycle selection
- ❌ Multi-iteration loops
- ❌ Template prompts
- ❌ Cost tracking
- ❌ Timeouts
- ❌ Advanced observability (progress file, summaries)

---

### Phase 2: Automation - Multi-Iteration with Smart Selection

**Goal**: Autonomous multi-iteration runs with intelligent cycle selection

**Features**:
- ✅ Cycle selector using Claude Sonnet/Haiku
- ✅ Selection logic optimizes for:
  - Balance (ensure all cycles run eventually)
  - Context-awareness (tests failing → prioritize gardening)
  - User priorities (P0 tasks first)
- ✅ Multi-iteration support (`--max-iterations 20`)
- ✅ Iteration context (configurable per cycle):
  - Some cycles get full JSONL history
  - Some get summaries only
  - Some can invoke CLI tool to access context
- ✅ Full observability:
  - Real-time output (Phase 1)
  - Progress file (`.flow/progress.json`)
  - Periodic summaries (every N iterations)
  - JSONL log (Phase 1)
- ✅ Enhanced cycle rules (complex logic)

**Success Criteria**:
- Run: `flow --max-iterations 20`
- System autonomously selects and executes cycles
- Balances cycle types over iterations
- Context-aware decisions (e.g., gardening after test failures)
- Real-time progress visible

---

### Phase 3: Advanced Features

**Features**:
- ✅ Template prompts with variables (`{{current_task}}`, `{{previous_summary}}`)
- ✅ Multi-part prompts (system + user)
- ✅ Configurable timeouts per cycle type
- ✅ Cost tracking (per cycle, per iteration, global)
- ✅ Parallel cycle execution (for independent cycles)
- ✅ Enhanced outcome capture plugins:
  - Git commit analysis
  - Test result parsing
  - Lint/clippy delta
- ✅ Recovery strategies (retry, skip, recovery cycles)

---

## Core Components

### 1. Cycle Configuration (`cycles.toml`)

> **Source of truth for permission syntax**: [Claude Code Permissions Docs](https://code.claude.com/docs/en/permissions)
> Permissions use the native `--allowedTools` format: `ToolName` or `ToolName(specifier)` with glob patterns.

```toml
# Global defaults
[global]
permissions = ["Read", "Edit(./src/**)", "Bash(cargo *)"]

# Coding Cycle
[[cycle]]
name = "coding"
description = "Pick a task from TODO.md, plan, implement, test"
prompt = """
You are Flow's coding cycle. Your job:
1. Read TODO.md and pick the highest priority pending task
2. Create a plan in plans/*.md
3. Implement with TDD (red-green-refactor)
4. Run tests and ensure they pass
5. Run cargo fmt and cargo clippy
6. Summarize what you accomplished
"""
permissions = ["Edit(./tests/**)", "Bash(cargo test *)"]
after = [] # Runs after these cycles complete
context = "summaries" # full | summaries | none

# Gardening Cycle
[[cycle]]
name = "gardening"
description = "Dependency updates, refactoring, docs, dead code removal"
prompt = """
You are Flow's gardening cycle. Your job:
1. Update dependencies (cargo update, check for outdated crates)
2. Suggest and apply refactorings
3. Improve documentation
4. Remove dead code
5. Improve test coverage
6. Summarize improvements
"""
permissions = ["Edit(./Cargo.toml)", "Bash(cargo update *)"]
after = ["coding"] # Auto-run after coding cycle
context = "none"

# Review Cycle
[[cycle]]
name = "review"
description = "Code review, security audit, documentation check"
prompt = """
You are Flow's review cycle. Your job:
1. Review recent changes for code quality
2. Check for security vulnerabilities
3. Verify documentation is up to date
4. Suggest improvements
5. Summarize findings
"""
permissions = [] # Read-only (uses global only)
after = []
context = "full"

# Planning Cycle
[[cycle]]
name = "planning"
description = "Analyze TODO, create plans, prioritize work"
prompt = """
You are Flow's planning cycle. Your job:
1. Analyze TODO.md and current progress
2. Create or update plans in plans/*.md
3. Prioritize tasks based on dependencies and value
4. Update TODO.md with new priorities
5. Summarize the plan
"""
permissions = ["Edit(./TODO.md)", "Edit(./plans/**)"]
after = []
context = "summaries"
```

### 2. JSONL Log Format (`.flow/log.jsonl`)

Each line is a JSON object:

```jsonl
{"iteration":1,"cycle":"coding","timestamp":"2026-02-14T10:30:00Z","outcome":"Implemented Pipeline::run() with basic structure","files_changed":["src/pipeline.rs"],"tests_passed":3,"duration_secs":180}
{"iteration":2,"cycle":"gardening","timestamp":"2026-02-14T10:33:00Z","outcome":"Updated 3 dependencies, removed 2 dead functions","files_changed":["Cargo.toml","src/claude.rs"],"tests_passed":3,"duration_secs":45}
{"iteration":3,"cycle":"coding","timestamp":"2026-02-14T10:35:00Z","outcome":"Added ClaudeClient implementation","files_changed":["src/claude.rs","tests/claude_test.rs"],"tests_passed":5,"duration_secs":220}
```

### 3. Progress File (`.flow/progress.json`)

Real-time status (overwritten each iteration):

```json
{
  "started_at": "2026-02-14T10:30:00Z",
  "current_iteration": 3,
  "max_iterations": 20,
  "current_cycle": "coding",
  "current_status": "running",
  "cycles_executed": {
    "coding": 2,
    "gardening": 1,
    "review": 0,
    "planning": 0
  },
  "total_duration_secs": 445,
  "last_outcome": "Added ClaudeClient implementation"
}
```

### 4. Cycle Selector Logic (Phase 2)

The cycle selector is invoked with:
- Current JSONL log (or summary)
- TODO.md state
- Recent outcomes
- Cycle balance (which cycles haven't run recently)

It returns the next cycle to execute.

**Selection Prompt (to Claude Sonnet/Haiku)**:

```
You are Flow's cycle selector. Analyze the current state and choose the next cycle.

Context:
- Last 5 iterations: [summary from JSONL]
- Current TODO.md state: [P0: 3 tasks, P1: 5 tasks]
- Cycle balance: coding=2, gardening=1, review=0, planning=0
- Recent test status: 5/5 passing

Available cycles: coding, gardening, review, planning

Choose the next cycle and explain why.
Output JSON: {"cycle": "review", "reason": "Haven't reviewed recently, good time to audit"}
```

---

## Implementation Phases

### Phase 1 Tasks (MVP - Dogfood Target)

1. **Cycle Configuration**
   - [ ] Define `cycles.toml` schema
   - [ ] Implement TOML parser for cycle definitions
   - [ ] Implement permission resolver (global + per-cycle, additive)
   - [ ] Validate cycle configuration on load

2. **Cycle Executor**
   - [ ] Implement `CycleExecutor` struct
   - [ ] Build Claude Code CLI command with `--allowedTools` flags
   - [ ] Stream stdout/stderr to terminal (real-time)
   - [ ] Capture exit code and detect failures
   - [ ] Extract outcome summary from Claude's response

3. **JSONL Logger**
   - [ ] Implement `.flow/log.jsonl` writer
   - [ ] Append-only log entries
   - [ ] Serialize cycle outcomes to JSON

4. **Cycle Rules Engine**
   - [ ] Parse `after: [...]` from cycle config
   - [ ] Implement rule evaluator
   - [ ] Trigger dependent cycles automatically

5. **CLI Interface**
   - [ ] Implement `--cycle <name>` argument
   - [ ] Fail-fast error handling
   - [ ] Pretty output formatting

6. **Initial Cycles**
   - [ ] Define coding cycle prompt
   - [ ] Define gardening cycle prompt
   - [ ] Define review cycle prompt
   - [ ] Define planning cycle prompt

7. **Testing & Validation**
   - [ ] Integration test: Run coding cycle end-to-end
   - [ ] Test: Gardening auto-triggers after coding
   - [ ] Test: Permission resolution (additive)
   - [ ] Test: JSONL logging works correctly

8. **Dogfooding**
   - [ ] Use Flow to implement next feature
   - [ ] Document learnings and iterate

---

### Phase 2 Tasks (Automation)

1. **Cycle Selector**
   - [ ] Implement selector using Claude Sonnet API
   - [ ] Selection logic (balance + context + priority)
   - [ ] JSONL log summarizer for selector context
   - [ ] TODO.md parser for task priorities

2. **Multi-Iteration Loop**
   - [ ] Implement `--max-iterations` support
   - [ ] Iteration counter and loop control
   - [ ] Stop conditions (max iterations, errors)

3. **Iteration Context**
   - [ ] Implement context modes (full, summaries, none)
   - [ ] Optional CLI tool cycles can invoke for context
   - [ ] Context injection into cycle prompts

4. **Enhanced Observability**
   - [ ] Implement `.flow/progress.json` writer
   - [ ] Periodic summary output (every N iterations)
   - [ ] Cycle balance statistics

5. **Testing**
   - [ ] Integration test: 5-iteration automated run
   - [ ] Test: Selector chooses balanced cycles
   - [ ] Test: Context-aware selection (e.g., after test failure)

---

### Phase 3 Tasks (Advanced Features)

1. **Template Prompts**
   - [ ] Variable substitution (`{{var}}`)
   - [ ] Multi-part prompts (system + user)

2. **Timeouts**
   - [ ] Per-cycle timeout configuration
   - [ ] Timeout enforcement

3. **Cost Tracking**
   - [ ] Parse Claude API usage from responses
   - [ ] Track per cycle, per iteration, global

4. **Parallel Cycles**
   - [ ] Dependency graph analysis
   - [ ] Parallel executor for independent cycles

5. **Enhanced Outcome Capture**
   - [ ] Git commit analysis plugin
   - [ ] Test result parser plugin
   - [ ] Lint/clippy delta plugin

---

## File Structure

```
flow/
├── .flow/                      # Runtime state (gitignored)
│   ├── log.jsonl               # Cycle execution log
│   └── progress.json           # Real-time progress
├── cycles.toml                 # Cycle definitions
├── src/
│   ├── main.rs                 # CLI entry
│   ├── lib.rs                  # Public API
│   ├── cycle/
│   │   ├── mod.rs              # Cycle module
│   │   ├── config.rs           # cycles.toml parser
│   │   ├── executor.rs         # Cycle execution
│   │   ├── selector.rs         # Cycle selection (Phase 2)
│   │   └── rules.rs            # Cycle rules engine
│   ├── claude/
│   │   ├── mod.rs              # Claude integration
│   │   ├── cli.rs              # CLI command builder
│   │   └── permissions.rs      # Permission resolver
│   ├── log/
│   │   ├── mod.rs              # Logging module
│   │   ├── jsonl.rs            # JSONL writer
│   │   └── progress.rs         # Progress file writer
│   └── observe/
│       ├── mod.rs              # Observability
│       └── stream.rs           # Output streaming
├── tests/
│   ├── cycle_executor_test.rs
│   ├── cycle_selector_test.rs
│   └── integration_test.rs
└── plans/
    ├── 001-mvp-pipeline-runner.md
    ├── 002-full-architecture.md (this file)
    └── TEMPLATE.md
```

---

## Success Metrics

### Phase 1 (MVP)
- [ ] Can execute: `flow --cycle coding`
- [ ] Coding cycle completes task from TODO.md
- [ ] Gardening auto-runs after coding
- [ ] JSONL log has accurate entries
- [ ] Used Flow to build at least one feature

### Phase 2 (Automation)
- [ ] Can execute: `flow --max-iterations 20`
- [ ] System runs autonomously without intervention
- [ ] Cycles are balanced over iterations
- [ ] Context-aware decisions observable in logs
- [ ] Progress visible in real-time

### Phase 3 (Advanced)
- [ ] Template prompts work correctly
- [ ] Timeouts prevent runaway cycles
- [ ] Cost tracking shows per-cycle costs
- [ ] Parallel cycles reduce total runtime

---

## Open Questions & Future Considerations

1. **Cycle Memory**: How should cycles remember their own learnings across iterations?
   - Option: Per-cycle `.flow/memory-{cycle}.md` file they can read/write
   - Cycles could refine their own strategies over time

2. **Cross-Project Cycles**: Could cycles.toml be shared across projects?
   - Flow could have default cycles + project overrides

3. **Cycle Marketplace**: Could we share successful cycle definitions?
   - Community-contributed cycles for different project types

4. **Visual Dashboard**: Web UI for monitoring long runs?
   - Real-time visualization of progress, cycle balance, outcomes

5. **Rollback**: Manual rollback for now, but future automated snapshots?
   - Git-based snapshots before each iteration
   - `flow rollback --to-iteration 5`

---

## Next Steps

1. ✅ Planning complete
2. ⏳ Break down Phase 1 tasks into TODO.md
3. ⏳ Update AGENTS.md with architecture
4. ⏳ Start TDD implementation of Phase 1
5. ⏳ Dogfood as soon as Phase 1 MVP works
