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
- [x] Implement step executor with session affinity
  - Priority: P0
  - Plan: [plans/003-multi-step-cycles.md](./plans/003-multi-step-cycles.md)
  - Completed: 2026-02-17
  - Components: `StepConfig` in config.rs, `SessionManager` in session.rs, multi-step execution in executor.rs, `StepOutcome` in jsonl.rs, `resolve_step_permissions` in permissions.rs, `build_command_with_session` in cli.rs, `StreamAccumulator.session_id` capture

- [x] Per-step permissions
  - Priority: P1
  - Completed: 2026-02-17
  - Description: Each step can have its own permissions (additive on top of cycle + global). Implemented as `resolve_step_permissions(global, cycle, step)`.

- [x] Add plan + plan-review steps to coding cycle
  - Priority: P1
  - Completed: 2026-02-17
  - Description: Converted `cycles.toml` coding cycle into a three-step multi-step cycle: (1) **plan** step — architect session reads TODO.md/AGENTS.md and writes implementation plan to `.flow/current-plan.md`; (2) **plan-review** step — architect continues session, critically evaluates the plan, writes APPROVED/REJECTED to `.flow/plan-review.md`, can `exit 1` to halt cycle on rejection; (3) **implement** step — coder session reads the approved plan and implements with TDD. Added config test `test_actual_cycles_toml_coding_is_multi_step` to verify real cycles.toml structure.

- [ ] Step-level routing: LLM-driven next-step selection
  - Priority: P1
  - Depends on: step executor (multi-step cycles)
  - Description: After each step completes, determine the next step to run. Default is `sequential` (proceed to the next step in TOML order). For steps that need conditional branching, allow an optional `router = "llm"` field in TOML: the step executor posts the completed step's `result_text` + the cycle's available step names to a model (same pattern as the cycle selector in `src/cycle/selector.rs`), and gets back a next-step name. This handles plan-review (approve → implement, block → plan), test steps (pass → proceed, fail → fix), security review (clean/warnings/critical → different paths), and any future multi-outcome structure — without hard-coding keyword signals. The router response should include a reason (loggable). Enforce a `max_visits` cap per step (default 3) to prevent infinite loops. Steps without `router` always proceed sequentially (backward compatible).

### Outcome Data Completeness
- [x] Populate `files_changed` from stream data or git diff
  - Priority: P1
  - Description: Parse file paths from Edit/Write ToolUse events in the stream. StreamAccumulator now tracks `files_changed` (deduplicated). Wired through CycleResult → build_outcome → CycleOutcome.
  - Completed: 2026-02-18

- [x] Populate `tests_passed` from stream data or cargo output
  - Priority: P1
  - Description: Parse `N passed` from cargo test output in non-error ToolResult content. StreamAccumulator accumulates `tests_passed` across all tool results. Wired through CycleResult → build_outcome → CycleOutcome.
  - Completed: 2026-02-18

### Multi-Cycle Health Tracking
- [x] Track cumulative health across iterations
  - Priority: P0
  - Description: If N cycles in a row fail or have high denial rates, stop the whole run.
  - Completed: 2026-02-17

### Enhanced Observability
- [ ] Implement .flow/progress.json writer
  - Priority: P1

- [ ] Periodic summary output (every N iterations)
  - Priority: P1

### Commands
- [ ] `flow doctor --repair` auto-fix mode
  - Priority: P1
  - Description: Auto-apply safe fixes (add missing permissions, set recommended min_interval). Non-destructive only.

- [x] `flow init` — scaffold cycles.toml + .flow/ for new projects
  - Priority: P1
  - Description: Static template with coding + gardening cycles and reasonable global permissions.
  - Completed: 2026-02-17
  - Components: `src/init.rs` (new) with `init()` fn + `CYCLES_TOML_TEMPLATE`, `Init` subcommand in main.rs, `run_init()` handler. 13 lib tests + 1 main test.

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
