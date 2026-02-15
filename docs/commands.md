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

### `zen rm <name>`
Remove an environment from disk and database.

```bash
zen rm myproject          # Asks for confirmation
zen rm myproject --yes    # Skip confirmation
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

### `zen list`
List all managed environments. Auto-detects terminal width and adjusts layout.

```bash
zen list                  # Auto-detect best format
zen list -f minimal       # Ultra-compact for narrow terminals
zen list -f compact       # Medium format, no paths
zen list -f wide          # Full table with all columns
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

Save and reuse environment configurations.

```bash
zen template create ml-base --python 3.12   # Start recording
# Install packages...
zen template exit                           # Save the template

zen template list                           # View saved templates
zen create newproject --template ml-base    # Create from template
zen template rm ml-base                     # Remove a template
```

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
