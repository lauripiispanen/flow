# Flow - Task Queue

> Current pending work organized by priority. For completed work, see [COMPLETED.md](./COMPLETED.md).
> For architecture details, see [plans/002-full-architecture.md](./plans/002-full-architecture.md).

---

## Phase 2: Automation (in progress)

### Iteration Context
- [x] Implement context modes (full, summaries, none)
  - Priority: P0
  - Description: The `context` field is parsed from config but not yet used. Inject log history into cycle prompts based on the mode.
  - Completed: 2026-02-17

- [x] Context injection into cycle prompts
  - Priority: P0
  - Completed: 2026-02-17

### Multi-Step Cycles (Session Reuse)
- [ ] Implement step executor with session affinity
  - Priority: P0
  - Plan: [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md)
  - Description: Execute steps sequentially within a cycle. Steps with matching session tags continue the same Claude Code session via `--continue`/`--resume`.

- [ ] Per-step permissions
  - Priority: P1
  - Description: Each step can have its own permissions (additive on top of cycle + global).

- [ ] Add plan + plan-review steps to coding cycle
  - Priority: P1
  - Depends on: step executor (multi-step cycles)
  - Description: Extend `cycles.toml` coding cycle into a multi-step cycle: (1) **plan** step — architect reads TODO.md and writes an implementation plan to `.flow/current-plan.md` (read + edit plan file only, no src writes); (2) **plan-review** step — reads the plan, writes approval/feedback to `.flow/plan-review.md`. The step executor calls `execute_with_display()` for each step, collects `CycleResult`, and checks the result before proceeding to the implement step. Intermediate artifacts stay on filesystem for inspectability (consistent with plan 003 design).

- [ ] Step-level routing: LLM-driven next-step selection
  - Priority: P1
  - Depends on: step executor (multi-step cycles)
  - Description: After each step completes, determine the next step to run. Default is `sequential` (proceed to the next step in TOML order). For steps that need conditional branching, allow an optional `router = "llm"` field in TOML: the step executor posts the completed step's `result_text` + the cycle's available step names to a model (same pattern as the cycle selector in `src/cycle/selector.rs`), and gets back a next-step name. This handles plan-review (approve → implement, block → plan), test steps (pass → proceed, fail → fix), security review (clean/warnings/critical → different paths), and any future multi-outcome structure — without hard-coding keyword signals. The router response should include a reason (loggable). Enforce a `max_visits` cap per step (default 3) to prevent infinite loops. Steps without `router` always proceed sequentially (backward compatible).

### Outcome Data Completeness
- [ ] Populate `files_changed` from stream data or git diff
  - Priority: P1
  - Description: Currently always empty. Parse from Edit/Write tool events or run `git diff --name-only` after cycle.

- [ ] Populate `tests_passed` from stream data or cargo output
  - Priority: P1
  - Description: Currently always 0. Parse from Bash tool results or run `cargo test` count after cycle.

### Multi-Cycle Health Tracking
- [ ] Track cumulative health across iterations
  - Priority: P0
  - Description: If N cycles in a row fail or have high denial rates, stop the whole run.

### Enhanced Observability
- [ ] Implement .flow/progress.json writer
  - Priority: P1

- [ ] Periodic summary output (every N iterations)
  - Priority: P1

### Commands
- [ ] `flow doctor --repair` auto-fix mode
  - Priority: P1
  - Description: Auto-apply safe fixes (add missing permissions, set recommended min_interval). Non-destructive only.

- [ ] `flow init` — scaffold cycles.toml + .flow/ for new projects
  - Priority: P1
  - Description: Static template with coding + gardening cycles and reasonable global permissions.

- [ ] `flow plan '<idea>'` — quick idea capture
  - Priority: P1
  - Description: One-shot. Claude reads TODO.md and project context, appends well-scoped tasks.

- [ ] `flow plan` (no args) — interactive deep planning
  - Priority: P1
  - Description: Conversational planning session. Produces TODO.md tasks, optional plans/*.md files.

---

## Phase 3: Advanced Features (future)

- [ ] Template prompts with variables
- [ ] Per-cycle timeout configuration
- [ ] Parallel cycle execution (wave-based)
- [ ] Model profiles (different models for coding/review/planning)
- [ ] State file (`.flow/state.md`) — compact living memory for cross-iteration continuity
- [ ] Pause/resume support — serialize run state, `flow resume` to continue
- [ ] Goal-backward gap closure loop (verify → plan gaps → execute → re-verify)
- [ ] Codebase mapping command — parallel analysis before first planning cycle
- [ ] Context window awareness — track token usage, warn when approaching limits
- [ ] Interactive `flow init` — detect project type, suggest tailored cycles

---

## Priority Levels

- **P0**: Required for current phase
- **P1**: Important, next up after P0s
- **P2**: Nice to have / future
