# Flow - Task Queue

> Current pending work organized by priority. For completed work, see [COMPLETED.md](./COMPLETED.md).
> For architecture details, see [plans/002-full-architecture.md](./plans/002-full-architecture.md).

---

## Phase 2: Automation (closing out)

### P0 — Required to close Phase 2

_(empty — all P0 items are complete)_

### P1 — Important Phase 2 follow-ups

- [x] `flow doctor --repair` auto-fix mode
  - Priority: P1
  - Status: Completed
  - Description: Added `--repair` flag to doctor subcommand. Auto-applies D001 (missing permissions) and D004 (`min_interval`) fixes using `toml_edit` for format-preserving TOML edits. Added `cycle_name` field to `Finding` struct. 13 new tests (10 doctor + 3 CLI).
  - Files: `src/doctor.rs`, `src/main.rs`, `Cargo.toml`

- [ ] Add `[global]` defaults explicitly to `cycles.toml`
  - Priority: P1
  - Status: **Blocked** — requires manual edit (no cycle has `Edit(./cycles.toml)` permission)
  - Description: Add `max_permission_denials = 10`, `circuit_breaker_repeated = 5`, `max_consecutive_failures = 3`, `summary_interval = 5` after the `[global]` permissions line. These are the current serde defaults — no behavior change, purely self-documenting. The `[selector]` section is already present (added in earlier iteration).
  - Files: `cycles.toml` line 8

- [ ] Add `Edit(./Cargo.toml)` to coding implement step permissions
  - Priority: P1
  - Status: **Blocked** — requires manual edit (no cycle has `Edit(./cycles.toml)` permission)
  - Description: Change implement step permissions from `["Edit(./TODO.md)", "Edit(./AGENTS.md)", "Bash(git *)"]` to `["Edit(./TODO.md)", "Edit(./AGENTS.md)", "Edit(./Cargo.toml)", "Bash(git *)"]`.
  - Files: `cycles.toml` line 92

---

## Phase 3: Advanced Features (future)

- [ ] Template prompts with variables (e.g., `{{todo_file}}`, `{{project_name}}`)
- [x] Per-cycle timeout configuration (`max_turns`, `max_cost_usd` on cycles and steps)
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
