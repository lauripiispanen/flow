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

- [ ] Enrich selector context with recent outcomes
  - Priority: P1
  - Depends on: `[selector]` config (can be done independently but better sequenced after)
  - Description: The selector currently gets cycle counts and TODO priorities but not a summary of recent outcomes. Feed the selector the last 5-10 cycle outcomes (name, success/failure, files changed, key accomplishments) so it can make better routing decisions — e.g., skip gardening if last gardening found nothing, prioritize review if several coding cycles completed without one.
  - Files: `src/cycle/selector.rs`, `src/main.rs`

- [ ] Periodic summary output every N iterations
  - Priority: P1
  - Description: During multi-iteration runs, print a summary every N iterations (configurable, default 5): total cost so far, cycle breakdown (coding×4, gardening×1), test count trend, success rate. Helps users monitor long runs without reading raw output. Data is already available in `RunProgress` and log entries.
  - Files: `src/cli/display.rs`, `src/main.rs`

- [ ] Show iteration progress in status bar
  - Priority: P1
  - Description: The status bar currently shows `[coding] ▶ 12 turns | $1.23 | 2m 15s | 0 errors`. Add iteration context: `[3/10] [coding] ▶ ...` and cumulative stats: `[coding×2, gardening×1]`. Requires passing iteration number and `RunProgress` into the status bar renderer.
  - Files: `src/cli/display.rs`, `src/cycle/executor.rs`

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
