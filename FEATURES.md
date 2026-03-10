# Zen — Feature Registry

> Flat inventory of all features, past and future. Each row is a self-contained feature with full metadata.
> This is the source of truth for what exists, what's planned, and what's just an idea.

## Legend

| Field | Values |
|-------|--------|
| **Priority** | 🔴 critical · 🟠 high · 🟡 medium · 🟢 low · ⚪ wishlist |
| **Risk** | 🔴 high (breaking/complex) · 🟡 medium · 🟢 low |
| **Effort** | low (hours) · mid (1-2 days) · high (3-5 days) · hardcore (week+) |
| **Status** | ✅ done · 🔧 wip · 📋 planned · 💡 idea |

---

## Features

| # | Feature | Area | Priority | Risk | Effort | Status | Suggested | Implemented | Replaces | Notes |
|---|---------|------|----------|------|--------|--------|-----------|-------------|----------|-------|
| 1 | `zen create` — create virtual env | core | 🔴 | 🟢 | mid | ✅ | 0.1.0 | 0.1.0 | — | Foundation |
| 2 | `zen list` — list all envs | core | 🔴 | 🟢 | mid | ✅ | 0.1.0 | 0.1.0 | — | |
| 3 | `zen rm` — remove env | core | 🔴 | 🟡 | low | ✅ | 0.1.0 | 0.1.0 | — | Deletes from disk + db |
| 4 | `zen install` — add packages | core | 🔴 | 🟢 | mid | ✅ | 0.1.0 | 0.1.0 | — | Wraps uv/pip |
| 5 | `zen info` / `zen show` | core | 🟠 | 🟢 | low | ✅ | 0.1.0 | 0.1.0 | — | |
| 6 | ~~`zen sync` / `zen scan`~~ | core | — | — | — | 🗑️ removed | 0.2.0 | 0.2.0 | — | Replaced by filesystem-as-truth; removed in 0.6.4 |
| 7 | `zen status` — system dashboard | core | 🟡 | 🟢 | mid | ✅ | 0.2.0 | 0.2.0 | — | |
| 8 | `zen activate` + shell hooks | shell | 🔴 | 🟡 | high | ✅ | 0.2.0 | 0.2.0 | — | `za` alias |
| 9 | `zen link` / `zen init` | project | 🟠 | 🟢 | mid | ✅ | 0.3.0 | 0.3.0 | — | Project-env binding |
| 10 | `zen unlink` | project | 🟡 | 🟢 | low | ✅ | 0.3.0 | 0.3.0 | — | |
| 11 | `zen links` | project | 🟡 | 🟢 | low | ✅ | 0.3.0 | 0.3.0 | — | |
| 12 | `zen export` / `zen import` | data | 🟡 | 🟡 | mid | ✅ | 0.3.0 | 0.3.0 | — | Portable JSON |
| 13 | `zen template` | core | 🟡 | 🟢 | mid | ✅ | 0.3.0 | 0.3.0 | — | Managed templates |
| 14 | `zen completions` | shell | 🟡 | 🟢 | low | ✅ | 0.3.0 | 0.3.0 | — | Static completions |
| 15 | `zen config` | core | 🟡 | 🟢 | low | ✅ | 0.4.0 | 0.4.0 | — | |
| 16 | `zen reset` | core | 🟡 | 🔴 | low | ✅ | 0.4.0 | 0.4.0 | — | Destructive reset |
| 17 | `zen note` (was `comment`) | meta | 🟢 | 🟢 | mid | ✅ | 0.4.0 | 0.4.0 | — | Env notes; renamed in 0.6.6 |
| 18 | ~~`zen fav`~~ → `zen label add <env> favorite` | meta | — | — | — | 🗑️ removed | 0.4.0 | 0.4.0 | — | Replaced by label system in 0.6.4 |
| 19 | `zen label add/rm/list` | meta | 🟢 | 🟢 | mid | ✅ | 0.4.0 | 0.4.0 | label/unlabel/labels | Consolidated in 0.6.4 |
| 20 | `zen find` — cross-env package search | discovery | 🟠 | 🟢 | mid | ✅ | 0.5.0 | 0.5.0 | — | Wildcard + CUDA-aware |
| 21 | `zen inspect` — pip show equivalent | discovery | 🟠 | 🟢 | low | ✅ | 0.5.0 | 0.5.0 | — | |
| 22 | `zen diff` — compare two envs | discovery | 🟡 | 🟢 | mid | ✅ | 0.5.0 | 0.5.0 | — | |
| 23 | `zen health` — env diagnostics | health | 🔴 | 🟡 | high | ✅ | 0.5.0 | 0.5.0 | — | Python, CUDA, deps |
| 24 | Adaptive CLI (Minimal/Compact/Wide) | output | 🟠 | 🟡 | high | ✅ | 0.5.2 | 0.5.2 | — | Terminal width detection |
| 25 | MCP server (`zen mcp`) | mcp | 🔴 | 🟡 | hardcore | ✅ | 0.5.0 | 0.5.0 | — | rmcp-based |
| 26 | `zen setup` — interactive wizards | core | 🟡 | 🟢 | mid | ✅ | 0.5.0 | 0.5.0 | — | |
| 27 | `EnvName` newtype + validation | types | 🔴 | 🟡 | mid | ✅ | 0.6.0 | 0.6.3 | raw `&str` | Typed boundary |
| 28 | `Diagnostic` trait + `HealthDiagnostic` | types | 🔴 | 🟡 | mid | ✅ | 0.6.0 | 0.6.3 | string-based | 11 typed variants |
| 29 | Zen color palette | output | 🟠 | 🟢 | low | ✅ | 0.6.3 | 0.6.3 | — | Blue/Pink/Orange/Gold/Red |
| 30 | Health icon overhaul (△ ! ★) | output | 🟠 | 🟢 | low | ✅ | 0.6.3 | 0.6.3 | `~` icon | Terminal-safe |
| 31 | `zen list` legend footer | output | 🟠 | 🟢 | low | ✅ | 0.6.3 | 0.6.3 | — | Health + fav counts |
| 32 | Color consistency pass (all commands) | output | 🟠 | 🟢 | mid | ✅ | 0.6.3 | 0.6.3 | — | info/health/inspect/list |
| 33 | Standardized report header UI | output | 🟡 | 🟢 | low | ✅ | 0.6.3 | 0.6.3 | — | Centered separator pattern |
| 34 | NumPy version coloring in list | output | 🟡 | 🟢 | low | ✅ | 0.6.3 | 0.6.3 | — | ≥2 blue, <2 orange |
| 35 | `Printer` enum (CLI vs MCP output) | output | 🟠 | 🟡 | mid | 📋 | 0.6.3 | — | — | Silent mode for MCP |
| 36 | `zen install --dry-run` | safety | 🔴 | 🟢 | mid | 📋 | 0.6.4 | — | — | Wraps `uv --dry-run` |
| 37 | `link` → `zen link [add\|rm\|list]` | structure | 🟠 | 🟡 | mid | ✅ | 0.7.0 | 0.6.4 | link/unlink/links | Done |
| 38 | `label` → `zen label [add\|rm\|list]` | structure | 🟠 | 🟡 | mid | ✅ | 0.7.0 | 0.6.4 | label/unlabel/labels | Done |
| 39 | `zen list --json` | output | 🟡 | 🟢 | low | 📋 | 0.7.0 | — | — | Machine-readable |
| 40 | `zen info --json` | output | 🟡 | 🟢 | low | 📋 | 0.7.0 | — | — | Scripting |
| 41 | `zen health --fix` | health | 🟠 | 🟡 | high | 📋 | 0.7.0 | — | — | Auto-resolve simple issues |
| 42 | `zen clone <env> <new>` | lifecycle | 🟡 | 🟡 | mid | 💡 | — | — | — | Duplicate env |
| 43 | `zen freeze <env>` | lifecycle | 🟡 | 🟢 | low | 💡 | — | — | — | → requirements.txt |
| 44 | `zen upgrade <env> <pkg>` | lifecycle | 🟡 | 🟡 | mid | 💡 | — | — | — | Conflict detection |
| 45 | `zen why <env> <pkg>` | discovery | 🟡 | 🟢 | high | 💡 | — | — | — | Reverse dep tree |
| 46 | `zen size <env>` | discovery | 🟢 | 🟢 | low | 💡 | — | — | — | Disk usage |
| 47 | `zen doctor` — all-env health scan | health | 🟡 | 🟢 | mid | 💡 | — | — | — | System-wide |
| 48 | Health history in DB | health | 🟢 | 🟡 | mid | 💡 | — | — | — | Track over time |
| 49 | Dynamic shell completion | shell | 🟡 | 🟢 | mid | 💡 | — | — | — | Tab-complete env names |
| 50 | Git hook integration | integration | 🟢 | 🟢 | mid | 💡 | — | — | — | Auto-link on clone |
| 51 | Env snapshots / rollback | lifecycle | 🟢 | 🔴 | hardcore | 💡 | — | — | — | Before/after install |
| 52 | DB migrations framework | infra | 🟠 | 🟡 | high | 📋 | 0.9.0 | — | — | Required for 1.0 |
| 53 | Lazy package scanning | perf | 🟡 | 🟡 | mid | 💡 | — | — | — | Scan on access |
| 54 | Parallel env scanning | perf | 🟢 | 🟡 | mid | 💡 | — | — | — | Rayon/tokio |
| 55 | 95%+ test coverage | quality | 🟠 | 🟢 | high | 📋 | 0.9.0 | — | — | Currently ~87% |
| 56 | Integration tests for all CLI cmds | quality | 🟠 | 🟢 | high | 📋 | 0.9.0 | — | — | |
| 57 | Cross-platform testing (ARM) | quality | 🟡 | 🟢 | mid | 💡 | — | — | — | Jetson CI |
| 58 | `zen help <topic>` built-in guides | docs | 🟢 | 🟢 | mid | 💡 | — | — | — | |
| 59 | Man page generation | docs | 🟢 | 🟢 | low | 💡 | — | — | — | clap-mangen |
| 60 | MCP stdout isolation | mcp | 🟠 | 🟡 | mid | 📋 | 0.6.4 | — | — | No stdout leaks |
| 61 | MCP tool schema improvements | mcp | 🟢 | 🟢 | low | 💡 | — | — | — | |
| 62 | MCP streaming for long ops | mcp | 🟢 | 🟡 | mid | 💡 | — | — | — | Progress notifications |
| 63 | Stable CLI interface guarantee | gate | 🔴 | 🟢 | low | 📋 | 1.0.0 | — | — | SemVer commitment |
| 64 | Stable MCP interface | gate | 🔴 | 🟢 | low | 📋 | 1.0.0 | — | — | |
| 65 | Stable DB schema + migrations | gate | 🔴 | 🟡 | mid | 📋 | 1.0.0 | — | — | |
| 66 | Prebuilt binaries (x86 + arm + musl) | release | 🟠 | 🟢 | mid | ✅ | 1.0.0 | 0.6.10 | — | GitHub Releases — 4 targets: x86_64-gnu, x86_64-musl, aarch64-gnu, aarch64-musl |
| 67 | Install script (`curl \| sh`) | release | 🟡 | 🟢 | low | ✅ | — | 0.6.10 | — | Smart glibc detection → musl fallback |
| 68 | `zen remote` — SSH env management | ⚪ | 🔴 | hardcore | 💡 | — | — | — | |
| 69 | `zen bench <env>` — benchmarks | ⚪ | 🟢 | high | 💡 | — | — | — | torch/numpy perf |
| 70 | `zen audit <env>` — vuln scanning | ⚪ | 🟢 | mid | 💡 | — | — | — | pip-audit |
| 71 | `zen share <env>` — export archive | ⚪ | 🟢 | mid | 💡 | — | — | — | |
| 72 | TUI dashboard (ratatui) | ⚪ | 🟡 | hardcore | 💡 | — | — | — | |
| 73 | Web UI — local dashboard | ⚪ | 🟡 | hardcore | 💡 | — | — | — | |
| 74 | NVIDIA package tracking in health/info | health | 🟡 | 🟡 | mid | 💡 | — | — | — | Show CUDA lib versions, detect cu12/cu13 mix |
| 75 | Research: NVIDIA core vs optional packages | research | 🟡 | 🟢 | low | 💡 | — | — | — | Which are torch deps vs standalone? See note below |
| 76 | Import name resolution (`top_level.txt`) | discovery | 🟠 | 🟢 | low | ✅ | 0.6.4 | 0.6.5 | — | Map pip name → Python import. See note below |
| 77 | ~~Label-based icons in `zen list`~~ | output | — | — | — | 🗑️ reverted | 0.6.4 | — | — | Implemented then reverted — user preferred ★-only |
| 78 | DB dead table removal | infra | 🟠 | 🟡 | mid | ✅ | 0.6.4 | 0.6.4 | — | Removed 6 tables, 8 dead functions, cleaned tests |
| 79 | `zen activate` smart selection (no-arg) | activation | 🔴 | 🟡 | mid | ✅ | 0.6.5 | 0.6.5 | — | Context-aware env selection from project hierarchy |
| 80 | Activation history schema | infra | 🟠 | 🟡 | mid | ✅ | 0.6.5 | 0.6.5 | — | `link_type`, `last_activated_at`, `activation_count` on `project_environments` |
| 81 | Rename "Source" → "Project" in `zen show` | output | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Confusing label — "Source" implies package origin |
| 82 | `zen install --dry-run` | safety | 🔴 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Wraps `uv pip install --dry-run` |
| 83 | `zen config list` | cli | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Display all active config key/values |
| 84 | Activation stats in `zen link list` | output | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Show count + last-activated per link |
| 85 | `zen link prune` | lifecycle | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Remove stale links (deleted envs + missing project dirs) |
| 86 | `zen activate --last` | activation | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Re-activate most recently used env globally |
| 87 | `zd` deactivate shortcut | shell | 🟢 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Mirrors `za` for activate |
| 88 | `zen link reset` — fine-grained history reset | lifecycle | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | `--activations`, `--history`, `--older-than <DAYS>` |
| 89 | `build.rs` alpha version stamping | infra | 🟠 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | `zen --version` shows `x.x.x-<commit>` at compile time |
| 90 | `zen create` guard checks | safety | 🔴 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Prevents overwrite of existing dirs and duplicate DB entries |
| 91 | `zen inspect` install timestamps | discovery | 🟠 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | Shows `Installed:` date from `.dist-info` mtime |
| 92 | `zen inspect -l` enhanced long format | output | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | 4-column view: name, version, installer, date |
| 93 | MCP env created date | mcp | 🟡 | 🟢 | low | ✅ | 0.6.5 | 0.6.5 | — | `Created:` from `pyvenv.cfg` mtime in `get_environment_details` |
| 94 | `zen run <env> <cmd>` | core | 🔴 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | — | Run command in env without activating; CLI + MCP `run_in_environment` |
| 95 | `remove_environment` MCP tool | mcp | 🟠 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | — | Wires existing `ops.remove_env()` to MCP |
| 96 | `zen uninstall` + MCP `uninstall_packages` | core | 🟠 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | — | Symmetric to install; uses `uv pip uninstall` |
| 97 | `compare_environments` deep diff | mcp | 🟠 | 🟡 | mid | ✅ | 0.6.6 | 0.6.6 | — | Shows version deltas + unique packages per env (was counts-only) |
| 98 | Label filter in MCP `list_environments` | mcp | 🟡 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | — | Optional `label` param for filtering |
| 99 | Active env inference for 8+ commands | cli | 🟠 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | — | `info/inspect/health/link/label/note` infer from `$VIRTUAL_ENV` |
| 100 | `zen link add --path <dir>` | cli | 🟡 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | — | Override project dir (default: cwd) |
| 101 | `build.rs` tag-aware versioning | infra | 🟡 | 🟢 | low | ✅ | 0.6.6 | 0.6.5 | — | Tagged → clean semver, dev → hash suffix |
| 102 | MCP install parity | mcp | 🔴 | 🟡 | mid | ✅ | 0.6.6 | 0.6.6 | — | `index_url`, `extra_index_url`, `pre`, `upgrade`, `editable`, wheel paths |
| 103 | Rename `comment` → `note` | cli | 🟡 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | `comment` | Simplified to env-only scoping; `comment` kept as hidden alias |
| 104 | `--all` flag for `note list` / `label list` | cli | 🟡 | 🟢 | low | ✅ | 0.6.6 | 0.6.6 | — | Cross-env listing without needing active env |
| 105 | DB file permissions `0600` | security | 🔴 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | Owner-only read/write on `zen.db` via `#[cfg(unix)]` |
| 106 | MCP path redaction | security | 🟠 | 🟡 | mid | 📋 | 0.6.7 | — | — | Agents see `~/…/name` not full paths |
| 107 | Branding tagline | meta | 🟢 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | "Peace of mind for Python environments" |
| 108 | SPDX headers on `.rs` files | quality | 🟢 | 🟢 | low | 📋 | 0.6.7 | — | — | Apache-2.0 license headers |
| 109 | Dead code removal | quality | 🟡 | 🟢 | mid | 📋 | 0.6.7 | — | — | Model tracking, insight logging leftovers |
| 110 | `zen privacy` configurable rules | security | 🟡 | 🟡 | high | 💡 | — | — | — | Encrypted storage, path recording rules |
| 111 | `.unwrap()` panic audit | safety | 🔴 | 🟡 | mid | ✅ | 0.6.7 | 0.6.7 | — | Replaced panicking unwraps with safe error handling |
| 112 | `install`/`uninstall` active env fallback | cli | 🟠 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | Infer env from `$VIRTUAL_ENV` when not specified |
| 113 | MCP `get_version` parity | mcp | 🟡 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | Returns same string as `zen --version` |
| 114 | MCP `run_in_environment` timeout | mcp | 🟡 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | 120s timeout prevents hangs |
| 115 | Error messages suggest commands | ux | 🟡 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | e.g. "Use: zen link add \<env\>" |
| 116 | Landing screen fix | ux | 🟢 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | `zen setup stack` → `zen setup stack-info` |
| 117 | Dev flags hidden from help | ux | 🟢 | 🟢 | low | ✅ | 0.6.7 | 0.6.7 | — | `--db-path`, `--home` not shown in `zen --help` |
| 118 | Bidirectional activation walk | activation | 🔴 | 🟡 | mid | ✅ | 0.6.8 | 0.6.8 | — | Downward subfolder (≤2) + upward ancestor (≤2) scan |
| 119 | Umbrella dir blocking | activation | 🟠 | 🟢 | low | ✅ | 0.6.8 | 0.6.8 | — | Children of `/` or `$HOME` excluded from ancestor walk |
| 120 | `zen link reset --path [dir]` | lifecycle | 🟡 | 🟢 | low | ✅ | 0.6.8 | 0.6.8 | — | Wipe ALL links for a project path |
| 121 | Full paths in activation menu | ux | 🟡 | 🟢 | low | ✅ | 0.6.8 | 0.6.8 | — | Replaces confusing `./`/`../` with absolute paths |
| 122 | Ctrl+C cursor restore | ux | 🟠 | 🟢 | low | ✅ | 0.6.8 | 0.6.8 | — | `ctrlc` crate — cross-platform signal handling |
| 123 | Windows: `bin/` → `Scripts/` | portability | 🟠 | 🟡 | mid | 📋 | 0.9.0 | — | — | Python venv uses `Scripts/` on Windows |
| 124 | Windows: site-packages path | portability | 🟠 | 🟡 | mid | 📋 | 0.9.0 | — | — | `Lib/site-packages` instead of `lib/pythonX.Y/site-packages` |
| 125 | Windows: PowerShell/CMD hooks | portability | 🟠 | 🟡 | high | 📋 | 0.9.0 | — | — | Shell hooks currently bash/fish only |
| 126 | Windows: config dir via `dirs` crate | portability | 🟠 | 🟢 | mid | 📋 | 0.9.0 | — | — | `~/.config/zen` → `%APPDATA%\zen` |
| 127 | Windows: conditional file permissions | portability | 🟡 | 🟢 | low | 📋 | 0.9.0 | — | — | `#[cfg(unix)]` guards already partial |
| 128 | `zen add` — track existing env | core | 🟠 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | — | Accepts venv root, bin/python, or bin/activate |
| 129 | `zen rm --cached` — untrack env | core | 🟠 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | — | Remove from DB only, keep files on disk |
| 130 | `zen ls` alias | cli | 🟢 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | — | Alias for `zen list` |
| 131 | `zen list -1` single-column output | output | 🟢 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | — | Names only, one per line |
| 132 | `zen list -l` long format | output | 🟢 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | — | Force wide layout |
| 133 | MCP `track_environment` | mcp | 🟠 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | `add_environment` | Register existing venv by path |
| 134 | MCP `untrack_environment` | mcp | 🟠 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | — | Remove from registry, keep files |
| 135 | MCP `run_in_environment` cwd support | mcp | 🟡 | 🟢 | low | ✅ | 0.6.10 | 0.6.10 | — | Optional working directory param |
| 136 | Activity log (`zen log`) | core | 🟠 | 🟡 | mid | ✅ | 0.6.10 | 0.6.10 | — | Tracks create/remove/install/uninstall events |
| 137 | Rustls-only TLS (OpenSSL dropped) | infra | 🟠 | 🟡 | low | ✅ | 0.6.10 | 0.6.10 | native-tls | Enables musl static builds; `reqwest` default-features=false |
| 138 | Musl static builds in CI | release | 🟠 | 🟢 | mid | ✅ | 0.6.10 | 0.6.10 | — | Jetson (glibc 2.35) + Raspi (glibc 2.36) support |
| 139 | Smart installer glibc detection | release | 🟠 | 🟢 | mid | ✅ | 0.6.10 | 0.6.10 | — | Auto-selects musl binary if glibc < 2.39 |
| 140 | `zen template create/edit` interactive REPL | core | 🟠 | 🟡 | high | ✅ | 0.7.0 | 0.6.12 | — | Step-by-step builder with live summary, `--step N`, subcommand help |
| 141 | `zen template export/import` (TOML) | data | 🟡 | 🟢 | mid | ✅ | 0.7.0 | 0.6.12 | — | Portable TOML format for template sharing |
| 142 | `zen template list` filters | cli | 🟡 | 🟢 | low | ✅ | 0.7.0 | 0.6.12 | — | `--name`, `--python`, `--has-pkg` filter flags |
| 143 | `zen rename <old> <new>` | core | 🟠 | 🟢 | low | ✅ | 0.7.0 | 0.6.12 | — | Rename environment in DB; validates uniqueness |
| 144 | Smart name suggestion in `zen add` | ux | 🟡 | 🟢 | mid | ✅ | 0.7.0 | 0.6.12 | — | Path-walking heuristic for generic venv names (`.venv` → `project-name`) |
| 145 | MCP `rename_environment` | mcp | 🟡 | 🟢 | low | ✅ | 0.7.0 | 0.6.12 | — | Remote rename via MCP; validates existence + uniqueness |
| 146 | PID-based stale session auto-recovery | safety | 🔴 | 🟡 | mid | ✅ | 0.6.12 | 0.6.12 | — | `active_sessions` stores PID; `clear_stale_session()` checks `/proc/<pid>` liveness — dead sessions auto-clear instead of blocking |
| 147 | Comma separator for multi-template `--from` | ux | 🟡 | 🟢 | low | ✅ | 0.6.12 | 0.6.12 | — | `--from a,b` works without quoting (pipe `\|` still supported); dedup prevents double-apply |
| 148 | MCP API consolidation (23 → 11 tools) | mcp | 🔴 | 🔴 | high | ✅ | 0.7.0 | 0.7.0 | — | Action-dispatch: `manage_environment`, `inspect_environment`, `manage_packages`, `find_package`, `manage_project`, `manage_metadata`. Breaking change. |
| 149 | PEP 440 version comparator (`pep440_rs`) | health | 🟠 | 🟢 | low | ✅ | 0.7.0 | 0.7.0 | `compare_versions` | Replace handrolled version comparison with `astral-sh/pep440_rs`. Fixes false-positive conflicts for post/dev/pre-release versions. |
| 150 | MCP structured JSON responses | mcp | 🟠 | 🟡 | mid | ✅ | 0.7.0 | 0.7.0 | — | All MCP tools return structured JSON instead of prose. Enables programmatic agent reasoning. Consistent error format with `ZenError` codes + retriable flag. |
| 152 | Shared output types (`output.rs`) | arch | 🟡 | 🟢 | low | ✅ | 0.7.0 | 0.7.0 | — | 14 response structs shared between MCP and CLI `--json`. Single source of truth for serialization. |
| 153 | CLI `--json` for list/info | cli | 🟠 | 🟢 | low | ✅ | 0.7.0 | 0.7.0 | — | `zen list --json`, `zen info --json`. Machine-readable output using shared types. |
| 154 | `zen health --fix` | health | 🟠 | 🟡 | mid | ✅ | 0.7.0 | 0.7.0 | — | Auto-installs missing dependencies. Per-package error handling. Only Missing deps are fixable. |
| 155 | Dead code sweep | infra | 🟡 | 🟢 | low | ✅ | 0.7.0 | 0.7.0 | — | Removed `get_system_summary`, `new_table_with_headers`. Cleaned `#[allow(dead_code)]` from `run_in_env_capture`. |
| 151 | MCP install/uninstall split | mcp | 🟡 | 🟢 | low | ✅ | 0.7.0 | 0.7.0 | — | Split `manage_packages` into `install_packages` + `uninstall_packages`. Install has 6 params that uninstall ignores. Goes to 11 tools. |
| 156 | `run_in_environment` pipe deadlock prevention | mcp | 🟠 | 🟡 | mid | 📋 | — | — | — | Timeout loop polls `try_wait` without draining stdout/stderr — chatty commands can fill OS pipe buffer and deadlock. Fix: concurrent reader threads. Flagged by Copilot; deferred because it only affects extremely verbose commands and requires refactoring the spawn path. |
| 157 | `log_path` write restriction | security | 🟡 | 🟡 | mid | 📋 | — | — | — | `log_path` in `run_in_environment` writes to arbitrary paths. Fix: restrict to `$ZEN_HOME/logs/` or validate path is relative without `..`. Flagged by Copilot; deferred because MCP runs locally via stdio (not network-exposed), so attack surface is minimal. |
| 158 | Rename rollback on DB failure | safety | 🟡 | 🟢 | low | 📋 | — | — | — | After directory rename, DB update can fail leaving inconsistent state. Fix: rollback directory rename if DB update fails. Flagged by Copilot; deferred — DB failures are extremely rare with SQLite bundled mode. |
| 159 | `HealthCheck.check` stable identifiers | mcp | 🟡 | 🟢 | low | 📋 | — | — | — | `check` and `message` fields are identical in health JSON. Fix: use stable IDs (`python`, `dependencies`, `cuda`) for `check` field. Flagged by Copilot; deferred as UX polish — no consumer relies on stable IDs yet. |
| 160 | `CompareEnvironmentsParams` validation | mcp | 🟡 | 🟢 | low | 📋 | — | — | — | `env_names` is `Vec<String>` — no validation at MCP boundary. Fix: use `Vec<EnvName>` or validate each entry. Deferred — requires `JsonSchema` impl for `EnvName` type. |
| 161 | `list_environments` mutex scope | mcp | 🟡 | 🟢 | low | 📋 | — | — | — | Filesystem I/O and env registration happen while holding the DB mutex. Fix: scan filesystem first, then lock DB only for updates. Flagged by Copilot; deferred — no contention in single-client stdio transport. |
| 162 | CLI tests for `--json` and `--fix` | testing | 🟡 | 🟢 | low | 📋 | — | — | — | `zen list --json`, `zen info --json`, and `zen health --fix` lacking CLI test coverage. Flagged by Copilot; deferred as nice-to-have test coverage improvement. |
| 163 | Tracked-vs-managed path detection | cli | 🟡 | 🟡 | mid | 📋 | — | — | — | `is_tracked` uses string `starts_with` on paths, which can misclassify envs whose names share a common prefix with `ZEN_HOME`. Fix: use canonical path comparison. Flagged by Copilot; deferred — false positives only with overlapping directory names (unlikely in practice). |
| 164 | `cargo audit` blocking in CI | infra | 🟡 | 🟢 | low | 📋 | — | — | — | CI security audit step is non-blocking (`|| echo`). Flagged by Copilot; intentionally non-blocking — advisories ≠ vulnerabilities, and failing CI on unmaintained-but-safe deps would block releases. |
| 165 | `zen protect <name>` | safety | 🟠 | 🟢 | low | ✅ | 0.7.1 | 0.7.1 | — | Mark environment as protected (🔒) |
| 166 | `zen unprotect <name>` | safety | 🟠 | 🟢 | low | ✅ | 0.7.1 | 0.7.1 | — | Remove protection from environment |
| 167 | `zen rm` protected-env enforcement | safety | 🔴 | 🟢 | low | ✅ | 0.7.1 | 0.7.1 | — | Refuses removal of protected envs unless `--force` is used |
| 168 | `is_protected` DB column | infra | 🟠 | 🟢 | low | ✅ | 0.7.1 | 0.7.1 | — | Schema v5; auto-migrated from v4 via `ALTER TABLE` |
| 169 | 🔒 indicator in `zen list` / `zen info` | output | 🟡 | 🟢 | low | ✅ | 0.7.1 | 0.7.1 | — | Visual protection indicator in CLI and `--json` |
| 170 | `is_protected` in MCP environment details | mcp | 🟡 | 🟢 | low | ✅ | 0.7.1 | 0.7.1 | — | No enforcement — agents trusted |
| 171 | Shell hook v4 — name+path protocol | shell | 🟠 | 🟡 | low | ✅ | 0.7.1 | 0.7.1 | v3 hooks | `--path-only` outputs name then path; hook reads both — fixes `za` display name bug |
| 172 | Fix stale `zen scan` in reset message | ux | 🟢 | 🟢 | low | ✅ | 0.7.1 | 0.7.1 | — | Now says "zen list" instead of removed command |

---

## Research Notes

### #75 — NVIDIA Package Ecosystem

Known NVIDIA pip packages (observed in production envs):

**Core CUDA libs** (likely torch deps):
```
nvidia-cublas, nvidia-cuda-cupti, nvidia-cuda-nvrtc, nvidia-cuda-runtime,
nvidia-cudnn, nvidia-cufft, nvidia-curand, nvidia-cusolver, nvidia-cusparse,
nvidia-nvjitlink, nvidia-nvtx, nvidia-nccl
```

**Specialized** (may be standalone installs):
```
nvidia-cufile, nvidia-cusparselt, nvidia-nvshmem, nvidia-ml-py, nvidia-modelopt
```

**Generations**: packages come in `-cu12` and `-cu13` (or unversioned = latest).

**Open questions**:
- Which are hard deps of `torch` vs independently installed?
- Can cu12 and cu13 variants coexist safely? (probably not)
- Should `zen health` flag cu12/cu13 mixing as a conflict?
- Should `zen info` show a "CUDA libs" summary line?

### #76 — Import Name Resolution

**Problem**: pip package names often differ from Python import names. MCP agents (and users)
try `import nvidia.modelopt` when the real import is `import modelopt`. This causes
false "module not found" errors even though the package is installed.

**Known offenders**:
```
opencv-python       → cv2
Pillow              → PIL
scikit-learn        → sklearn
nvidia-modelopt     → modelopt
python-dateutil     → dateutil
beautifulsoup4      → bs4
pyyaml              → yaml
```

**Solution**: Read `top_level.txt` from each package's `.dist-info/` directory (already scanned).
This file lists the actual importable top-level modules.

**Implementation** (v0.6.4 — internal testing):
1. In `utils.rs`: add `read_top_level(dist_info_path)` — reads `top_level.txt` on demand (no storage)
2. MCP `get_package_details`: call it at query time, return `import_name` field
3. `zen inspect`: **hidden for now** — validate via MCP first
4. Future: surface in CLI once validated

**Effort**: S — the scan engine already walks `.dist-info/` dirs, just read one more file.
**Risk**: 🟢 low — additive, no breaking changes.
**Target**: v0.6.4 (next patch)
