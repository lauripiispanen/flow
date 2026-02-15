# Flow - Automated Coding Pipeline Runner

> **PREFERENCE**: Use retrieval-led reasoning over pre-training-led reasoning. Read the linked files for current project state, architecture, and tasks. Do not rely on general knowledge about build tools or pipelines—Flow has specific patterns defined in this repository.

---

## Quick Start for New Contributors

**Orient → Setup → Pick Task → Implement**

1. Read this file (3 min) → [TODO.md](./TODO.md) (2 min) → [plans/002-full-architecture.md](./plans/002-full-architecture.md) (5 min)
2. Run: `cargo build && cargo test-all` (verify setup)
3. Pick task from [TODO.md](./TODO.md) Phase 1 section
4. Follow TDD: Write tests → Implement → Refactor

**Suggested First Tasks**: Cycle config parser | JSONL logger | Permission resolver

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
cycles.toml: [global.permissions | [[cycle]]: name|prompt|permissions|after|context]
.flow/log.jsonl: {iteration|cycle|timestamp|outcome|files_changed|tests_passed|duration_secs}
.flow/progress.json: {started_at|current_iteration|max_iterations|current_cycle|current_status|cycles_executed}
```

**Components** → Files:
- Config parsing → `src/cycle/config.rs` | Parse cycles.toml TOML
- Permissions → `src/claude/permissions.rs` | Hierarchical additive merge (global+cycle)
- Executor → `src/cycle/executor.rs` | Build claude-code CLI command with --allowedTools flags
- CLI builder → `src/claude/cli.rs` | Construct subprocess invocation
- Streaming → `src/observe/stream.rs` | Real-time stdout/stderr to terminal
- JSONL logger → `src/log/jsonl.rs` | Append-only .flow/log.jsonl
- Rules engine → `src/cycle/rules.rs` | Parse "after: [cycles]", trigger dependents
- CLI interface → `src/main.rs` | Clap arg parsing, --cycle <name>

**Phases**:
- P1 (MVP): Manual execution `flow --cycle coding` | Dogfood target | TDD implementation | 28 tasks in TODO.md
- P2 (Auto): Multi-iteration `flow --max-iterations 20` | AI cycle selection | Balance+context+priority optimization
- P3 (Advanced): Templates | Timeouts | Cost tracking | Parallel cycles

**Full architecture**: [plans/002-full-architecture.md](./plans/002-full-architecture.md)

---

## Current Status

**Completed**: Project setup | Cargo config | Docs structure | Planning | JSONL Logger | Cycle Config Parser | Permission Resolver | Claude CLI Builder | Cycle Executor | Cycle Rules Engine | CLI Interface | cycles.toml | Auto-trigger dependent cycles
**In Progress**: Nothing
**Next**: First dogfood test (`flow --cycle coding`) | Integration tests | Pretty output (P1)

**Test Status**:
- ✅ 62 passing (58 lib + 4 main: pipeline + jsonl + config + permissions + cli + executor + rules + main)
- ❌ 3 failing (tests/pipeline_test.rs - intentionally unimplemented, TDD red state)

**Component Status**:
```
Cycle Config Parser    | ✅ | src/cycle/config.rs (17 tests)
Permission Resolver    | ✅ | src/claude/permissions.rs (7 tests)
Cycle Executor         | ✅ | src/cycle/executor.rs (12 tests)
Claude CLI Builder     | ✅ | src/claude/cli.rs (7 tests)
Output Streamer        | ✅ | Built into executor (async line-by-line streaming)
JSONL Logger          | ✅ | src/log/jsonl.rs (6 tests)
Cycle Rules Engine    | ✅ | src/cycle/rules.rs (8 tests)
CLI Interface         | ✅ | src/main.rs (clap --cycle, fail-fast, auto-trigger)
cycles.toml           | ✅ | cycles.toml (coding + gardening cycles)
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
**Gardening**: Deps update|refactor|docs|dead code|coverage | Perms: Read|Edit(./Cargo.toml)|Bash(cargo update *) | Triggers: after=[coding]
**Review**: Code review|security|docs check | Perms: Read (read-only)
**Planning**: Analyze TODO|create plans|prioritize | Perms: Read|Edit(./TODO.md)|Edit(./plans/**)

**Permission Model**: Hierarchical additive (global + per-cycle, only adds never removes). Uses native Claude Code `--allowedTools` syntax (e.g., `Read`, `Edit(./src/**)`, `Bash(cargo *)`)

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
├── cycles.toml            ← Cycle definitions (to be created)
├── .flow/                 ← Runtime state (gitignored)
│   ├── log.jsonl          ← Cycle execution history
│   └── progress.json      ← Real-time progress
├── plans/
│   ├── 001-mvp-pipeline-runner.md  ← Original MVP plan (superseded)
│   ├── 002-full-architecture.md    ← Complete architecture (read for deep dive)
│   └── TEMPLATE.md                  ← Template for new plans
├── src/
│   ├── main.rs            ← CLI entry
│   ├── lib.rs             ← Public API
│   ├── pipeline.rs        ← Original pipeline stub (being refactored to cycles)
│   ├── cycle/             ← To be created
│   │   ├── config.rs      ← Parse cycles.toml
│   │   ├── executor.rs    ← Execute cycles
│   │   └── rules.rs       ← Dependency triggers
│   ├── claude/            ← To be created
│   │   ├── cli.rs         ← CLI command builder
│   │   └── permissions.rs ← Permission resolver
│   ├── log/               ← To be created
│   │   ├── jsonl.rs       ← JSONL logger
│   │   └── progress.rs    ← Progress tracker
│   └── observe/           ← To be created
│       └── stream.rs      ← Output streaming
└── tests/
    ├── pipeline_test.rs   ← Integration tests (3 failing - TDD red)
    └── ...                ← More tests to be added
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
- [PLANNING_QUESTIONS.md](./PLANNING_QUESTIONS.md) - **Rationale**: Why we made specific design decisions
- [README.md](./README.md) - **Public docs**: User-facing documentation
- [Vercel AGENTS.md pattern](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals) - **Inspiration**: Source of this pattern

**Vercel Key Insight**: "An 8KB docs index embedded directly in AGENTS.md achieved 100% pass rate, while skills maxed out at 79%"

---

## Quick Wins for New Contributors

**Easy**: JSONL logger (independent, clear scope, good tests) → `src/log/jsonl.rs`
**Medium**: Cycle config parser (well-defined, TOML crate exists) → `src/cycle/config.rs`
**Medium**: Permission resolver (clear algorithm, good for TDD) → `src/claude/permissions.rs`
**Hard**: Cycle executor (integrates many components) → `src/cycle/executor.rs`

Pick based on your comfort level. All tasks have specs in plans/002-full-architecture.md.

---

**Last Updated**: 2026-02-14 | **Status**: Phase 1 planning complete, implementation starting | **Next Milestone**: First component with passing tests
