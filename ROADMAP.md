# Zen — Roadmap to 1.0.0

> Release plan. Features are pulled from [FEATURES.md](./FEATURES.md) and assigned here.

## Color Palette

| Name | RGB | Usage |
|------|-----|-------|
| **Zen Blue** | `(100, 200, 255)` | ✓ pass, env names, source |
| **Peace Pink** | `(255, 182, 193)` | △ minor health |
| **Stressed Orange** | `(255, 140, 0)` | ! drift, numpy < 2.0 |
| **Serene Golden** | `(255, 215, 0)` | ★ favorites |
| **Lava Red** | `.red()` | ✗ broken |

---

## v0.6.3 — Types & Color Palette ✅

- ✅ `EnvName` newtype + validation (#27-28)
- ✅ Zen color palette + icon overhaul (#29-34)

## v0.6.4 — CLI Simplification & Safety ✅

- ✅ `link` subcommand restructure (#37)
- ✅ `label` subcommand restructure (#38)
- 🗑️ ~~Label-based icons in `zen list`~~ (#77) — reverted
- ✅ DB dead table removal + test cleanup (#78)
- ✅ `zen sync` removed — filesystem-as-truth (#6)
- ✅ `zen fav` removed — use `zen label add <env> favorite` (#18)

## v0.6.5 — Smart Activate & UX Polish ✅

- ✅ `zen activate` smart selection — no-arg context menu (#79)
- ✅ Activation history schema — link_type, timestamps, counts (#80)
- ✅ Rename "Source" → "Project" in `zen show` (#81)
- ✅ `zen install --dry-run` (#82)
- ✅ `zen config list` (#83)
- ✅ Activation stats in `zen link list` (#84)
- ✅ `zen link prune` — remove stale links (#85)
- ✅ `zen activate --last` — re-activate most recent (#86)
- ✅ `zd` deactivate shortcut (#87)
- ✅ `zen link reset` — fine-grained history reset (#88)
- ✅ `build.rs` alpha version stamping `x.x.x-<commit>` (#89)
- ✅ `zen create` guard checks — prevent overwrite/duplicates (#90)
- ✅ `zen inspect` install timestamps from `.dist-info` mtime (#91)
- ✅ `zen inspect -l` enhanced long format: name, version, installer, date (#92)
- ✅ Import name resolution from `top_level.txt` (#76)
- ✅ MCP env created date from `pyvenv.cfg` mtime (#93)

## v0.6.6 — Agent Lifecycle Completeness ✅

- ✅ `zen run <env> <cmd>` — run in env without activating (#94)
- ✅ `remove_environment` MCP tool (#95)
- ✅ `zen uninstall` CLI + MCP (#96)
- ✅ `compare_environments` deep diff (#97)
- ✅ Label filter in MCP `list_environments` (#98)
- ✅ Active env inference for info/inspect/health/link/label/note (#99)
- ✅ `zen link add --path <dir>` (#100)
- ✅ `build.rs` tag-aware versioning (#101)
- ✅ MCP install parity: index-url, wheel paths, pre, upgrade, editable (#102)
- ✅ Rename `comment` → `note`, env-only scoping (#103)
- ✅ `--all` flag for `note list` and `label list` (#104)

## v0.6.7 — Security Hardening & Branding ✅

- ✅ L1: DB file permissions `0600` (owner-only) (#105)
- ✅ Branding: "Peace of mind for Python environments" (#107)
- ✅ `.unwrap()` panic audit — replaced with safe error handling (#111)
- ✅ `zen install`/`uninstall` active env fallback via `$VIRTUAL_ENV` (#112)
- ✅ MCP `get_version` parity with CLI `--version` (#113)
- ✅ MCP `run_in_environment` 120s timeout (#114)
- ✅ Error messages suggest concrete commands (#115)
- ✅ Landing screen: `zen setup stack` → `zen setup stack-info` (#116)
- ✅ Dev flags (`--db-path`, `--home`) hidden from help (#117)

## v0.6.8 — Smart Activation & Portability Foundation ✅

- ✅ Bidirectional activation walk — downward subfolder scan + upward exact ancestors (#118)
- ✅ Umbrella dir blocking — children of `/` or `$HOME` excluded from walk (#119)
- ✅ `zen link reset --path [dir]` — wipe ALL links for a path (#120)
- ✅ Full paths in activation menu — replaces confusing `./`/`../` (#121)
- ✅ Ctrl+C cursor restore — `ctrlc` crate for cross-platform signal handling (#122)
- ✅ `ctrlc` crate added for Windows portability (#122)

## v0.6.10 — Musl Builds & CLI Polish ✅

- ✅ `zen add` — register existing virtual environments by path (#128)
- ✅ `zen rm --cached` — remove from registry only, keep files on disk (#129)
- ✅ `zen ls` alias for `zen list` (#130)
- ✅ `zen list -1` / `-l` output modes (#131)
- ✅ MCP `track_environment` / `untrack_environment` (#132)
- ✅ Musl build targets (x86_64 + aarch64) in CI (#133)
- ✅ `install.sh` smart glibc/musl detection (#134)
- ✅ Rustls-only — dropped OpenSSL dependency (#135)

## v0.6.12 — Template REPL & MCP Color Control ✅

- ✅ Interactive REPL for `zen template create/edit` (#140)
- ✅ `zen template export/import` (TOML) (#141)
- ✅ `zen template list` filters (`--name`, `--python`, `--has-pkg`) (#142)
- ✅ `zen rename <old> <new>` (#143)
- ✅ Smart name suggestion in `zen add` (#144)
- ✅ MCP `rename_environment` (#145)
- ✅ PID-based stale session auto-recovery (#146)
- ✅ Comma separator for multi-template `--from` (#147)
- ✅ Proper MCP color control via `ZenOps::new_plain()` — no ANSI in tool responses

## v0.7.0 — Next Release

- 📋 MCP API consolidation (27 → 10 tools) (#148) — action-dispatch pattern
- 📋 PEP 440 version comparator via `pep440_rs` (#149) — fix false-positive health conflicts
- 📋 JSON output for list/info (#39-40)
- 📋 `zen health --fix` (#41)
- 📋 SPDX headers on all `.rs` files (#108)
- 📋 Dead code removal: model tracking, insight logging (#109)
- 📋 L2: MCP path redaction — agents see `~/…/name` instead of full paths (#106)

## v0.8.0 — Lifecycle & Discovery

- 💡 `zen privacy` — configurable path recording rules with encrypted storage (#110)
- 💡 `zen clone` (#42)
- 💡 `zen freeze` (#43)
- 💡 `zen upgrade` with conflict detection (#44)
- 💡 `zen why` reverse dep tree (#45)
- 💡 `zen doctor` all-env scan (#47)

## v0.9.0 — Polish, Quality & Cross-Platform

- 📋 Database migrations framework (#52)
- 📋 95%+ test coverage (#55)
- 📋 Integration tests for all CLI commands (#56)
- 📋 Windows portability: `bin/` → `Scripts/` path abstraction (#123)
- 📋 Windows portability: `lib/pythonX.Y/site-packages` → `Lib/site-packages` (#124)
- 📋 Windows portability: PowerShell/CMD shell hooks (#125)
- 📋 Windows portability: `~/.config/zen` → `%APPDATA%\zen` via `dirs` crate (#126)
- 📋 Windows portability: file permissions conditional on `#[cfg(unix)]` (#127)
- 💡 Cross-platform testing — Windows CI (#57)
- 💡 Dynamic shell completion (#49)

## v1.0.0 — Stable Release

- 📋 Stable CLI interface guarantee (#63)
- 📋 Stable MCP interface (#64)
- 📋 Stable DB schema + migrations (#65)
- 📋 Prebuilt binaries x86 + arm (#66)
