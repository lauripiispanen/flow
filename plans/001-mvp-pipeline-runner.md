# Plan 001: MVP Pipeline Runner

**Status**: Not Started
**Priority**: P0
**Created**: 2026-02-14
**Target**: v0.1.0

## Goal

Build the minimum viable Flow pipeline runner that can:
1. Execute a single iteration of a hardcoded pipeline (Plan → Implement → Test)
2. Invoke Claude Code CLI for each step
3. Demonstrate the core loop functionality

## Non-Goals (Deferred)

- Configuration file support (hardcode pipeline for MVP)
- Multi-iteration loops (single iteration only)
- State persistence (stateless MVP)
- Error recovery (fail fast)
- Step cadence system (all steps run every iteration)

## Architecture

### Components

1. **CLI Interface** (`src/main.rs`)
   - Parse command-line arguments
   - Initialize pipeline
   - Trigger single iteration

2. **Pipeline** (`src/pipeline.rs`)
   - Define pipeline steps (Plan, Implement, Test)
   - Execute steps sequentially
   - Return results

3. **Claude Integration** (`src/claude.rs`)
   - Shell out to `claude-code` CLI
   - Construct prompts for each step
   - Capture stdout/stderr
   - Parse results

4. **Step Definitions** (`src/steps/`)
   - `plan.rs`: Generate implementation plan
   - `implement.rs`: Write code based on plan
   - `test.rs`: Run tests and validate

### Data Flow

```
User Input (task description)
    ↓
Pipeline::new(task)
    ↓
Step 1: Plan
    → ClaudeClient::execute("Create a plan for: {task}")
    → Capture plan output
    ↓
Step 2: Implement
    → ClaudeClient::execute("Implement this plan: {plan}")
    → Capture implementation results
    ↓
Step 3: Test
    → ClaudeClient::execute("Run tests for the implementation")
    → Capture test results
    ↓
Pipeline Result
    → Success/Failure
    → Output summary
```

## Implementation Plan

### Phase 1: TDD Setup (Red)
1. Write integration test: `tests/pipeline_test.rs`
   - Test: "pipeline executes plan-implement-test sequence"
   - Expected: Pipeline completes all three steps
   - Status: FAIL (not implemented)

2. Write unit tests for each component:
   - `tests/claude_test.rs`: Mock Claude CLI invocation
   - `tests/step_test.rs`: Test individual step logic

### Phase 2: Implementation (Green)
1. Implement `src/claude.rs`:
   - `ClaudeClient` struct
   - `execute(prompt: &str) -> Result<String>`
   - Shell command execution
   - Output capture

2. Implement `src/pipeline.rs`:
   - `Pipeline` struct
   - `Step` enum (Plan, Implement, Test)
   - `run() -> Result<PipelineResult>`

3. Implement `src/steps/`:
   - `plan.rs`: Plan step logic
   - `implement.rs`: Implement step logic
   - `test.rs`: Test step logic

4. Implement `src/main.rs`:
   - CLI argument parsing (clap)
   - Pipeline initialization
   - Result output

### Phase 3: Refactor
- Extract common patterns
- Improve error messages
- Add logging
- Documentation

## Acceptance Criteria

- [ ] `cargo test-all` passes
- [ ] `cargo clippy-all` passes with no warnings
- [ ] `cargo fmt-check` passes
- [ ] Can run: `flow --task "Add a hello world function"`
- [ ] Pipeline executes all three steps in sequence
- [ ] Claude Code is invoked for each step
- [ ] Results are captured and displayed
- [ ] Errors are handled gracefully

## Testing Strategy

### Unit Tests
- Claude CLI invocation (mocked)
- Pipeline step execution
- Error handling

### Integration Tests
- End-to-end pipeline execution (with real Claude Code)
- CLI argument parsing
- Output formatting

### Manual Testing
```bash
# Test 1: Simple task
cargo run -- --task "Create a function that adds two numbers"

# Test 2: More complex task
cargo run -- --task "Implement a binary search tree"

# Test 3: Error handling
cargo run -- --task "" # Empty task should error
```

## Dependencies

### Rust Crates
- `clap`: CLI argument parsing
- `tokio`: Async runtime (for future parallel execution)
- `anyhow`: Error handling
- `thiserror`: Custom error types
- `serde`, `serde_json`: Serialization (for future config)

### External Tools
- `claude-code` CLI must be installed and in PATH

## Open Questions

1. **Claude Code CLI invocation format**: What's the exact command syntax?
   - Research: Check `claude-code --help`
   - Decision needed before implementation

2. **Prompt engineering**: How should we structure prompts for each step?
   - Iterate during implementation
   - Document successful patterns

3. **Output parsing**: How do we capture and structure Claude's responses?
   - Start with raw stdout capture
   - Refine based on actual output format

## Success Metrics

- Single iteration completes in < 5 minutes
- All tests pass
- Code coverage > 80%
- Zero clippy warnings

## Next Steps After MVP

1. Multi-iteration support
2. Configuration file format (YAML)
3. State persistence
4. Step cadence system
5. Error recovery and retry logic
