# Plan 003: Multi-Step Cycles with Session Affinity

**Status**: Planning
**Priority**: P0 (Phase 2)
**Created**: 2026-02-16
**Target**: Phase 2 — after frequency constraints

## Goal

Enable cycles to be composed of multiple **steps**, where each step is a separate Claude Code invocation. Steps within a cycle can share sessions (via session tags), enabling patterns like:

- Architect plans → Coder implements → Architect reviews (with full planning context)
- Planner outlines → Implementer codes → Planner verifies alignment
- Reviewer audits → Fixer addresses issues (with reviewer's context)

This separates concerns (different prompts, permissions, roles per step) while preserving context where it matters.

## Terminology

This plan uses the formalized hierarchy (see AGENTS.md § Terminology):

| Level | Term | Definition |
|-------|------|------------|
| 1 | **Step** | A single Claude Code invocation (one prompt → one session interaction) |
| 2 | **Cycle** | A named workflow of one or more steps |
| 3 | **Iteration** | One numbered pass in a run; the selector picks a cycle, its steps execute |
| 4 | **Run** | The entire execution from `flow` invocation to completion |

## Non-Goals

- Parallel step execution within a cycle (steps are always sequential)
- Dynamic step selection (all steps in a cycle always execute in order)
- Cross-cycle session sharing (sessions are scoped to one cycle execution)
- Session persistence across iterations (a new cycle execution gets fresh sessions)

## Motivation

Discovered during dogfood 2: a coding cycle could benefit from separation of planning and implementation. An architect with read-only permissions plans the approach, then a coder with write permissions implements it, then the architect reviews with full context of what it planned. Today this requires three separate cycles, losing the architect's planning context on review.

## Architecture

### Config Format

Single-step cycles (current, backward compatible):

```toml
[[cycle]]
name = "gardening"
prompt = "You are Flow's gardening cycle..."
permissions = ["Edit(./Cargo.toml)"]
after = ["coding"]
```

Multi-step cycles (new):

```toml
[[cycle]]
name = "coding"
description = "Plan, implement, and review with separated concerns"
after = []
context = "summaries"

[[cycle.step]]
name = "plan"
session = "architect"
prompt = """
You are the architect. Read TODO.md, pick the highest priority task,
and write a detailed implementation plan to .flow/current-plan.md.
Do not write any code.
"""
permissions = ["Read", "Edit(./.flow/current-plan.md)"]

[[cycle.step]]
name = "implement"
session = "coder"
prompt = """
Read .flow/current-plan.md and implement the plan with TDD.
Follow the plan exactly. Do not deviate from the architecture.
"""
permissions = ["Read", "Edit(./src/**)", "Edit(./tests/**)", "Bash(cargo *)"]

[[cycle.step]]
name = "review"
session = "architect"    # continues the architect's session
prompt = """
Review the implementation against your original plan.
Check: Does it match the architecture? Are tests comprehensive?
Are there issues to flag? Write findings to .flow/review-notes.md.
"""
permissions = ["Read", "Edit(./.flow/review-notes.md)"]
```

### Key Design Decisions

**Session tags, not session IDs**: Config uses a human-readable tag (e.g., `"architect"`). The executor maps tags to actual Claude Code session IDs at runtime. Same tag within one cycle execution = same session. Different cycle executions always get fresh sessions.

**`--continue` vs `--resume`**: When a step continues a previous session, the executor uses Claude Code's session continuation mechanism. The exact flag (`--continue`, `--resume`, or session file) depends on what Claude Code supports — the executor abstracts this.

**Permission inheritance**: Step permissions are additive on top of cycle + global permissions. A step can add permissions but not remove them (consistent with existing model). If a cycle has no steps, cycle-level `prompt` and `permissions` behave as a single implicit step (backward compatible).

**Intermediate artifacts**: Steps communicate through the filesystem (e.g., `.flow/current-plan.md`). This is intentional — it's inspectable, debuggable, and doesn't require Flow to shuttle data between steps.

### Data Flow

```
Cycle "coding" selected for iteration 3
│
├── Step 1: "plan"
│   ├── session tag: "architect" → new session (ID: abc123)
│   ├── prompt: "Read TODO.md, pick task, write plan..."
│   ├── permissions: global + Read + Edit(./.flow/current-plan.md)
│   ├── output: writes .flow/current-plan.md
│   └── result: logged as step within iteration
│
├── Step 2: "implement"
│   ├── session tag: "coder" → new session (ID: def456)
│   ├── prompt: "Read plan, implement with TDD..."
│   ├── permissions: global + Edit(./src/**) + Bash(cargo *)
│   ├── output: source code changes, test results
│   └── result: logged as step within iteration
│
├── Step 3: "review"
│   ├── session tag: "architect" → continue session abc123
│   ├── prompt: "Review implementation against your plan..."
│   ├── permissions: global + Read + Edit(./.flow/review-notes.md)
│   ├── output: writes .flow/review-notes.md
│   └── result: logged as step within iteration
│
└── Cycle complete → log iteration outcome (aggregated from steps)
```

### Components

1. **Step Config Parser** (`src/cycle/config.rs`)
   - Parse `[[cycle.step]]` arrays from TOML
   - Validate step names unique within cycle
   - Validate session tags are non-empty strings
   - Fall back to single implicit step when cycle has `prompt` but no `[[cycle.step]]`

2. **Step Executor** (`src/cycle/executor.rs`)
   - Execute steps sequentially within a cycle
   - Track session tag → session ID mapping for the cycle execution
   - First step with a given tag: create new session
   - Subsequent steps with same tag: continue existing session
   - Aggregate step results into overall CycleResult

3. **Session Manager** (new: `src/claude/session.rs`)
   - Map session tags to Claude Code session IDs
   - Handle session creation and continuation
   - Cleanup / release sessions after cycle completes

4. **JSONL Logger** (`src/log/jsonl.rs`)
   - Extend log format to optionally include per-step data
   - Cycle-level entry remains the primary record (backward compatible)
   - Optional `steps` array in log entry for detailed step-level data

### Log Format Extension

```jsonl
{
  "iteration": 3,
  "cycle": "coding",
  "timestamp": "2026-02-16T10:30:00Z",
  "outcome": "Planned and implemented feature X, review passed",
  "duration_secs": 540,
  "num_turns": 95,
  "total_cost_usd": 4.20,
  "steps": [
    {"name": "plan", "session": "architect", "duration_secs": 120, "num_turns": 25, "cost_usd": 0.80},
    {"name": "implement", "session": "coder", "duration_secs": 300, "num_turns": 55, "cost_usd": 2.60},
    {"name": "review", "session": "architect", "duration_secs": 120, "num_turns": 15, "cost_usd": 0.80}
  ]
}
```

The `steps` field is optional — single-step cycles omit it for backward compatibility.

## Backward Compatibility

- Cycles with a top-level `prompt` field and no `[[cycle.step]]` entries work exactly as today (single implicit step)
- Log entries without `steps` field are valid (existing logs remain readable)
- No changes to CLI interface — multi-step is purely a config/executor concern
- Session affinity is opt-in: steps without a `session` tag each get a fresh session

## Implementation Plan

### Phase 1: Config (TDD)
1. Parse `[[cycle.step]]` in config.rs
2. Validate step definitions (names, session tags)
3. Support both single-step (legacy) and multi-step cycle configs
4. Reject invalid configs (steps + top-level prompt, empty step list, etc.)

### Phase 2: Session Manager (TDD)
1. Create `src/claude/session.rs`
2. Track session tag → session ID mapping
3. Build CLI args for new vs continued sessions
4. Unit test session lifecycle

### Phase 3: Step Executor (TDD)
1. Extend `CycleExecutor` to iterate steps within a cycle
2. Per-step permission resolution (global + cycle + step, additive)
3. Per-step display and logging
4. Aggregate step results into CycleResult

### Phase 4: Integration
1. Wire into main execution path
2. Extend JSONL log with optional step data
3. Integration tests with multi-step cycle configs
4. Update cycles.toml with a multi-step cycle (e.g., coding)

## Open Questions

1. **What happens if a step fails mid-cycle?**
   - Option A: Stop the cycle immediately (fail-fast, consistent with Phase 1 philosophy)
   - Option B: Allow configurable behavior per step (e.g., review step is optional)
   - **Recommendation**: Option A for now. We can add skip/optional steps later.

2. **Should the review step be able to fail the cycle?**
   - If the architect finds issues, should the cycle be marked as failed?
   - Probably yes — the exit code of the last step determines cycle success.
   - But: a review finding issues is arguably a *successful* review. Needs thought.

3. **Session continuation mechanism in Claude Code**
   - Need to verify: does Claude Code support `--continue <session-id>` or `--resume`?
   - The executor will abstract this, but we need to know the actual CLI interface.
   - **Action**: Check Claude Code docs before implementing session manager.

4. **Step-to-step data passing beyond filesystem**
   - Current design: steps communicate via filesystem artifacts (.flow/current-plan.md)
   - Alternative: Flow could inject previous step's result_text into next step's prompt
   - **Recommendation**: Start with filesystem. It's inspectable and simple. Add prompt injection later if needed.

## Dependencies

### Internal
- Cycle config parser (done)
- Cycle executor (done)
- JSONL logger (done)
- Claude CLI builder (done — needs session continuation support)

### External
- Claude Code session continuation CLI interface (need to verify exact flags)

## Success Metrics

- [ ] Multi-step cycle config parses and validates correctly
- [ ] Steps execute sequentially with correct session affinity
- [ ] Architect's review step has full context from planning step
- [ ] Single-step cycles work exactly as before (no regression)
- [ ] Log entries capture per-step data
- [ ] At least one multi-step cycle defined in cycles.toml and tested via dogfood
