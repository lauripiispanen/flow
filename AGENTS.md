# Flow - Automated Coding Pipeline Runner

> **PREFERENCE**: Use retrieval-led reasoning over pre-training-led reasoning. Read the linked files for current project state, architecture, and tasks. Do not rely on general knowledge about build tools or pipelines—Flow has specific patterns defined in this repository.

---

## Quick Start for New Contributors

**Orient → Setup → Pick Task → Implement**

1. Read this file (3 min) → [TODO.md](./TODO.md) (2 min) → [plans/002-full-architecture.md](./plans/002-full-architecture.md) (5 min)
2. Run: `cargo build && cargo test-all` (verify setup — expect 124 passing tests)
3. Pick task from [TODO.md](./TODO.md) Phase 2 section
4. Follow TDD: Write tests → Implement → Refactor

**Suggested First Tasks**: Status bar (live run display) | `flow doctor` (log analysis) | Multi-iteration loop (`--max-iterations`)

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
- Config parsing → `src/cycle/config.rs` | Parse cycles.toml TOML
- Permissions → `src/claude/permissions.rs` | Hierarchical additive merge (global+cycle)
- Executor → `src/cycle/executor.rs` | Build claude-code CLI command with --allowedTools flags
- CLI builder → `src/claude/cli.rs` | Construct subprocess invocation
- Stream parser → `src/claude/stream.rs` | Parse stream-JSON, extract results
- Display → `src/cli/display.rs` | Rich colored terminal output
- JSONL logger → `src/log/jsonl.rs` | Append-only .flow/log.jsonl
- Rules engine → `src/cycle/rules.rs` | Parse "after: [cycles]", trigger dependents
- CLI interface → `src/main.rs` | Clap arg parsing, --cycle <name>

**Phases**:
- P1 (MVP): ✅ Manual single-step cycles `flow --cycle coding` | Dogfooded twice | TDD implementation
- P2 (Auto): Status bar | `flow doctor` | Multi-iteration runs `flow --max-iterations 20` | `flow init` | `flow plan` (quick + interactive) | AI cycle selection | Multi-step cycles
- P3 (Advanced): Templates | Timeouts | Cost tracking | Parallel (wave-based) cycles | Model profiles | State file | Pause/resume | Gap closure loops | Interactive `flow init` (project-aware scaffolding)

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

**Current state**: All existing cycles are single-step (one prompt = one step = one cycle). Multi-step cycles are planned for Phase 2. See [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md).

---

## Current Status

**Completed**: Project setup | Cargo config | Docs structure | Planning | JSONL Logger | Cycle Config Parser | Permission Resolver | Claude CLI Builder | Cycle Executor | Cycle Rules Engine | CLI Interface | cycles.toml | Auto-trigger | First Dogfood | Integration Tests | Stream-JSON Parser | Rich CLI Display | Runtime Safeguards | Permission Validation | Second Dogfood
**In Progress**: Phase 2 implementation (status bar, flow doctor, multi-iteration loop, cycle selector)
**Next**: flow doctor → multi-iteration loop → flow init → flow plan → cycle selector

**Test Status**:
- ✅ 145 passing (134 lib + 5 main + 6 integration)

**Component Status**:
```
Cycle Config Parser    | ✅ | src/cycle/config.rs (29 tests + 10 perm validation + 3 min_interval)
Permission Resolver    | ✅ | src/claude/permissions.rs (7 tests)
Cycle Executor         | ✅ | src/cycle/executor.rs (16 tests)
Claude CLI Builder     | ✅ | src/claude/cli.rs (8 tests)
Stream-JSON Parser    | ✅ | src/claude/stream.rs (23 tests)
Rich CLI Display      | ✅ | src/cli/display.rs (25 tests, includes status bar + health colors)
JSONL Logger          | ✅ | src/log/jsonl.rs (8 tests)
Cycle Rules Engine    | ✅ | src/cycle/rules.rs (8 tests + 6 frequency)
CLI Interface         | ✅ | src/main.rs (5 tests, rich display + safeguards)
cycles.toml           | ✅ | cycles.toml (coding + gardening, corrected perms)
Integration Tests     | ✅ | tests/integration_test.rs (6 tests)
Runtime Safeguards    | ✅ | circuit breaker + between-cycle gate
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

**Coding**: TODO.md task → plan → implement (TDD) → test → lint | Perms: Read|Edit(./src/**)|Edit(./tests/**)|Bash(cargo *)
**Gardening**: Deps update|refactor|docs|dead code|coverage | Perms: Read|Edit(./Cargo.toml)|Bash(cargo update *) | Triggers: after=[coding], min_interval=3
**Review**: Goal-backward verification — EXISTS|SUBSTANTIVE|WIRED checks | Perms: Read (read-only) | Triggers: after=[coding], min_interval=2
**Planning**: Analyze TODO|create plans|prioritize|scope tasks | Perms: Read|Edit(./TODO.md)|Edit(./plans/**)|Bash(git *)

All current cycles are single-step. Phase 2 adds multi-step cycles with session affinity (see [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md)).

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
├── TODO.md                ← Tasks by phase (read next)
├── PLANNING_QUESTIONS.md  ← Decisions & rationale
├── README.md              ← Public-facing docs
├── Cargo.toml             ← Deps + lints config
├── cycles.toml            ← Cycle definitions (coding + gardening)
├── .flow/                 ← Runtime state (gitignored)
│   ├── log.jsonl          ← Cycle execution history (8 entries from 2 dogfoods)
│   └── progress.json      ← Real-time progress (Phase 2)
├── plans/
│   ├── 001-mvp-pipeline-runner.md  ← Original MVP plan (superseded)
│   ├── 002-full-architecture.md    ← Complete architecture (read for deep dive)
│   ├── 003-multi-step-cycles.md    ← Multi-step cycles with session affinity (Phase 2)
│   └── TEMPLATE.md                  ← Template for new plans
├── src/
│   ├── main.rs            ← CLI entry (clap + execution loop)
│   ├── lib.rs             ← Public API
│   ├── cycle/
│   │   ├── config.rs      ← Parse cycles.toml (29 + 10 tests)
│   │   ├── executor.rs    ← Execute cycles (16 tests)
│   │   └── rules.rs       ← Dependency triggers (8 tests)
│   ├── claude/
│   │   ├── cli.rs         ← CLI command builder (8 tests)
│   │   ├── permissions.rs ← Permission resolver (7 tests)
│   │   ├── stream.rs      ← Stream-JSON parser (23 tests)
│   │   └── session.rs     ← Session manager (Phase 2, planned)
│   ├── cli/
│   │   └── display.rs     ← Rich CLI display (13 tests)
│   └── log/
│       └── jsonl.rs       ← JSONL logger (8 tests)
└── tests/
    └── integration_test.rs ← End-to-end tests (6 tests)
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

**Phase 1 is complete.** Next tasks are Phase 2:

**Medium**: Status bar — persistent bottom line during runs → `src/cli/display.rs`, `src/cycle/executor.rs`
**Medium**: `flow doctor` — log analysis + config lint + permission suggestions → `src/doctor.rs` (new)
**Medium**: Multi-iteration loop (`--max-iterations`) → `src/main.rs`
**Easy**: `flow init` — scaffold cycles.toml + .flow/ for new projects → `src/init.rs` (new)
**Medium**: `flow plan` — quick idea capture (`flow plan 'idea'`) + interactive deep planning (`flow plan`) → `src/plan.rs` (new)
**Hard**: Cycle selector (AI-driven cycle selection) → new module

All Phase 2 tasks have specs in TODO.md.

---

**Last Updated**: 2026-02-16 | **Status**: Phase 1 complete, Phase 2 in progress (frequency constraints done) | **Next Milestone**: Multi-step cycles + cycle selector
