# Flow - Task Queue

> Current pending work organized by priority. For completed work, see [COMPLETED.md](./COMPLETED.md).
> For architecture details, see [plans/002-full-architecture.md](./plans/002-full-architecture.md).

**Vision**: Flow is the Makefile for AI development — a per-repo config that encodes development methodology as code. It sits between company orchestrators that dispatch work and AI agents that execute it, enforcing balanced investment across work types, quality gates, and permission boundaries.

---

## P0 — Policy Layer Foundations

These features make Flow trustworthy enough that an orchestrator can delegate to it and teams can run it unattended with confidence.

- [ ] Quality gates between cycles
  - Description: Configurable gates that must pass before a cycle's output is accepted: `require_tests_pass`, `require_lint_clean`, `require_no_new_warnings`. Failed gates mark the cycle failed and influence the next selector decision. This is the trust contract — what makes Flow's output reliable enough for automated pipelines.
  - Files: `src/cycle/executor.rs`, `src/cycle/config.rs`

- [ ] Investment categories on cycles
  - Description: Each cycle declares a `category` — `new`, `improving`, `keeping_the_lights_on`, `productivity`. The selector targets a configurable ratio across categories, not just ad-hoc heuristics. This is the core differentiator: investment balance as code. Add `[balance]` section to `cycles.toml` for target ratios and `flow stats` reporting.
  - Files: `src/cycle/config.rs`, `src/cycle/selector.rs`

- [ ] Human checkpoint gates
  - Description: Mark certain cycle transitions as requiring human approval before proceeding. Flow pauses, shows the output (e.g. a plan), and waits for approval. Bridges autonomous and supervised modes — teams start supervised and gradually go hands-off. Essential for trust-building during adoption.
  - Files: `src/cycle/executor.rs`, `src/main.rs`

---

## P1 — Integration & Intelligence

These make Flow useful as a component in larger systems and smarter over time.

- [ ] External work intake
  - Description: An input channel for company orchestrators to push work items to Flow — watched directory, GitHub issues with labels, simple file-based queue, or webhook. This is what completes the sandwich: orchestrator pushes tasks, Flow consumes them through balanced cycle machinery.

- [ ] Balance reporting (`flow stats`)
  - Description: Show investment distribution over the last N cycles/runs. "You spent 80% on coding, 5% on gardening — balance is skewed." The feedback loop that makes investment balance actionable. Also the data a company orchestrator would query.
  - Files: `src/main.rs`, `src/log/jsonl.rs`

- [ ] Model profiles per cycle/step
  - Description: `model = "opus"` for planning, `model = "sonnet"` for gardening, `model = "haiku"` for diagnostics. Maps to `--model` flag. The selector itself could run on haiku. Easy win for cost optimization.
  - Files: `src/cycle/config.rs`, `src/claude/cli.rs`

- [ ] Agent Teams integration
  - Description: Allow a cycle step to declare `team = true` or `workers = 3`. Flow spawns an Agent Teams session for that step. The step prompt becomes the team lead's instructions. Flow provides the **what** and **when**, Agent Teams provides the **how many** and **how fast**.
  - Files: `src/cycle/config.rs`, `src/cycle/executor.rs`

- [ ] Learning from history
  - Description: Analyze JSONL logs to improve cycle selection: "coding cycles after planning have 80% success rate; coding without planning has 40%." Feed empirical data into the selector prompt.
  - Files: `src/cycle/selector.rs`, `src/log/jsonl.rs`

---

## P2 — Capabilities & Polish

- [ ] Parallel cycle execution (wave-based)
- [ ] State file (`.flow/state.md`) — compact living memory for cross-iteration continuity
- [ ] Pause/resume support — serialize run state, `flow resume` to continue
- [ ] Goal-backward gap closure loop (verify → plan gaps → execute → re-verify)
- [ ] Codebase mapping command — parallel analysis before first planning cycle
- [ ] Context window awareness — track token usage, warn when approaching limits
- [ ] Interactive `flow init` — detect project type, suggest tailored cycles
- [ ] `flow plan '<idea>'` — quick idea capture (Claude reads TODO.md, appends well-scoped tasks)
- [ ] `flow plan` (no args) — interactive deep planning session
- [ ] Dry-run mode — show what cycles would be selected and what prompts would be sent
- [ ] GitHub Actions integration — `flow run` as a GitHub Action, triggered by labels/schedule
- [ ] Notification webhooks — cycle completion/failure alerts to Slack, Discord, etc.
- [ ] Template library — shareable `cycles.toml` templates for common stacks

---

## Housekeeping (blocked)

- [ ] Add `[global]` defaults explicitly to `cycles.toml`
  - Status: **Blocked** — requires manual edit (no cycle has `Edit(./cycles.toml)` permission)
  - Description: Add `max_permission_denials = 10`, `circuit_breaker_repeated = 5`, `max_consecutive_failures = 3`, `summary_interval = 5` after the `[global]` permissions line. Purely self-documenting, no behavior change.

- [ ] Add `Edit(./Cargo.toml)` to coding implement step permissions
  - Status: **Blocked** — requires manual edit (no cycle has `Edit(./cycles.toml)` permission)

---

## Priority Levels

- **P0**: Policy layer foundations — makes Flow trustworthy as a delegated policy layer
- **P1**: Integration & intelligence — connects Flow to larger systems, makes it smarter
- **P2**: Capabilities & polish — important but not on the critical path to the vision
