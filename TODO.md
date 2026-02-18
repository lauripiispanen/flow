# Flow - Task Queue

> Current pending work organized by priority. For completed work, see [COMPLETED.md](./COMPLETED.md).
> For architecture details, see [plans/002-full-architecture.md](./plans/002-full-architecture.md).

---

## Phase 2: Automation (wrapping up)

### P0 — Required to close Phase 2

- [ ] Update AGENTS.md to reflect current state
  - Priority: P0
  - Description: AGENTS.md is severely stale. Test counts say 270 (actual: 355). File organization is missing 6 source files (`context.rs`, `router.rs`, `session.rs`, `init.rs`, `doctor.rs`, `progress.rs`, `testutil.rs`). Component status test counts are outdated. Phase status still says "Next: Plan+plan-review steps" (done). Quick Start suggests tasks that are all completed. Docs cycle not mentioned in Cycle Types. Needs a full refresh.
  - Files: `AGENTS.md`

- [ ] Move completed Phase 2 items to COMPLETED.md
  - Priority: P0
  - Description: TODO.md has ~15 checked-off items cluttering it. Move them to COMPLETED.md under a "Phase 2: Automation" section, preserving completion dates and component details.
  - Files: `TODO.md`, `COMPLETED.md`

### P1 — Important Phase 2 follow-ups

- [x] Add `[selector]` config section to cycles.toml
  - Status: Completed
  - Priority: P1
  - Description: Added `SelectorConfig` struct to `config.rs` with `prompt` field, wired into `build_selector_prompt()` in `selector.rs`. Custom prompt replaces the hardcoded "Selection Criteria" section; absent/empty prompt preserves backward-compatible defaults. 6 new tests. Note: `[selector]` section still needs to be added to `cycles.toml` (blocked by permission model during this cycle).
  - Files: `src/cycle/config.rs`, `src/cycle/selector.rs`, `src/cycle/rules.rs`

- [x] Enrich selector context with recent outcomes
  - Status: Completed
  - Priority: P1
  - Description: Enriched `RecentOutcome` with `files_changed_count`, `tests_passed`, `cost_usd`, `duration_secs`, `denial_count` from `CycleOutcome`. Updated `format_log_summary()` to render compact enriched lines: `#3 coding [ok] 2m $1.23 3 files 42 tests: Implemented feature X`. Zero values (0 files, 0 tests, 0 denials) are omitted to reduce noise. 7 new tests.
  - Files: `src/cycle/selector.rs`

- [x] Periodic summary output every N iterations
  - Status: Completed
  - Priority: P1
  - Description: Added `summary_interval` to `GlobalConfig` (default 5, 0 = disabled). Added `total_cost_usd` to `RunProgress` (backward compat via serde default). `render_run_summary()` in `display.rs` formats a compact 4-line block: iteration progress, cycle breakdown (coding×3, gardening×2), success rate (4/5 succeeded), cost + duration. Wired into main loop via `print_periodic_summary()` + `should_print_summary()`. 12 new tests.
  - Files: `src/cycle/config.rs`, `src/log/progress.rs`, `src/cli/display.rs`, `src/main.rs`

- [x] Show iteration progress in status bar
  - Status: Completed
  - Priority: P1
  - Description: Added `iteration_context: Option<(u32, u32)>` to `StatusLine`. When `max > 1`, renders `[3/10] [coding] ▶ ...` prefix. Threaded through `execute_with_display()` → `execute_single_step()`/`execute_steps()` → `execute_and_log()`. Single-iteration runs (`max == 1`) suppress the prefix. Cumulative stats deferred — periodic summary already covers that. 4 new tests.
  - Files: `src/cli/display.rs`, `src/cycle/executor.rs`, `src/main.rs`

- [ ] `flow doctor --repair` auto-fix mode
  - Priority: P1
  - Description: Add `--repair` flag to the doctor subcommand. Auto-apply safe, non-destructive fixes: (1) add missing permissions suggested by D001, (2) set `min_interval` on triggered cycles per D004 suggestions. Read cycles.toml, apply fixes, write back, report what changed. Skip anything destructive.
  - Files: `src/doctor.rs`, `src/main.rs`, `src/cycle/config.rs` (needs a `write_config()` or similar)

### P2 — Nice to have (may defer to Phase 3)

- [ ] `flow plan '<idea>'` — quick idea capture
  - Priority: P2
  - Description: One-shot command. Claude reads TODO.md and project context, appends well-scoped tasks based on the idea. Example: `flow plan 'add timeout support'` would analyze the codebase and add specific TODO entries.
  - Files: new `src/plan.rs`, `src/main.rs`

- [ ] `flow plan` (no args) — interactive deep planning
  - Priority: P2
  - Description: Conversational planning session. Claude analyzes project state, produces TODO.md tasks and optional `plans/*.md` files. More thorough than quick capture.
  - Files: new `src/plan.rs`, `src/main.rs`

---

## Phase 3: Advanced Features (future)

- [ ] Template prompts with variables (e.g., `{{todo_file}}`, `{{project_name}}`)
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
