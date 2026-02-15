# Flow - Planning Questions

**Status**: Awaiting answers
**Date**: 2026-02-14

## Context

We're planning the full architecture for Flow before breaking it down into TODO.md tasks. Please answer the questions below by filling in your responses.

---

## Decisions Made So Far

✅ **Cycle definitions**: YAML/TOML config file
✅ **Outcome capture**: Pluggable (start with Claude summary, extend to git/tests/lints later)
✅ **Initial cycles**: Coding, Review, Gardening, Planning
✅ **Log storage**: JSONL (JSON Lines) format

---

## Questions Needing Answers

### 1. Cycle Selection Logic

When the selection model picks the next cycle, what should it optimize for?

**Options:**
- [ ] Balance (ensure all cycle types run eventually)
- [ ] Context-aware (if tests are failing, prioritize gardening)
- [ ] User-defined priorities (P0 tasks first, etc.)
- [ ] All of the above
- [ ] Other: _______________

**Your answer:**
```
All of the above
```

---

### 2. Cycle Rules Syntax

For rules like "always run gardening after coding", should this be:

**Options:**
- [ ] In the cycles config (e.g., `after: ["coding"]` in gardening definition)
- [ ] Separate rules file (e.g., `rules.yaml`)
- [ ] Both (config for simple rules, separate file for complex logic)
- [ ] Other: _______________

**Your answer:**
```
Start with cycles config for now
```

---

### 3. Permission Allowlists

Should allowlists be:

**Options:**
- [ ] Per-cycle (coding cycle gets more permissions than review)
- [ ] Global (same allowlist for all cycles)
- [ ] Configurable per-project
- [ ] All of the above (hierarchical: global defaults + per-cycle overrides)
- [ ] Other: _______________

**Your answer:**
```
hierarchical but only additive
```

---

### 4. Failure Handling

If a cycle fails (Claude errors, tests fail, etc.), should Flow:

**Options:**
- [ ] Stop immediately (fail-fast)
- [ ] Skip and try next cycle (continue on error)
- [ ] Retry the same cycle (with max retry count)
- [ ] Run a specific "recovery" cycle
- [ ] Configurable per cycle
- [ ] Other: _______________

**Your answer:**
```
stop. no recovery yet since we need to understand what kind of errors happen
```

---

### 5. Gardening Cycle Contents

What should the gardening cycle include specifically?

**Options:**
- [ ] `cargo fmt` (formatting)
- [ ] `cargo clippy --fix` (auto-fix lints)
- [ ] Dependency updates (cargo update, version bumps)
- [ ] Refactoring suggestions
- [ ] Documentation improvements
- [ ] Dead code removal
- [ ] Test coverage improvements
- [ ] All of the above
- [ ] Other: _______________

**Your answer:**
```
fmt & clippy are part of coding direct feedback. others are periodical so suitable for gardening cycle
```

---

### 6. Dogfooding Timeline

When should we start using Flow to build Flow?

**Options:**
- [ ] After basic MVP (manual single cycle execution)
- [ ] After cycle selection works
- [ ] After first successful multi-iteration run
- [ ] Gradually (use it for specific tasks while building)
- [ ] Other: _______________

**Your answer:**
```
start with manual single cycle run ASAP and look into automating once that works
```

---

### 7. Iteration Context

Should cycles see what happened in previous iterations?

**Options:**
- [ ] Yes, always (full JSONL log available to selection model)
- [ ] Yes, but summarized (last N iterations or summary)
- [ ] No (each cycle is independent)
- [ ] Configurable per cycle
- [ ] Other: _______________

**Your answer:**
```
depends on cycle. some may only need instruction summaries. others may look at what previous tasks have done and some may even use a memory file to remember what they have done in earlier cycles and may refine their own working (so I guess configurable? or maybe a cli call the cycle can invoke if needed?)
```

---

### 8. Cost Tracking

Should Flow track API costs?

**Options:**
- [ ] Yes, per cycle
- [ ] Yes, per iteration
- [ ] Yes, global total
- [ ] All of the above
- [ ] No (not for MVP)
- [ ] Other: _______________

**Your answer:**
```
later, but not MVP
```

---

### 9. Time Limits

What timeout mechanisms should exist?

**Options:**
- [ ] Timeout per cycle (e.g., 10 minutes max)
- [ ] Global timeout for entire run
- [ ] No timeouts (let it run)
- [ ] Configurable per cycle type
- [ ] All of the above
- [ ] Other: _______________

**Your answer:**
```
none in MVP but later configurable per cycle type
```

---

### 10. Rollback Capability

If an iteration goes wrong, should we support rollback?

**Options:**
- [ ] Yes, git-based (revert to commit before iteration)
- [ ] Yes, with confirmation prompt
- [ ] No (trust the process, fix forward)
- [ ] Manual only (user triggers rollback)
- [ ] Other: _______________

**Your answer:**
```
manual only
```

---

### 11. Observability

How should we monitor progress during a multi-iteration run?

**Options:**
- [ ] Real-time terminal output (streaming logs)
- [ ] Progress file that updates (.flow/progress.json)
- [ ] Periodic summaries (every N iterations)
- [ ] All of the above
- [ ] Minimal (just JSONL log, check after completion)
- [ ] Other: _______________

**Your answer:**
```
all of the above
```

---

### 12. Parallel Cycles

Should we ever run multiple cycles in parallel?

**Options:**
- [ ] No, always sequential (one cycle at a time)
- [ ] Yes, if cycles are independent (e.g., docs + tests)
- [ ] Configurable (some cycles can run in parallel)
- [ ] Future feature (not for MVP)
- [ ] Other: _______________

**Your answer:**
```
future feature for independent cycles
```

---

### 13. Cycle Prompt Format

How should cycle prompts be structured in the config?

**Options:**
- [ ] Simple string (single prompt for entire cycle)
- [ ] Template with variables (e.g., `{{current_task}}`, `{{previous_summary}}`)
- [ ] Multi-part (system prompt + user prompt)
- [ ] All of the above
- [ ] Other: _______________

**Your answer:**
```
MVP simple string but later all of the above
```

---

### 14. What Else Am I Forgetting?

Any other concerns, features, or considerations we should plan for?

**Your thoughts:**
```
I do think iterative work on this tool will yield a lot of good ideas so important to start dogfooding ASAP
```

---

## Next Steps

Once you've filled this out, I'll:
1. Create comprehensive plan in `plans/002-full-architecture.md`
2. Break down into concrete tasks in `TODO.md`
3. Update `AGENTS.md` with the architecture
4. Start implementing with TDD approach
