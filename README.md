# Zen

---

**Peace of mind for Python environments.**

> *Less clutter, more clarity.*

One directory for all your Python environments. One CLI to keep track of them. Not another environment manager — just a lightweight cli that remembers what you'd rather not have to.

You already have enough to worry about. Which environment had CUDA 13? Did you set up that experiment with PyTorch nightly or stable? What was running in that project you haven't touched in two weeks?

Zen remembers — so you don't have to.

It's fast, lightweight, and not here to replace anything — just here to make juggling multiple Python environments painless, especially when GPUs and CUDA versions are involved.

It also ships with a built-in [MCP server](docs/mcp.md), so AI coding agents can manage your environments through structured tool calls.

> [!NOTE]
> A personal tool shared as-is. Vibecoded in Rust. No promises, no roadmap, no guarantees.

---

## Install

```bash
curl -sSf https://raw.githubusercontent.com/vistralis/zen/main/install.sh | sh
```

Or build from source:

```bash
git clone https://github.com/vistralis/zen.git && cd zen
cargo build --release
cp target/release/zen ~/.local/bin/
```

---

## Get Started

Set up your shell once:

```bash
# Add to .bashrc or .zshrc
eval "$(zen hook bash)"   # or zsh
```

Create an environment:

```bash
$ zen create ml_env
  ✓ Environment 'ml_env' created successfully.

$ zen activate ml_env     # alias: za ml_env
$ zen deactivate          # alias: zd
```

Install packages — Zen delegates to uv, so it's fast:

```bash
$ zen install ml_env numpy scipy opencv-python

# Need a specific CUDA build? Just pass the index:
$ zen install ml_env torch torchvision \
    --index-url https://download.pytorch.org/whl/cu130
```

See everything at a glance:

```
$ zen list
★ ml_env           3.12.3  ✓  torch:2.10.0+cu130  numpy:2.3.5
  vision           3.12.3  ✓  torch:2.9.1+cu128   numpy:2.2.6
  experiment       3.12.3  !  torch:2.11.0.dev     numpy:2.4.2
  diffusion        3.12.3  ✓  torch:2.10.0+cu130  numpy:2.3.5
  data_pipeline    3.12.3  ✓                       numpy:1.26.4

  5 environments  ✓ 4 ok  ! 1 drift  ★ 1 fav
```

Find a package across all environments:

```
$ zen find torch
  ml_env        torch  2.10.0+cu130
  vision        torch  2.9.1+cu128
  experiment    torch  2.11.0.dev20260129+cu126
  diffusion     torch  2.10.0+cu130
```

Check health — broken symlinks, CUDA conflicts, missing dependencies:

```
$ zen health ml_env
  ✓ Python binary          ok
  ✓ Symlink integrity      ok
  ✓ CUDA version           cu130 (consistent)
  ✓ Dependency conflicts   none detected
```

---

## Philosophy

### Let go of the clutter
You create an environment, install some packages, start a project. Then another. And another. Three weeks later you're staring at fifteen directories in `/envs/` wondering what half of them were for.

### Know where everything is
Every environment is tracked — Python version, packages, CUDA stack, health status, and which project it belongs to. All queryable. All instant.

### Designed for ML development
CUDA versions matter. `cu128` and `cu130` are not interchangeable. Zen tracks which build of PyTorch is in each environment, detects CUDA version mixing, and lets you search across environments with CUDA-aware version matching.

### Your projects, remembered
Link an environment to a project directory. Next time you `cd` into that project and type `zen activate` — it picks the right one. No arguments, no thinking.

```bash
$ zen link add ml_env
  ✓ Linked 'ml_env' to /home/user/projects/classifier

# Later, in that same directory:
$ zen activate
  ✓ Activated 'ml_env'
```

Supports multiple environments per project — main, test, experiment — with tags and defaults.

### Works with AI agents
Zen ships a built-in [MCP server](docs/mcp.md) so AI coding assistants can manage environments through structured tool calls — no shell access required.

---

## Commands

| Command | What it does |
|---------|-------------|
| `zen create <name>` | Create a new environment |
| `zen add <path>` | Register an existing virtual environment |
| `zen rm <name>` | Remove an environment (`--cached` for DB-only) |
| `zen activate [name]` | Activate (smart selection when no name given) |
| `zen deactivate` | Deactivate |
| `zen list` / `zen ls` | List all environments (`-1` names only, `-l` long) |
| `zen info <name>` | Detailed environment view |
| `zen install [env] <pkgs>` | Install packages |
| `zen uninstall [env] <pkgs>` | Remove packages |
| `zen run <env> <cmd>` | Run a command without activating |
| `zen find <pkg>` | Find a package across all environments |
| `zen inspect <env> <pkg>` | Detailed package info |
| `zen diff <env1> <env2>` | Compare two environments |
| `zen health [name]` | Environment health check |
| `zen link add/rm/list` | Project–environment links |
| `zen label add/rm/list` | Organize with labels |
| `zen note add/list/rm` | Attach notes |
| `zen rename <old> <new>` | Rename an environment |
| `zen template` | Create, edit, inspect, export/import templates |
| `zen mcp` | Start the MCP server |

Full reference → [docs/commands.md](docs/commands.md)

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ZEN_HOME` | `~/.local/share/zen/envs` | Where environments live |
| `ZEN_DOJO` | `~/.config/zen` | Database and config |

```bash
export ZEN_HOME=/localdisk/envs   # fast SSD for your environments
```

## How It Works

- **Standard venvs** — nothing proprietary, your environments are just Python venvs
- **SQLite** — one small database file tracks everything
- **uv** — all package operations delegate to uv for speed
- **Shell hook** — lightweight function for activate/deactivate in the current shell
- **MCP** — JSON-RPC over stdio for AI agent integration

## License

Apache-2.0
