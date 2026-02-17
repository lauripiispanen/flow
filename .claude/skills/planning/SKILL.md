---
name: planning
description: Analyze project state, prioritize TODO.md, create or refine plans for upcoming work
---

# Planning Skill

Use this skill to run the planning cycle: analyze the project's current state and organize what comes next.

## Step 1: Orient

1. Read `AGENTS.md` — understand project architecture, current status, and phase
2. Read `TODO.md` thoroughly — what's done, in progress, blocked, and pending
3. Read recent entries in `.flow/log.jsonl` — understand velocity, repeated failures, and patterns

## Step 2: Assess Current State

Answer these questions:

- **Scope**: Are Phase 2 (or current phase) tasks properly scoped and prioritized?
- **Hidden work**: Are there implicit tasks not yet captured in TODO.md?
- **Follow-up**: Do any completed items have follow-up work needed?
- **Blockers**: Are any tasks blocked? Why? Is the blocker documented?
- **Sizing**: Are tasks small enough for a single coding cycle to complete?

## Step 3: Update TODO.md

Make TODO.md accurate and actionable:

1. **Re-prioritize**: Move tasks up/down based on current understanding
2. **Add new tasks**: Capture discovered work with priority and context
3. **Mark blockers**: Note tasks that are blocked and explain why
4. **Remove stale tasks**: Delete tasks that are no longer relevant
5. **Sharpen descriptions**: Make each task specific enough that a coding cycle can pick it up cold

Each task in TODO.md should be:
- Completable in a single coding cycle
- Specific about what needs to be done (not vague like "improve X")
- Linked to the relevant source file or plan document

## Step 4: Create or Update Plans (If Needed)

If a new plan is needed:
- Create it in `plans/` following `plans/TEMPLATE.md` format
- Reference the plan from the relevant TODO.md task
- Keep plans focused on one area (don't mix concerns)

If an existing plan is outdated:
- Update the relevant sections
- Note what changed and why

## Step 5: Commit

Commit your changes with a descriptive message explaining what you reorganized or added and why.

## Constraints

- Focus on **actionable, well-scoped tasks** — the output of planning is better TODO.md entries, not essays
- Every task should be completable in a single coding cycle
- Keep TODO.md clean — remove stale items, don't let it become a graveyard
- Do not start implementing — this cycle is analysis and organization only
