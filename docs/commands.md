# Command Reference

Complete reference for all Zen commands.

## Environment Lifecycle

### `zen create <name>`
Create a new Python environment.

```bash
zen create myproject                    # Use system default Python
zen create myproject --python 3.10      # Specific Python version
zen create myproject --template ml-base # From a saved template
zen create myproject --ml --cuda 12.8   # With PyTorch + CUDA
```

### `zen add <path>`
Register an existing virtual environment. Accepts a venv root directory, `bin/python`, or `bin/activate`.

```bash
zen add /path/to/myenv              # Infer name from directory
zen add /path/to/myenv -n custom    # Override name
zen add /path/to/bin/python         # Resolve from python binary
```

### `zen rm <name>`
Remove an environment from disk and database.

```bash
zen rm myproject          # Asks for confirmation
zen rm myproject --yes    # Skip confirmation
zen rm myproject --cached # Remove from database only, keep files on disk
```

### `zen activate [name]`
Activate an environment in the current shell (requires [shell hook](installation.md#shell-integration)).

```bash
zen activate myproject    # Activate by name
zen activate              # Smart selection: uses project link or last activated
zen activate --last       # Re-activate most recently used environment
za myproject              # Shortcut
```

### `zen deactivate`
Deactivate the current environment.

```bash
zen deactivate
zd                        # Shortcut
```

## Package Management

Zen delegates all package operations to [uv](https://github.com/astral-sh/uv) for speed.

### `zen install [name] <packages>`
Install packages into an environment.

```bash
zen install numpy pandas                     # Into active environment
zen install myproject numpy pandas           # Into specific environment
zen install torch --index-url https://...    # Custom index (CUDA builds)
zen install ./my_package.whl                 # Local wheel
zen install --dry-run numpy                  # Preview without installing
```

### `zen uninstall [name] <packages>`
Remove packages from an environment.

```bash
zen uninstall numpy pandas
zen uninstall myproject numpy
```

### `zen run <name> <command>`
Run a command inside an environment without activating it.

```bash
zen run myproject python -c "import torch; print(torch.__version__)"
zen run myproject pip list
```

## Discovery

### `zen list` (alias: `zen ls`)
List all managed environments. Auto-detects terminal width and adjusts layout.

```bash
zen list                     # Auto-detect best format
zen ls                       # Same as zen list
zen list --format minimal    # Ultra-compact for narrow terminals
zen list --format compact    # Medium format, no paths
zen list --format wide       # Full table with all columns
zen list -1                  # Names only, one per line
zen list -l                  # Long format with paths
zen list --sort date         # Sort by creation date
```

### `zen info <name>`
Show detailed information about an environment: Python version, packages, labels, notes, linked projects.

```bash
zen info myproject
```

### `zen find <package>`
Find a package across all environments. Supports wildcards and CUDA-aware version matching.

```bash
zen find torch            # Exact match across all envs
zen find "*torch*"        # Wildcard search
zen find "torch==2.10"    # Version match (CUDA-aware: matches 2.10.0+cu130)
```

### `zen inspect <env> <package>`
Detailed info about a specific package — version, installer, source, editable status, install date.

```bash
zen inspect myproject torch
zen inspect myproject -l   # Long format: all packages with installer and date
```

### `zen diff <env1> <env2>`
Compare packages between two environments side by side.

```bash
zen diff dev production
```

## Health & Diagnostics

### `zen health [name]`
Check if an environment is healthy: Python binary, symlinks, CUDA consistency, dependency conflicts.

```bash
zen health myproject
zen health                # Check active environment
```

### `zen status`
System-wide dashboard: active environment, total environments, health summary.

## Project Linking

### `zen link add/rm/list`
Associate environments with project directories. Zen remembers these links and uses them for smart activation.

```bash
zen link add myproject              # Link to current directory
zen link add myproject --path /path/to/project
zen link list                       # Show all links
zen link rm myproject               # Remove a link
zen link prune                      # Clean up stale links
zen link reset --activations        # Reset activation counts
```

## Organization

### `zen label add/rm/list`
Tag environments with labels for organization.

```bash
zen label add myproject ml
zen label add myproject production
zen label list                      # List all labels across envs
zen label list --all                # Include all environments
```

### `zen note add/list/rm`
Attach notes to environments for context.

```bash
zen note add myproject "Uses custom CUDA 13.0 build"
zen note list myproject
zen note rm <uuid>
```

## Templates

Save and reuse environment configurations. Templates record packages, versions, index URLs, and wheels as ordered steps that replay when creating new environments.

### `zen template create <name>`
Start an interactive session to build a new template.

```bash
zen template create ml-base                    # Default Python
zen template create ml-base --python 3.12      # Specific Python version
```

This opens the **template REPL** — an interactive session where you add packages step by step:

```
[ml-base:latest] (0 steps, 0 pkgs) > add numpy scipy
[ml-base:latest] (1 step, 2 pkgs)  > add torch --index-url https://download.pytorch.org/whl/cu130
[ml-base:latest] (2 steps, 3 pkgs) > list
  Step 1: numpy, scipy
  Step 2: torch  [index: https://download.pytorch.org/whl/cu130]
[ml-base:latest] (2 steps, 3 pkgs) > save
  ✓ Template 'ml-base:latest' saved (2 steps, 3 packages)
```

**REPL commands:**
- `add <pkg> [pkg...]` — add packages to the current step
- `add <pkg> --index-url <url>` — add with a custom index (creates a new step)
- `add <pkg> --at N` / `--after N` / `--before N` — insert at a specific step
- `drop <pkg>` — remove a package by name
- `drop <N>` — remove an entire step by number
- `list` — show current template contents
- `save` — save and exit
- `quit` — abort without saving

> The REPL accepts `pip install`, `uv pip install`, and `zen install` syntax — prefixes are stripped automatically.

### `zen template edit <name>`
Edit an existing template. With no action, opens the interactive REPL pre-loaded with existing steps.

```bash
zen template edit ml-base                      # Interactive REPL

# One-shot mode (no REPL):
zen template edit ml-base add pandas           # Add a package
zen template edit ml-base add torch --step 2   # Add to specific step
zen template edit ml-base drop bitsandbytes    # Drop a package
zen template edit ml-base drop 3               # Drop step 3
```

### `zen template inspect <name>`
Docker-style layered view of a template's contents.

```bash
zen template inspect ml-base
zen template inspect ml-base:v2       # Specific version
```

### `zen template list`
List all saved templates with optional filters.

```bash
zen template list                     # All templates
zen template list --name ml           # Filter by name substring
zen template list --python 3.12       # Filter by Python version
zen template list --has-pkg torch     # Filter by package name
```

### `zen template rm <name>`
Remove a template.

```bash
zen template rm ml-base
```

### `zen template export <name>`
Export a template to a portable TOML file for sharing.

```bash
zen template export ml-base                # Writes ml-base.toml
zen template export ml-base -o custom.toml # Custom output path
```

### `zen template import <file>`
Import a template from a TOML file.

```bash
zen template import ml-base.toml
```

### Creating environments from templates

Use `--template` (or `--from`) with `zen create`:

```bash
zen create myenv --from ml-base            # Single template
zen create myenv --from ml-base,extras     # Multiple templates (comma-separated)
zen create myenv --from ml-base --strict   # Pin exact versions from snapshot
```

When combining multiple templates, Zen detects and warns about package version conflicts and index URL mismatches. The last template wins for any overlapping packages.

## Data Management

### `zen export [file]`
Export your environment registry to JSON.

```bash
zen export                # Print to stdout
zen export registry.json  # Save to file
```

### `zen import <file>`
Import an environment registry from JSON.

```bash
zen import registry.json
```

### `zen reset`
Reset the database to a fresh state. Environments on disk are preserved.

### `zen config <key> [value]`
Get or set configuration values.

```bash
zen config list            # Show all config
zen config display_format  # Get a value
zen config display_format compact  # Set a value
```

## Integration

### `zen hook <shell>`
Generate shell integration scripts. See [installation](installation.md#shell-integration).

### `zen completions <shell>`
Generate shell completion scripts. See [installation](installation.md#shell-completions).

### `zen mcp`
Start the MCP server for AI agent integration. See [MCP reference](mcp.md).

### `zen setup`
Interactive setup wizard for first-time configuration.
