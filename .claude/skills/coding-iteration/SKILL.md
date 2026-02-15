---
name: coding-iteration
description: Execute TDD workflow with RED/GREEN/REFACTOR phases and quality verification
---

# Coding Iteration Skill

Use this skill at the start of each coding iteration to follow the proper TDD workflow and quality standards.

## Phase 1: Orient & Select Task

1. **Read project context** (if not already familiar):
   - `AGENTS.md` - Project overview and current status
   - `TODO.md` - Available tasks by priority
   - Relevant architecture docs in `plans/`

2. **Select task**:
   - Pick from TODO.md Phase 1 tasks (prioritize P0)
   - Prefer independent, well-defined tasks
   - Consider: Easy (JSONL logger) → Medium (Config parser, Permissions) → Hard (Executor)

3. **Understand scope**:
   - Read architecture specification for the component
   - Identify dependencies (can it be built independently?)
   - Note file paths and expected interfaces

## Phase 2: TDD Implementation (RED → GREEN → REFACTOR)

### Step 1: RED - Write Failing Tests

1. Create test file/module first
2. Write comprehensive tests that describe the desired behavior
3. **Run tests to see them FAIL**: `cargo test <module>`
4. Verify tests fail for the right reason (not compilation errors)

### Step 2: GREEN - Minimal Implementation

1. Write the minimum code to make tests pass
2. Don't worry about perfect code yet - just make it work
3. **Run tests to see them PASS**: `cargo test <module>`
4. Verify all new tests pass

### Step 3: REFACTOR - Clean Up

1. Improve code quality while keeping tests green
2. Extract functions, improve naming, add documentation
3. **Run tests after each refactor**: `cargo test <module>`
4. Ensure tests stay green throughout

## Phase 3: Quality Verification

Before committing, run full verification:

```bash
# 1. Run all tests (not just your module)
cargo test-all

# 2. Check for clippy warnings
cargo clippy-all

# 3. Format code
cargo fmt

# 4. Verify no warnings in your new code
cargo clippy --lib -- -D warnings
```

**Note**: The pre-commit hook will enforce these, but run manually to catch issues early.

## Phase 4: Commit

1. **Update TODOs** (use `/update-todos` skill):
   - Mark completed tasks as `[x]` and update status
   - Move completed tasks to `## ✅ Completed` section
   - Add new discovered tasks if relevant
   - Update progress summary
   - Verify TODO.md is accurate before committing

2. **Stage relevant files**:
   ```bash
   git add <specific files>
   ```
   - Prefer explicit file paths over `git add .`
   - Avoid accidentally staging unrelated changes
   - **Always include TODO.md if it was updated**

3. **Create descriptive commit**:
   ```bash
   git commit -m "$(cat <<'EOF'
   <Imperative verb> <what was accomplished>

   <Why this change matters>

   Key changes:
   - Bullet point 1
   - Bullet point 2

   <Any relevant context>
   EOF
   )"
   ```

4. **Verify commit succeeded**:
   ```bash
   git status
   ```

## TDD Anti-Patterns to Avoid

❌ **Don't**:
- Write implementation before tests
- Write tests and implementation simultaneously
- Skip running tests to see them fail first
- Commit without running full verification
- Use `git add .` or `git add -A` (be explicit)
- Write too many tests before implementing (small iterations)

✅ **Do**:
- Write one test at a time
- See each test fail before implementing
- Make small, incremental changes
- Run tests frequently
- Keep refactoring steps small
- Commit working, tested code

## Example Workflow

```bash
# 1. Create test (RED)
# Write test in src/log/jsonl.rs:
# #[test]
# fn test_append_creates_file() { ... }

cargo test log::jsonl::test_append_creates_file
# ❌ FAIL - test should fail

# 2. Implement (GREEN)
# Write minimal JsonlLogger::append() implementation

cargo test log::jsonl::test_append_creates_file
# ✅ PASS - test now passes

# 3. Refactor
# Improve error handling, add docs, etc.

cargo test log::jsonl
# ✅ PASS - all tests still pass

# 4. Verify
cargo test-all && cargo clippy-all && cargo fmt

# 5. Commit
git add src/log/jsonl.rs
git commit -m "Implement JSONL append functionality"
```

## Component-Specific Guidelines

### For Config Parsers
- Test invalid TOML first (error cases)
- Test valid minimal config
- Test all optional fields
- Test schema validation

### For Core Logic
- Test edge cases first
- Test error conditions
- Test happy path last
- Use property-based testing if complex

### For I/O Operations
- Use tempfile for testing
- Test file creation, append, read
- Test error cases (permissions, missing files)
- Clean up resources in tests

## Success Criteria

An iteration is complete when:
- ✅ All tests pass (`cargo test-all`)
- ✅ Zero clippy warnings (`cargo clippy-all`)
- ✅ Code is formatted (`cargo fmt`)
- ✅ Changes are committed
- ✅ Implementation matches architecture spec
- ✅ Component is production-ready (error handling, docs)

## Phase 5: Reflect (After Iteration Complete)

**Use `/reflect` skill to evaluate the iteration:**
- What went well?
- What needed guidance?
- Are feedback loops fast enough?
- What concrete improvements are needed?

The `/reflect` skill provides a structured framework for identifying:
- Positive patterns to reinforce
- Friction points that needed intervention
- Feedback automation gaps
- Concrete, actionable improvements

**When to reflect:**
- After completing significant components
- When receiving corrective feedback
- When encountering repeated friction
- When user explicitly asks

See `~/.claude/skills/reflect.md` for the complete reflection framework.
