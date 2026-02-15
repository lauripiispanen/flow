# Flow

> An automated coding pipeline runner that orchestrates structured code production using Claude Code

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Overview

Flow is a Rust-based orchestrator that enables automated, iterative software development by coordinating Claude Code CLI invocations. Each iteration executes a configurable pipeline (e.g., Plan → Implement → Test) in a fresh context, allowing for focused, incremental progress.

### Key Features

- **Configurable Pipelines**: Define custom sequences of steps tailored to your project
- **Variable Cadence**: Run different steps at different frequencies (e.g., refactor every 5 iterations)
- **Context Reset**: Each iteration starts fresh via new Claude Code CLI invocation
- **Autonomous Operation**: Agent-driven task selection from TODO.md and plans/*.md
- **TDD Native**: Built with test-driven development as a first-class workflow

## Status

🚧 **Under Active Development** - Currently implementing MVP

## Quick Start

### Prerequisites

- Rust 1.70+ (2021 edition)
- Claude Code CLI installed and in PATH

### Installation

```bash
git clone https://github.com/yourusername/flow.git
cd flow
cargo build --release
```

### Usage

```bash
# Run a single iteration on a task
cargo run -- --task "Create a function that adds two numbers"

# Run multiple iterations (future)
cargo run -- --iterations 20

# Execute a specific plan (future)
cargo run -- --plan plans/001-mvp.md
```

## Development

### Setup

```bash
# Install dependencies
cargo build

# Run tests
cargo test-all

# Run linter
cargo clippy-all

# Format code
cargo fmt
```

### Workflow

This project follows strict TDD (Test-Driven Development):

1. **Red**: Write failing tests first
2. **Green**: Implement minimum code to pass tests
3. **Refactor**: Clean up while keeping tests green

### Project Structure

```
flow/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Public library interface
│   ├── pipeline.rs      # Pipeline orchestration
│   ├── claude.rs        # Claude Code CLI integration
│   └── steps/           # Individual step implementations
├── tests/               # Integration tests
├── plans/               # Detailed implementation plans
├── TODO.md              # Task queue
└── AGENTS.md            # Agent context (Vercel pattern)
```

## Architecture

Flow operates on a simple model:

1. **Agent** analyzes TODO.md and plans/*.md to select next task
2. **Pipeline** executes configured sequence of steps
3. **Steps** invoke Claude Code CLI with appropriate prompts
4. **Results** are captured and aggregated
5. Process repeats for next iteration

See [AGENTS.md](./AGENTS.md) for detailed architecture documentation.

## Contributing

This project is currently in early development. Contributions welcome once MVP is stable!

## License

MIT License - see [LICENSE](LICENSE) file for details

## Inspiration

- [Vercel's AGENTS.md pattern](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals)
- Test-Driven Development methodology
- Agentic coding workflows

## Roadmap

- [x] Project structure and TDD setup
- [ ] MVP: Core pipeline runner (in progress)
- [ ] Configuration file support (YAML/TOML)
- [ ] Multi-iteration loops
- [ ] State persistence
- [ ] Step cadence system
- [ ] Error recovery and retry logic
- [ ] Observability and logging
