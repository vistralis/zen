# Changelog

All notable changes to Zen are documented here.

---

## v0.6.12

### Templates — Interactive REPL & Portability

The template system now supports a full interactive REPL for building templates step by step, TOML export/import for sharing, and multi-template composition.

**New commands:**
- `zen template create <name>` — interactive REPL session: add packages, set index URLs, include wheels, organize into steps
- `zen template edit <name>` — interactive editing with the same REPL, or one-shot `add`/`drop` subcommands
- `zen template inspect <name>` — Docker-style layered view of template contents
- `zen template export <name>` — export to portable TOML file
- `zen template import <file>` — import from TOML file
- `zen template list` — now supports `--name`, `--python`, `--has-pkg` filters
- `zen template drop <target>` — remove a package or step from the active session

**REPL features:**
- `add <pkg> [pkg...]` — add packages to the current step
- `add <pkg> --index-url <url>` — add with a custom PyPI index (creates a new step)
- `add <pkg> --at N` / `--after N` / `--before N` — insert at a specific step position
- `drop <pkg|N>` — remove a package by name or a step by number
- `list` — show current template contents
- `save` — save and exit
- `quit` — abort without saving
- Accepts `pip install`, `uv pip install`, `zen install` syntax — prefixes are stripped automatically
- Live status bar shows step count and total packages

**Multi-template composition:**
- `zen create myenv --from tpl1,tpl2` — apply multiple templates (comma-separated)
- Pipe `|` separator still supported for backwards compatibility
- Duplicate templates are automatically deduplicated
- Conflict detection: warns when templates override the same package with different versions
- Index URL conflicts are flagged separately

### Environment Management

- `zen rename <old> <new>` — rename an environment in the database
- `zen add` — improved smart name suggestion for generic venv names (`.venv` → `project-name`)
- Stale REPL session auto-recovery: sessions store PID and auto-clear if the process has died

### MCP Server

- `rename_environment` tool added
- All 27 tools verified compatible with Antigravity IDE

### Internal

- REPL extracted into dedicated `repl.rs` module with pure parsing (fully testable)
- 36 REPL parser unit tests
- Clippy clean under `-D warnings`
- 72 total tests passing

---

## v0.6.8

- Bidirectional activation walk (subfolder + ancestor scan)
- Umbrella directory blocking (`/`, `$HOME`)
- `zen link reset --path` — wipe all links for a project
- Full paths in activation menu
- Ctrl+C cursor restore
- Template session-only storage with conflict detection
- Activity log (`zen log`)

## v0.6.7

- DB file permissions `0o600`
- `.unwrap()` panic audit — safe error handling
- `install`/`uninstall` active env fallback from `$VIRTUAL_ENV`
- MCP `get_version`, `run_in_environment` timeout (120s)
- Error messages suggest commands

## v0.6.5

- Smart activation (`zen activate` with no args)
- Activation history tracking
- `zen install --dry-run`
- `zen config list`
- `zen activate --last`
- `zd` deactivate shortcut
- `build.rs` alpha version stamping

## v0.6.4

- Import name resolution (`top_level.txt`)
- Dead table removal (6 tables, 8 functions)
- `zen label add/rm/list` consolidated
- `zen link add/rm/list` consolidated

## v0.6.3

- `EnvName` newtype with validation
- `Diagnostic` trait with 11 typed variants
- Zen color palette (blue/pink/orange/gold/red)
- Health icon overhaul (△ ! ★)
- `zen list` legend footer
