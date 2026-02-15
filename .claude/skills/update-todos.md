# Update TODOs Skill

Use this skill to keep TODO.md synchronized with completed work. Run this BEFORE committing code.

## Why Update TODOs?

- **Tracks progress**: Shows what's complete vs. what's left
- **Prevents confusion**: Avoids "Not Started" for completed work
- **Enables planning**: Clear view of what to work on next
- **Documents velocity**: History of completed tasks
- **Supports dogfooding**: Flow will read TODO.md to select tasks

## When to Update TODOs

**Always update before committing when:**
- ✅ You completed a task listed in TODO.md
- ✅ You completed part of a multi-step task
- ✅ You discovered new tasks that should be tracked
- ✅ Task priorities changed based on learnings

**Don't update for:**
- ❌ Trivial fixes not tracked in TODO.md
- ❌ Refactoring that doesn't complete a task
- ❌ Work-in-progress (wait until complete)

## Update Process

### Step 1: Identify Completed Tasks

Read through what you implemented and find matching tasks in TODO.md:

```bash
# Quick scan for relevant sections
grep -A 5 "JSONL Logger" TODO.md
grep -A 5 "Cycle Config" TODO.md
```

### Step 2: Mark Tasks as Complete

For each completed task:

1. **Change checkbox**: `[ ]` → `[x]`
2. **Update status**: `Status: Not Started` → `Status: Completed`
3. **Add completion date** (optional): `Completed: 2026-02-14`

**Example:**
```markdown
# Before
- [ ] Implement .flow/log.jsonl writer
  - Status: Not Started
  - Priority: P0
  - Files: `src/log/jsonl.rs`

# After
- [x] Implement .flow/log.jsonl writer
  - Status: Completed
  - Priority: P0
  - Files: `src/log/jsonl.rs`
  - Completed: 2026-02-14
```

### Step 3: Move to Completed Section

Cut completed tasks and move to the `## ✅ Completed` section:

**Format:**
```markdown
## ✅ Completed

### [Date] - [Component Name]
- [x] Task 1
- [x] Task 2
- [x] Task 3

**Files:** `src/log/jsonl.rs`, `src/log/mod.rs`
**Tests:** 6 tests passing
**Commit:** [commit hash or reference]
```

### Step 4: Add New Tasks (If Discovered)

If implementation revealed new tasks:

```markdown
### New Tasks Discovered
- [ ] Add integration test for JSONL log rotation
  - Status: Not Started
  - Priority: P2
  - Discovered during: JSONL logger implementation
```

### Step 5: Update Progress Summary

If TODO.md has a progress summary or percentage, update it:

```markdown
**Phase 1 Progress: 3/28 tasks complete (11%)**
```

## Task Update Template

Use this template for moving tasks to completed:

```markdown
### [YYYY-MM-DD] - [Component Name]

**Completed:**
- [x] Task 1 description
- [x] Task 2 description
- [x] Task 3 description

**Implementation:**
- Files: `src/path/to/file.rs`
- Tests: X tests passing
- Coverage: [Brief description]
- Commit: [Short commit message or hash]

**Notes:**
- [Any important context or decisions made]
- [New tasks discovered]
- [Deviations from original plan]
```

## Examples

### Example 1: JSONL Logger Completion

**Before:**
```markdown
### JSONL Logger
- [ ] Implement .flow/log.jsonl writer
  - Status: Not Started
  - Priority: P0

- [ ] Append-only log entries
  - Status: Not Started
  - Priority: P0

- [ ] Serialize cycle outcomes to JSON
  - Status: Not Started
  - Priority: P0
```

**After (moved to Completed section):**
```markdown
## ✅ Completed

### 2026-02-14 - JSONL Logger

**Completed:**
- [x] Implement .flow/log.jsonl writer
- [x] Append-only log entries
- [x] Serialize cycle outcomes to JSON

**Implementation:**
- Files: `src/log/jsonl.rs`, `src/log/mod.rs`
- Tests: 6 comprehensive tests passing
- Coverage: CycleOutcome struct, JsonlLogger, read/write operations
- Commit: "Implement JSONL logger for cycle execution history"

**Notes:**
- Added chrono dependency for ISO 8601 timestamps
- Used tempfile for testing
- Full error handling with anyhow
- Zero clippy warnings
```

### Example 2: Partial Task Completion

If you only completed part of a larger task:

```markdown
### Cycle Executor
- [x] Implement CycleExecutor struct
  - Status: Completed
  - Priority: P0
  - Completed: 2026-02-14

- [ ] Build Claude Code CLI command with -p flags
  - Status: In Progress
  - Priority: P0
  - Started: 2026-02-14

- [ ] Stream stdout/stderr to terminal
  - Status: Not Started
  - Priority: P0
```

Keep partially complete tasks in the original section until fully done.

## Integration with Coding Iteration

This skill should be called in Phase 4 (Commit) of `/coding-iteration`:

```
Phase 4: Commit
├─ Step 1: Update TODOs (/update-todos)
├─ Step 2: Stage relevant files
├─ Step 3: Create descriptive commit
└─ Step 4: Verify commit succeeded
```

## Automation Opportunities

**Current state:** Manual updates
**Future:** Could be automated with:
- Pre-commit hook that checks if TODO.md needs updates
- Script to detect completed tasks based on git diff
- AI-powered TODO sync based on commit messages

For now, manual updates ensure accuracy and mindful tracking.

## Common Mistakes

❌ **Don't:**
- Mark tasks complete without verifying they're fully done
- Leave completed tasks in "Not Started" sections
- Forget to update status fields
- Mix completed and incomplete tasks in same section

✅ **Do:**
- Only mark tasks complete when all acceptance criteria met
- Move completed tasks to dedicated section
- Update both checkboxes AND status fields
- Group completed tasks by date or component
- Add brief notes about implementation decisions

## Verification Checklist

Before committing, verify:
- [ ] All completed work has corresponding checked tasks
- [ ] Status fields updated (`Completed`, dates added)
- [ ] Completed tasks moved to ✅ Completed section
- [ ] New discovered tasks added if relevant
- [ ] Progress summary updated if it exists
- [ ] TODO.md is properly formatted (no syntax errors)

## Quick Reference

```bash
# Find your tasks
grep -A 3 "Logger\|Config\|Permission" TODO.md

# Check completed section
tail -n 50 TODO.md

# Verify formatting
# (Just read the file, ensure checkboxes and structure correct)
```

---

**Remember:** TODO.md is the source of truth for project status. Keep it accurate and up-to-date.
