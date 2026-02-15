---
name: reflect
description: Reflect on a completed coding iteration to identify improvements
---

# Iteration Reflection Skill

Use this skill at the END of each coding iteration to reflect on what happened and identify improvements.

## Reflection Framework

After completing an iteration, systematically evaluate:

### 1. What Went Well? ✅

Identify positive patterns to reinforce:
- Did the task selection work well?
- Was the component well-defined?
- Did tests provide good coverage?
- Was the code quality high?
- Did documentation help?
- Were dependencies minimal?

**Example from JSONL Logger iteration:**
- ✅ AGENTS.md pattern worked perfectly - quick orientation
- ✅ Component selection excellent (independent, foundational, testable)
- ✅ Zero clippy warnings, comprehensive tests
- ✅ Clean module organization

### 2. What Needed Guidance? ⚠️

Identify friction points that required user intervention:
- Were there ambiguities in requirements?
- Did I make incorrect assumptions?
- Were there preference mismatches?
- Did I need correction on approach?
- Were there multiple back-and-forth interactions for simple things?

**Example from JSONL Logger iteration:**
- ⚠️ Git attribution preference - required correction + settings update (2 interactions)
- ⚠️ Not pure TDD - wrote tests + implementation together instead of RED→GREEN→REFACTOR

### 3. Feedback Automation Analysis 🔧

Evaluate the development feedback loop:

**Questions to ask:**
- Were there manual steps that could be automated?
- Did I forget to run important checks?
- Could errors have been caught earlier?
- Are there repetitive commands that should be scripted?
- Is the feedback loop fast enough?
- Are there missing guardrails?

**Example issues identified:**
- ❌ No pre-commit verification - easy to commit broken code
- ❌ Manual process - have to remember all verification commands
- ❌ No continuous feedback for TDD
- ❌ Didn't verify tooling before using it

### 4. Concrete Action Items 📋

**Critical:** Don't just identify problems - propose concrete, actionable solutions.

**Format each action item as:**
```
Priority: [High/Medium/Low]
Problem: [What's broken or missing]
Solution: [Specific, implementable fix]
Effort: [Small/Medium/Large]
```

**Example action items from JSONL Logger iteration:**

```
Priority: High
Problem: No pre-commit verification - can commit broken code
Solution: Add .git/hooks/pre-commit that runs tests, clippy, fmt
Effort: Small

Priority: High
Problem: Manual verification is error-prone
Solution: Document automated pre-commit hook
Effort: Small

Priority: Medium
Problem: TDD workflow not clearly defined
Solution: Create /coding-iteration skill with RED→GREEN→REFACTOR
Effort: Medium

Priority: Low
Problem: Suggested cargo-watch but doesn't fit my execution model
Solution: Skip watch mode (doesn't make sense for blocking tools)
Effort: None
```

## Reflection Template

Copy this template for each iteration reflection:

```markdown
## Iteration: [Component Name]

### ✅ What Went Well
-
-
-

### ⚠️ What Needed Guidance
-
-

### 🔧 Feedback Automation Analysis
Current gaps:
-

Potential improvements:
-

### 📋 Action Items

**High Priority:**
1. [Problem] → [Solution] (Effort: X)
2.

**Medium Priority:**
1. [Problem] → [Solution] (Effort: X)

**Low Priority / Future:**
1.

### Action Item Disposition (ALL items must be addressed)

| # | Item | Disposition | Artifact |
|---|------|-------------|----------|
| 1 | ... | Implemented / Added to TODO.md / Added to AGENTS.md | link or file |
| 2 | ... | ... | ... |
```

## When to Reflect

**Trigger reflection when:**
1. Completing a significant component
2. User provides corrective feedback
3. Encountering repeated friction
4. Finishing a multi-step task
5. User explicitly asks "/reflect"

**Don't reflect on:**
- Trivial changes (typo fixes, formatting)
- Middle of a flow (wait until component done)
- When explicitly told to skip it

## Acting on Reflections (MANDATORY)

**Action items are not optional.** After reflection, you MUST execute them before moving on. Reflection without follow-through is waste.

After identifying action items:

1. **Implement immediately by default:**
   - High priority → Do it now, before the next coding iteration
   - Medium priority → Do it now if small effort, otherwise add to TODO.md with full context
   - Low priority → Add to TODO.md with full context
   - **Every action item must result in either a code change or a TODO.md entry — no items left hanging**

2. **For each action item, do one of:**
   - **Implement**: Make the change (update docs, add validation, fix the process)
   - **Document in TODO.md**: Add as a tracked task with priority, description, and context
   - **Add to AGENTS.md**: If it's a process/anti-pattern lesson, add it to the anti-patterns section
   - Never just "note it" without a concrete artifact

3. **Verify completeness:**
   - Before ending the reflection, review every action item and confirm it was either implemented or tracked
   - If an action item was skipped, explain why explicitly

## Meta-Improvement

This skill itself should improve over time:

- If reflection finds consistent patterns, update this skill
- If certain categories are never useful, remove them
- If new types of issues emerge, add categories
- Keep it practical - reflection should take < 5 min

## Example: Complete Reflection

```markdown
## Iteration: JSONL Logger Implementation

### ✅ What Went Well
- AGENTS.md structure provided perfect orientation
- Task selection was optimal (independent, foundational)
- 6 comprehensive tests with 100% coverage
- Zero clippy warnings achieved
- Clean separation of concerns

### ⚠️ What Needed Guidance
- Git attribution: didn't know user preference, needed 2 interactions to fix
- TDD process: wrote tests + impl together instead of RED first

### 🔧 Feedback Automation Analysis
- Missing: Pre-commit validation hook
- Missing: Automated verification before commit
- Missing: Clear TDD workflow documentation
- Issue: Suggested tools (cargo-watch) that don't fit my execution model

### 📋 Action Items

**High Priority:**
1. Add pre-commit hook → Auto-run tests/clippy/fmt (Small)
2. Create /coding-iteration skill → Document proper TDD (Medium)
3. Update settings.json → Disable git attribution (Small)

**Medium Priority:**
1. Update AGENTS.md → Reference new automation (Small)

**Low Priority:**
1. cargo-watch → Skip (doesn't fit blocking execution model)

### Decision: Implement now before next iteration
```

---

**Remember:** Reflection without action is waste. Every action item must produce an artifact: a code change, a TODO.md entry, or an AGENTS.md update. If you finish reflecting and haven't changed any files, you skipped the most important part.
