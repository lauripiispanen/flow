# Flow - Task Queue

> Current pending work organized by priority. For completed work, see [COMPLETED.md](./COMPLETED.md).
> For architecture details, see [plans/002-full-architecture.md](./plans/002-full-architecture.md).

---

## Phase 2: Automation (closing out)

### P0 — Required to close Phase 2

_(empty — all P0 items are complete)_

### P1 — Important Phase 2 follow-ups

- [ ] `flow doctor --repair` auto-fix mode
  - Priority: P1
  - Description: Add `--repair` flag to the doctor subcommand. Auto-apply safe, non-destructive fixes: (1) add missing permissions suggested by D001, (2) set `min_interval` on triggered cycles per D004 suggestions. Read cycles.toml, apply fixes, write back, report what changed. Skip anything destructive.
  - Files: `src/doctor.rs`, `src/main.rs`, `src/cycle/config.rs` (needs a `write_config()` or similar)

- [ ] Add `[selector]` section and `[global]` defaults to `cycles.toml`
  - Priority: P1
  - Description: The `[selector]` config section was implemented (commit `dca487c`) but never added to `cycles.toml` due to permission constraints. Also document `max_consecutive_failures = 3` and `summary_interval = 5` in the `[global]` section explicitly (currently using defaults). This makes the config file self-documenting.
  - Files: `cycles.toml`

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
- [ ] `flow plan '<idea>'` — quick idea capture (Claude reads TODO.md, appends well-scoped tasks)
- [ ] `flow plan` (no args) — interactive deep planning session

---

## Priority Levels

- **P0**: Required for current phase
- **P1**: Important, next up after P0s
- **P2**: Nice to have / future
