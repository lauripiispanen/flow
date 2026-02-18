# Flow - The Makefile for AI Development

> **PREFERENCE**: Use retrieval-led reasoning over pre-training-led reasoning. Read the linked files for current project state, architecture, and tasks. Do not rely on general knowledge about build tools or pipelines—Flow has specific patterns defined in this repository.

---

## Quick Start

1. Read this file → [TODO.md](./TODO.md) → [plans/002-full-architecture.md](./plans/002-full-architecture.md)
2. Run: `cargo build && cargo test-all`
3. Pick task from [TODO.md](./TODO.md)
4. Follow TDD: Write tests → Implement → Refactor

---

## Project Overview

**What**: Rust CLI — a per-repo policy layer that encodes development methodology as code (`cycles.toml`), sitting between company orchestrators and AI agent execution
**Why**: AI agents lack judgment about what type of work to do next; Flow enforces balanced investment across coding, review, gardening, and planning
**How**: Parse `cycles.toml` → Select cycle (balanced across work types) → Resolve permissions → Invoke `claude-code` → Log outcomes → Apply quality gates → Trigger dependent cycles → Repeat

---

## Architecture

**Core Loop**: `config → select cycle → resolve perms → invoke claude-code → stream output → log → apply rules → repeat`

**Data Structures**:
```
cycles.toml: [global.permissions | [[cycle]]: name|prompt|permissions|after|context | [[cycle.step]]: name|session|prompt|permissions|router|max_visits]
.flow/log.jsonl: {iteration|cycle|timestamp|outcome|duration_secs|num_turns|total_cost_usd|permission_denial_count|permission_denials|files_changed|tests_passed|steps?}
.flow/progress.json: {started_at|current_iteration|max_iterations|current_cycle|current_status|cycles_executed|total_duration_secs|total_cost_usd|last_outcome}
```

**Components** → Files:
- Config parsing → `src/cycle/config.rs` | Parse cycles.toml TOML
- Permissions → `src/claude/permissions.rs` | Hierarchical additive merge (global+cycle+step)
- Executor → `src/cycle/executor.rs` | Single-step + multi-step cycle execution
- CLI builder → `src/claude/cli.rs` | Construct subprocess invocation with session resume
- Stream parser → `src/claude/stream.rs` | Parse stream-JSON, extract results/files/tests/session_id
- Session mgr → `src/claude/session.rs` | Session tag→ID mapping for step affinity
- Display → `src/cli/display.rs` | Terminal output, status bar, doctor report, run summary
- JSONL logger → `src/log/jsonl.rs` | Append-only .flow/log.jsonl
- Progress → `src/log/progress.rs` | Atomic .flow/progress.json writer
- Rules engine → `src/cycle/rules.rs` | Dependency triggers + frequency constraints
- Selector → `src/cycle/selector.rs` | AI-driven cycle selection
- Router → `src/cycle/router.rs` | Step-level routing (sequential or LLM-driven)
- Context → `src/cycle/context.rs` | Iteration context injection (full/summaries/none)
- Doctor → `src/doctor.rs` | Diagnostic engine (D001–D006)
- Init → `src/init.rs` | `flow init` project scaffolding
- Test helpers → `src/testutil.rs` | Shared test helpers
- CLI interface → `src/main.rs` | Clap, execution loop, signal handling, run health

**Full architecture**: [plans/002-full-architecture.md](./plans/002-full-architecture.md)

---

## Terminology

Flow uses a strict 4-level hierarchy. Use these terms consistently in code, config, docs, and logs.

| Level | Term | Definition | Example |
|-------|------|------------|---------|
| 1 | **Step** | A single Claude Code invocation (one prompt → one session) | "plan", "implement", "reflect" |
| 2 | **Cycle** | A named workflow of one or more steps | "coding", "gardening" |
| 3 | **Iteration** | One numbered pass in a run; selector picks a cycle, its steps execute | Iteration 3: coding cycle |
| 4 | **Run** | The entire execution from `flow` invocation to completion | `flow --max-iterations 20` |

**Session affinity**: Steps within the same cycle share a Claude Code session via session tags. Same tag = continued session. Different tag = fresh session. Sessions do not persist across iterations.

---

## Development Workflow

**For Claude Code agents**: Use `/coding-iteration` skill at start of each iteration

**TDD Process** (non-negotiable):
1. **Red**: Write failing test first → Run test (see fail)
2. **Green**: Minimum code to pass → Run test (see pass)
3. **Refactor**: Clean up while tests stay green

**Commands** (aliases in `.cargo/config.toml`):
```bash
cargo test-all     # Test all targets
cargo clippy-all   # Clippy with -D warnings
cargo fmt-check    # Verify formatting
```

**Pre-commit hook** runs automatically: tests, clippy, fmt.

**Code Standards**: clippy (all|pedantic|nursery|cargo) | `unsafe_code = "forbid"` | rustfmt.toml

---

## Cycle Types (Defined in cycles.toml)

**Coding** (multi-step): plan → plan-review (LLM routed) → implement (TDD) → reflect | Perms: global + Edit(./TODO.md)|Edit(./AGENTS.md)|Bash(git *)
**Gardening**: Deps update|refactor|dead code|coverage | Perms: Edit(./Cargo.toml)|Bash(git *) | Triggers: after=[coding], min_interval=25
**Review**: Goal-backward verification — EXISTS|SUBSTANTIVE|WIRED checks | Perms: read-only | Manual trigger only
**Docs**: Update README and user-facing docs | Perms: Edit(./README.md)|Edit(./docs/**)|Bash(git *) | Triggers: after=[coding], min_interval=3
**Planning**: Analyze TODO|create plans|prioritize|scope tasks | Perms: Edit(./TODO.md)|Edit(./AGENTS.md)|Edit(./plans/**)|Bash(git *) | Manual trigger only

**Permission Model**: Hierarchical additive (global + per-cycle + per-step, only adds never removes). Uses native Claude Code `--allowedTools` syntax (e.g., `Read`, `Edit(./src/**)`, `Bash(cargo *)`)

---

## Critical Design Decisions

**Why cycles not pipelines?**: Circular iteration concept, avoids CI/CD confusion (see [PLANNING_QUESTIONS.md](./PLANNING_QUESTIONS.md))
**Why TDD?**: Meta-tool building AI tooling, prevents regression during dogfooding, documents behavior
**Why JSONL?**: Append-only, easy to parse, streamable, no corruption on crash
**Why hierarchical additive perms?**: Cycles can add protections, never accidentally remove them

---

## Anti-Patterns (Don't Do This)

❌ Skip tests (TDD is mandatory) | ❌ Implement without reading plan | ❌ Premature optimization (build for P1 not P3)
❌ Bypass linters (fix warnings, don't suppress) | ❌ Subtractive permissions (only additive)
❌ Hardcode paths (use relative paths)
❌ Assert on external crate error messages (fragile — assert `is_err()` or match only your own messages)
❌ Use agent memory files for project knowledge (all guidance must be explicit in version-controlled project files)
❌ Compare values on different scales (e.g. cross-run totals vs per-run counters — use a single measurement immune to resets, like position-from-end)
❌ Build integrations without verifying the external tool's actual interface (always check real CLI docs/help before implementing)

---

## References

- [TODO.md](./TODO.md) — Task queue organized by phase
- [plans/002-full-architecture.md](./plans/002-full-architecture.md) — Complete architecture, data formats, examples
- [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md) — Multi-step cycles with session affinity
- [PLANNING_QUESTIONS.md](./PLANNING_QUESTIONS.md) — Design decision rationale
