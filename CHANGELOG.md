# Changelog

All notable changes to Zen are documented here.

---

## v0.7.1

### Protected Environments

Environments can now be marked as protected, preventing accidental removal.

**New commands:**
- `zen protect <name>` — mark an environment as protected (🔒)
- `zen unprotect <name>` — remove protection from an environment
- `zen rm` now refuses to remove protected environments unless `--force` is used

**Visibility:**
- 🔒 indicator appears in `zen list` and `zen info` for protected environments
- `is_protected` field included in MCP environment details and JSON output

### Shell Hook Improvements

- Shell hooks upgraded to v4 — `za` menu selection now shows correct zen alias name instead of directory basename
- Activation message now shows environment path: `✓ Activated environment: name (/path)`
- Fixed `zen reset` message referencing removed `zen scan` command

### Security

- Remove `eval` in fish hook — use direct command substitution
- Sanitize `display_name` for PS1 via `tr` allow-list
- `is_protected` propagates DB errors instead of silent fallback

### Internal

- DB schema v5 (adds `is_protected` column with automatic migration from v4)
- 109 tests passing (42 unit × 2 + 12 CLI + 13 integration)

---

## v0.7.0

### MCP Architecture Overhaul

The MCP server was completely redesigned around an action-dispatch pattern, reducing 23 individual tools to 11 consolidated tools. Each tool now accepts an `action` parameter and returns structured JSON instead of human-readable prose.

**MCP changes:**
- 23 → 11 tools via action-dispatch consolidation (#148)
- Structured JSON responses for all tools (#150)
- Install and uninstall split into separate tools (#151)
- Path redaction — agents see `~/…/name` instead of full paths (#106)
- `run_in_environment` log_path — full output capture to file

**Code quality:**
- SPDX license headers on all `.rs` source files (#108)
- Dead code removal: model tracking and insight logging subsystems (#109)
- PEP 440 version comparator via `pep440_rs` for accurate health checks (#149)
- JSON output mode for `zen list` and `zen info` (#39, #40)
- `zen health --fix` — automatic resolution of fixable issues (#41)

### Internal

- All MCP tools use `ZenOps::new_plain()` — no ANSI in structured responses
- 94 tests passing
- Clippy clean under `-D warnings`

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
