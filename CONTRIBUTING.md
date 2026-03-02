# Contributing to Zen

Thank you for your interest in contributing to Zen! This guide will help you get
started.

## Getting Started

```bash
git clone https://github.com/vistralis/zen.git
cd zen
cargo build --release
cargo test
```

**Requirements**: Rust stable (1.85+), Python 3.10+.

## Development Workflow

### Branch Naming

Use descriptive branch names following this convention:

| Pattern | When |
|---------|------|
| `dev-vX.Y.Z-feature` | Feature branches targeting a release |
| `fix/short-description` | Bug fixes |
| `docs/short-description` | Documentation changes |

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(#42): add zen clone command
fix: correct version comparison for post-release tags
chore: remove dead code
docs: update ROADMAP for v0.7.0
refactor: extract shared output types to output.rs
```

Format: `type(scope): description`

| Type | When |
|------|------|
| `feat` | New feature |
| `fix` | Bug fix |
| `chore` | Maintenance (deps, cleanup) |
| `docs` | Documentation only |
| `refactor` | Code restructuring (no behavior change) |
| `test` | Adding or fixing tests |

### Pull Request Process

1. **Fork** the repository or create a feature branch
2. **Implement** your changes with tests
3. **Run checks** locally before pushing:

```bash
cargo fmt --all -- --check    # Formatting
cargo clippy -- -D warnings   # Lints (must be zero warnings)
cargo test                     # All tests must pass
cargo build --release          # Release build must succeed
```

4. **Push** and open a PR against `main`
5. **CI** runs automatically (lint → test → release build)
6. **Review** — address feedback, keep commits clean
7. **Merge** — squash-merge preferred for feature branches

## Code Standards

### Formatting & Linting

- **`cargo fmt`** — all code must pass `cargo fmt --all -- --check`
- **`cargo clippy`** — zero warnings with `-D warnings`
- Both are enforced by CI

### SPDX Headers

Every `.rs` file must start with:

```rust
// SPDX-License-Identifier: Apache-2.0
```

### Documentation

- All `pub` functions and types must have `///` doc comments
- Module-level `//!` comments describe the module's purpose
- Non-obvious logic should have inline `//` comments

### Error Handling

- Use `Result<T, Box<dyn Error>>` for fallible functions
- MCP tools return structured JSON errors with `retriable` flags
- Never `unwrap()` on user-provided data

### Input Validation

- All environment names go through `EnvName::new()` (validates format)
- All user-facing names go through `validate_name()` (rejects path traversal, shell injection)
- File paths are never interpolated into shell commands

## Architecture

See [docs/architecture.md](docs/architecture.md) for the full system design.

**Key principles**:

- **Directory is truth** — disk state takes precedence over database
- **Newtypes for safety** — `EnvName`, `InstallOptions` prevent misuse
- **Shared output types** — `output.rs` is the single source of truth for CLI `--json` and MCP responses

**Module layout**:

```
src/
├── main.rs          # CLI entry point, clap argument parsing
├── lib.rs           # Library root (module declarations)
├── mcp.rs           # MCP server (11 tools: 6 standalone + 5 action-dispatch)
├── ops.rs           # Core operations layer
├── db.rs            # SQLite database
├── utils.rs         # Python/venv utilities
├── output.rs        # Shared response types (CLI + MCP)
├── types.rs         # Domain types (EnvName, HealthReport)
├── commands/        # CLI command implementations
│   ├── list.rs, info.rs, health.rs, ...
└── tests/           # Integration and CLI tests
```

## Testing

### Test Categories

| Suite | Command | What it tests |
|-------|---------|--------------|
| **Unit** | `cargo test --lib` | Types, parsing, validation |
| **Integration** | `cargo test --test integration_test` | Database, config, templates |
| **CLI** | `cargo test --test cli_test` | End-to-end CLI commands with real venvs |

### Expectations

- All existing tests must pass before submitting a PR
- New features should include tests
- CI runs all three suites on Ubuntu (x86_64 + ARM)

## Security

- **No hardcoded credentials** — CI checks for secrets in source
- **Parameterized SQL** — never use `format!()` with SQL
- **No shell injection** — all `Command::new` uses `.arg()`, never string interpolation
- **DB permissions** — database created with `0o600`
- **MCP is stdio-only** — no TCP/UDP listeners
- **Path redaction** — MCP responses show `~/…/name` instead of full paths

## Questions?

Open an issue at [github.com/vistralis/zen/issues](https://github.com/vistralis/zen/issues).
