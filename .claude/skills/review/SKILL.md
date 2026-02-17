---
name: review
description: Goal-backward code review — verify recently completed work EXISTS, is SUBSTANTIVE, and is WIRED correctly
---

# Review Skill

Use this skill to run the review cycle: a goal-backward verification of recently completed work.

## What Is Goal-Backward Review?

Instead of reading code forward and noting observations, start from what *should* be true and verify it is.

For each recently completed task, check three levels:

| Level | Question |
|-------|----------|
| **EXISTS** | Do the expected files, functions, structs, and tests actually exist? |
| **SUBSTANTIVE** | Is the implementation real — not stubs, not empty bodies, not TODOs? |
| **WIRED** | Is it connected? Can it actually be reached from the entry point? |

## Step 1: Orient

1. Read `AGENTS.md` and `TODO.md` to understand what work has been completed recently
2. Read `.flow/log.jsonl` to see recent cycle outcomes and iteration history

## Step 2: Identify What to Review

From the context above, list the recently completed tasks or components. Focus on work done in the last few iterations.

## Step 3: Verify Each Task

For each completed task, apply the three-level check:

### EXISTS Check
- Does the file exist? (e.g., `src/log/jsonl.rs`)
- Does the struct/function/trait exist?
- Are the tests present?

### SUBSTANTIVE Check
- Is the function body real, or is it `todo!()` / empty?
- Do tests actually assert something meaningful?
- Is error handling real, or just `unwrap()`?

### WIRED Check
- Is the module declared in `lib.rs` or `mod.rs`?
- Is the function called from the appropriate entry point?
- Does the CLI expose the expected flags/subcommands?
- Would an integration test reach this code?

## Step 4: Anti-Pattern Scan

Check for:
- Leftover `TODO` or `FIXME` comments in recently-committed code
- Dead code introduced by the recent work
- Inconsistent naming with surrounding code
- Functions that exist but are never called
- Tests that always pass (trivially true assertions)

## Step 5: Test Coverage Check

- Does the test coverage match the implementation scope?
- Are error paths tested, or only the happy path?
- Are edge cases covered?

## Step 6: Report Findings

Summarize clearly:
- What passed at each level (EXISTS / SUBSTANTIVE / WIRED)
- What has gaps — be specific: file, line number, what's missing
- Distinguish critical gaps (broken functionality) from minor ones (style, missing docs)

## Constraints

- **This is a read-only cycle. Do NOT modify any source files.**
- Be specific about gaps — vague feedback is not useful
- Report what you actually verified, not assumptions
- Don't rewrite the code in your head — check what's actually there
