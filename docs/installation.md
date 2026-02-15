# Installation

## Quick Install (coming soon)

```bash
curl -sSL https://zen.dev/install | sh
```

## From GitHub

```bash
# Install directly from the repository
cargo install --git https://github.com/vistralis/zen.git
```

## Build from Source

Requirements: Rust toolchain (1.85+)

```bash
git clone https://github.com/vistralis/zen.git
cd zen
cargo build --release
cp target/release/zen ~/.local/bin/
```

Make sure `~/.local/bin` is in your `PATH`.

## Shell Integration

Add this to your `~/.bashrc` or `~/.zshrc`:

```bash
eval "$(zen hook bash)"   # for bash
eval "$(zen hook zsh)"    # for zsh
```

This creates:
- `zen activate <name>` — activate an environment in the current shell
- `zen deactivate` — deactivate the current environment
- `za <name>` — shortcut for `zen activate`
- `zd` — shortcut for `zen deactivate`

## Shell Completions

```bash
# Generate completion scripts
zen completions bash > ~/.local/share/bash-completion/completions/zen
zen completions zsh > ~/.zfunc/_zen
```

## Verify Installation

```bash
zen --version
zen status
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ZEN_HOME` | `~/.local/share/zen/envs` | Where environments are stored |
| `ZEN_DOJO` | `~/.config/zen` | Database and configuration directory |

```bash
# Example: use a fast local disk for environments
export ZEN_HOME=/localdisk/envs
export ZEN_DOJO=/localdisk/.zen
```

## Cross-Platform

Zen is tested on:
- Ubuntu 24.04 (x86_64)
- Ubuntu 22.04 (x86_64)
- Ubuntu 24.04 (aarch64 / ARM64)

Works anywhere Rust compiles and Python venvs are supported.
