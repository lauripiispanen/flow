---
name: gardening
description: Maintain and improve the codebase — dependency updates, refactoring, docs, dead code removal, test coverage
---

# Gardening Skill

Use this skill to run the gardening cycle: maintenance and incremental improvement of the codebase.

## Step 1: Orient

1. Read `AGENTS.md` for project context and current status
2. Read `TODO.md` to understand what's been completed and what's in progress

## Step 2: Dependency Health

Check for outdated dependencies:

```bash
cargo outdated   # if available
```

Or review `Cargo.toml` manually for pinned versions that may be stale. Upgrade cautiously — one dep at a time, verify tests pass after each.

## Step 3: Dead Code & Complexity

Look for:
- Unused imports (`cargo clippy` will flag these)
- Dead functions or types not referenced anywhere
- Overly complex code that can be simplified
- Duplicated logic that can be extracted

## Step 4: Test Coverage

- Identify under-tested modules (look for modules with few or no `#[test]` blocks)
- Add tests for edge cases or error paths that are missing coverage
- Do NOT add trivial tests just to increase numbers — only add meaningful coverage

## Step 5: Documentation

- Add or improve doc comments for public items missing them
- Update `AGENTS.md` if anything is out of date
- Clarify confusing code with inline comments (only where logic is non-obvious)

## Step 6: Refactoring

Run refactorings that genuinely simplify the codebase:
- Extract repeated patterns into helpers
- Rename things that are confusingly named
- Reorganize code that has grown incoherent

Keep changes focused and incremental. One logical improvement per commit.

## Step 7: Verify

```bash
cargo test --lib && cargo clippy --lib && cargo fmt
```

Zero clippy warnings required. All existing tests must stay green.

## Step 8: Commit

Commit improvements with descriptive messages. Group related changes (e.g., all dep updates together, all refactors together).

## Constraints

- Do NOT break existing tests
- Do NOT add features — this is maintenance, not development
- Keep changes incremental and reviewable
- Zero clippy warnings required
