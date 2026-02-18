# Flow - Automated Coding Pipeline Runner

> **PREFERENCE**: Use retrieval-led reasoning over pre-training-led reasoning. Read the linked files for current project state, architecture, and tasks. Do not rely on general knowledge about build tools or pipelines—Flow has specific patterns defined in this repository.

---

## Quick Start for New Contributors

**Orient → Setup → Pick Task → Implement**

1. Read this file (3 min) → [TODO.md](./TODO.md) (2 min) → [plans/002-full-architecture.md](./plans/002-full-architecture.md) (5 min)
2. Run: `cargo build && cargo test-all` (verify setup — expect 355 passing tests)
3. Pick task from [TODO.md](./TODO.md)
4. Follow TDD: Write tests → Implement → Refactor

**Suggested First Tasks**: Selector config section | Periodic summary output | Status bar iteration progress

---

## Project Overview

**What**: Rust CLI that orchestrates Claude Code in autonomous cycles (coding, gardening, planning, review)
**Why**: Build a tool that builds itself—Flow will eventually implement its own features
**How**: Parse `cycles.toml` → Invoke `claude-code` with permissions → Log outcomes → Trigger dependent cycles

**Meta Goal**: Dogfood ASAP. Use Flow to build Flow once Phase 1 MVP works.

---

## Architecture (Compressed Index)

**Core Loop**: `config → select cycle → resolve perms → invoke claude-code → stream output → log → apply rules → repeat`

**Data Structures**:
```
cycles.toml: [global.permissions | [[cycle]]: name|prompt|permissions|after|context | [[cycle.step]]: name|session|prompt|permissions (Phase 2)]
.flow/log.jsonl: {iteration|cycle|timestamp|outcome|duration_secs|num_turns|total_cost_usd|permission_denial_count|steps? (Phase 2)}
.flow/progress.json: {started_at|current_iteration|max_iterations|current_cycle|current_status|cycles_executed}
```

**Components** → Files:
- Config parsing → `src/cycle/config.rs` | Parse cycles.toml TOML (CycleConfig, StepConfig, GlobalConfig)
- Permissions → `src/claude/permissions.rs` | Hierarchical additive merge (global+cycle+step)
- Executor → `src/cycle/executor.rs` | Single-step + multi-step cycle execution with display
- CLI builder → `src/claude/cli.rs` | Construct subprocess invocation with session resume
- Stream parser → `src/claude/stream.rs` | Parse stream-JSON, extract results/files/tests
- Session mgr → `src/claude/session.rs` | Session tag→ID mapping for step affinity
- Display → `src/cli/display.rs` | Rich terminal output, status bar, doctor report
- JSONL logger → `src/log/jsonl.rs` | Append-only .flow/log.jsonl with step outcomes
- Progress → `src/log/progress.rs` | Atomic .flow/progress.json writer for run monitoring
- Rules engine → `src/cycle/rules.rs` | Parse "after: [cycles]", trigger dependents with frequency
- Selector → `src/cycle/selector.rs` | AI-driven cycle selection from log context + priorities
- Router → `src/cycle/router.rs` | Step-level routing (sequential or LLM-driven)
- Context → `src/cycle/context.rs` | Iteration context injection (full/summaries/none)
- Doctor → `src/doctor.rs` | Diagnostic engine (D001–D006)
- Init → `src/init.rs` | `flow init` project scaffolding
- CLI interface → `src/main.rs` | Clap (subcommands: run, init, doctor), execution loop

**Phases**:
- P1 (MVP): ✅ Complete — manual single-step cycles, dogfooded twice
- P2 (Auto): ✅ Mostly complete — status bar, doctor, multi-iteration, init, AI selector, multi-step cycles, step routing, context modes, SIGINT handling, progress.json, files_changed/tests_passed tracking. Remaining: selector config, periodic summaries, doctor --repair, flow plan
- P3 (Advanced): Templates | Timeouts | Parallel cycles | Model profiles | State file | Pause/resume | Gap closure loops | Interactive init

**Full architecture**: [plans/002-full-architecture.md](./plans/002-full-architecture.md)
**Multi-step cycles plan**: [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md)

---

## Terminology

Flow uses a strict 4-level hierarchy. Use these terms consistently in code, config, docs, and logs.

| Level | Term | Definition | Example |
|-------|------|------------|---------|
| 1 | **Step** | A single Claude Code invocation (one prompt → one session) | "plan", "implement", "review" |
| 2 | **Cycle** | A named workflow of one or more steps | "coding", "gardening" |
| 3 | **Iteration** | One numbered pass in a run; selector picks a cycle, its steps execute | Iteration 3: coding cycle |
| 4 | **Run** | The entire execution from `flow` invocation to completion | `flow --max-iterations 20` |

**Session affinity**: Steps within the same cycle execution can share a Claude Code session via session tags. Same tag = continued session. Different tag = fresh session. Sessions do not persist across iterations.

**Current state**: The coding cycle is multi-step (plan → plan-review → implement). All other cycles are single-step. The plan-review step uses LLM routing (`router = "llm"`) to loop back to the plan step on rejection.

---

## Current Status

**Completed**: Phase 1 (MVP) | Phase 2 core: status bar, doctor (D001–D006), multi-iteration, AI selector, multi-step cycles (session affinity, per-step perms, step routing), context modes, SIGINT handling, progress.json, files_changed/tests_passed, cumulative health tracking, flow init, coding plan+review steps
**In Progress**: Phase 2 wrap-up (selector config, periodic summaries, doctor --repair)
**Next**: `[selector]` config section → enrich selector context → periodic summary output

**Test Status**:
- ✅ 355 passing (325 lib + 24 main + 6 integration)

**Component Status** (all ✅):
```
config.rs         | 47 tests | CycleConfig, StepConfig, GlobalConfig, StepRouter, validation
permissions.rs    | 7 tests  | 3-layer additive merge (global+cycle+step)
session.rs        | 9 tests  | Session tag→ID mapping, --resume args
executor.rs       | 28 tests | Single+multi-step execution, StepAggregator, shutdown
cli.rs            | 12 tests | Command builder with session resume
stream.rs         | 35 tests | Stream-JSON parser, files_changed, tests_passed, session_id
display.rs        | 28 tests | CycleDisplay, StatusLine, HealthColor, doctor report
jsonl.rs          | 18 tests | CRUD, StepOutcome, CycleOutcome.steps, permission_denials
progress.rs       | 11 tests | RunProgress, RunStatus, atomic writes
rules.rs          | 14 tests | Dependency triggers + frequency constraints
selector.rs       | 27 tests | AI-driven cycle selection
router.rs         | 22 tests | Sequential + LLM step routing, VisitTracker
context.rs        | tests    | Iteration context injection (full/summaries/none)
doctor.rs         | 18 tests | D001–D006 diagnostics
init.rs           | 13 tests | Project scaffolding template
main.rs           | 24 tests | CLI parsing, run loop, gates, helpers
integration_test  | 6 tests  | End-to-end flows
```

---

## Development Workflow

**For Claude Code agents**: Use `/coding-iteration` skill at start of each iteration

**TDD Process** (non-negotiable):
1. **Red**: Write failing test first → Run test (see fail)
2. **Green**: Minimum code to pass → Run test (see pass)
3. **Refactor**: Clean up while tests stay green

**Commands**:
```bash
# Daily development
cargo test --lib && cargo clippy --lib && cargo fmt

# Verification before commit
cargo test --lib     # Library tests pass
cargo clippy --lib   # Zero warnings
cargo fmt            # Format code

# Aliases (defined in .cargo/config.toml)
cargo check-all    # Check all targets
cargo test-all     # Test all targets (includes integration tests)
cargo clippy-all   # Clippy with -D warnings
cargo fmt-check    # Verify formatting
```

**Pre-commit Hook**:
- Automatically runs on `git commit`
- Verifies: library tests pass, no clippy warnings, code formatted
- Located at `.git/hooks/pre-commit`
- Only checks library code until integration tests implemented

**Code Standards**:
- Linting: clippy (all|pedantic|nursery|cargo)
- Safety: `unsafe_code = "forbid"`
- Formatting: rustfmt.toml (stable features)
- Docs: Warn on missing docs

---

## Cycle Types (Defined in cycles.toml)

**Coding** (multi-step): plan → plan-review (LLM routed) → implement (TDD) | Global perms + Edit(./TODO.md)|Edit(./AGENTS.md)|Bash(git *)
**Gardening**: Deps update|refactor|dead code|coverage | Perms: Edit(./Cargo.toml)|Bash(git *) | Triggers: after=[coding], min_interval=25
**Review**: Goal-backward verification — EXISTS|SUBSTANTIVE|WIRED checks | Perms: read-only | Manual trigger only
**Docs**: Update README and user-facing docs | Perms: Edit(./README.md)|Edit(./docs/**)|Bash(git *) | Triggers: after=[coding], min_interval=3
**Planning**: Analyze TODO|create plans|prioritize|scope tasks | Perms: Edit(./TODO.md)|Edit(./AGENTS.md)|Edit(./plans/**)|Bash(git *) | Manual trigger only

**Permission Model**: Hierarchical additive (global + per-cycle + per-step, only adds never removes). Uses native Claude Code `--allowedTools` syntax (e.g., `Read`, `Edit(./src/**)`, `Bash(cargo *)`)

---

## Critical Design Decisions

**Why cycles not pipelines?**: Circular iteration concept, avoids CI/CD confusion (see [PLANNING_QUESTIONS.md](./PLANNING_QUESTIONS.md))
**Why TDD?**: Meta-tool building AI tooling, prevents regression during dogfooding, documents behavior
**Why Phase 1 first?**: Learn failure modes before building recovery, dogfood sooner, iterate faster
**Why JSONL?**: Append-only, easy to parse, streamable, no corruption on crash
**Why hierarchical additive perms?**: Cycles can add protections, never accidentally remove them

---

## Anti-Patterns (Don't Do This)

❌ Skip tests (TDD is mandatory) | ❌ Implement without reading plan | ❌ Premature optimization (build for P1 not P3)
❌ Bypass linters (fix warnings, don't suppress) | ❌ Subtractive permissions (only additive)
❌ Hardcode paths (use relative paths) | ❌ Block dogfooding (ship P1 fast to learn)
❌ Assert on external crate error messages (fragile — assert `is_err()` or match only your own messages)
❌ Use agent memory files for project knowledge (all guidance must be explicit in version-controlled project files)
❌ Build integrations without verifying the external tool's actual interface (always check real CLI docs/help before implementing — we almost built permissions on an invented format)

---

## File Organization

```
flow/
├── AGENTS.md              ← You are here (index/map)
├── TODO.md                ← Pending tasks by phase (read next)
├── COMPLETED.md           ← Historical record of completed work
├── PLANNING_QUESTIONS.md  ← Decisions & rationale
├── README.md              ← Public-facing docs
├── Cargo.toml             ← Deps + lints config
├── cycles.toml            ← Cycle definitions (coding + gardening + review + docs + planning)
├── .flow/                 ← Runtime state (gitignored)
│   ├── log.jsonl          ← Cycle execution history
│   ├── progress.json      ← Real-time run progress (written during execution, deleted on completion)
│   ├── current-plan.md    ← Written by coding cycle's plan step
│   └── plan-review.md     ← Written by coding cycle's plan-review step
├── plans/
│   ├── 001-mvp-pipeline-runner.md  ← Original MVP plan (superseded)
│   ├── 002-full-architecture.md    ← Complete architecture (read for deep dive)
│   ├── 003-multi-step-cycles.md    ← Multi-step cycles with session affinity
│   └── TEMPLATE.md                  ← Template for new plans
├── src/
│   ├── main.rs            ← CLI entry (clap subcommands, execution loop, signal handling)
│   ├── lib.rs             ← Public API re-exports
│   ├── init.rs            ← `flow init` scaffolding
│   ├── doctor.rs          ← Diagnostic engine (D001–D006)
│   ├── testutil.rs        ← Shared test helpers (make_test_outcome)
│   ├── cycle/
│   │   ├── config.rs      ← Parse cycles.toml (CycleConfig, StepConfig, GlobalConfig)
│   │   ├── executor.rs    ← Single+multi-step execution, StepAggregator
│   │   ├── rules.rs       ← Dependency triggers + frequency constraints
│   │   ├── selector.rs    ← AI-driven cycle selection
│   │   ├── router.rs      ← Step routing (sequential + LLM)
│   │   └── context.rs     ← Iteration context injection
│   ├── claude/
│   │   ├── cli.rs         ← CLI command builder with session resume
│   │   ├── permissions.rs ← 3-layer permission resolver (global+cycle+step)
│   │   ├── stream.rs      ← Stream-JSON parser (events, files, tests, session_id)
│   │   └── session.rs     ← Session tag→ID mapping for step affinity
│   ├── cli/
│   │   └── display.rs     ← Rich display, status bar, health colors, doctor report
│   └── log/
│       ├── jsonl.rs       ← JSONL logger (CycleOutcome, StepOutcome)
│       └── progress.rs    ← Atomic progress.json writer (RunProgress, RunStatus)
└── tests/
    └── integration_test.rs ← End-to-end tests
```

---

## Example Workflow: Implementing "Cycle Config Parser"

**You have empty context, need to implement a task**:

TODO.md → plans/002 Section 1 → Write test (RED) → Implement (GREEN) → Refactor → Test+Lint → Commit

See plans/002-full-architecture.md for cycles.toml format examples and implementation guidance.

---

## References (Read These for Details)

- [TODO.md](./TODO.md) - **START HERE**: Task queue organized by phase
- [plans/002-full-architecture.md](./plans/002-full-architecture.md) - **Deep dive**: Complete architecture, data formats, examples
- [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md) - **Phase 2**: Multi-step cycles with session affinity
- [PLANNING_QUESTIONS.md](./PLANNING_QUESTIONS.md) - **Rationale**: Why we made specific design decisions
- [README.md](./README.md) - **Public docs**: User-facing documentation
- [Vercel AGENTS.md pattern](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals) - **Inspiration**: Source of this pattern

**Vercel Key Insight**: "An 8KB docs index embedded directly in AGENTS.md achieved 100% pass rate, while skills maxed out at 79%"

---

## Quick Wins for New Contributors

**Phase 1 complete. Phase 2 nearly complete.** Remaining Phase 2 tasks:

**Medium**: Add `[selector]` config section to cycles.toml — decouple selector from `--todo` flag
**Easy**: Periodic summary output every N iterations during multi-iteration runs
**Easy**: Show iteration progress in status bar (`[3/10] [coding] ▶ ...`)
**Medium**: `flow doctor --repair` auto-fix mode for safe config fixes

See [TODO.md](./TODO.md) for full task list.

---

**Last Updated**: 2026-02-18 | **Status**: Phase 1 complete, Phase 2 nearly complete (355 tests) | **Next Milestone**: Selector config section → close Phase 2
