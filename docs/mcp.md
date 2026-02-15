# MCP Server Reference

Zen includes a built-in [Model Context Protocol](https://modelcontextprotocol.io/) server, so AI coding assistants can manage your environments without asking you to run commands.

## Starting the Server

```bash
zen mcp
```

The server communicates over stdio using JSON-RPC. Configure your editor or agent to launch `zen mcp` as an MCP server.

### Example: VS Code / Cursor configuration

```json
{
  "mcpServers": {
    "zen": {
      "command": "zen",
      "args": ["mcp"]
    }
  }
}
```

## Available Tools

### Environment Management

| Tool | Description |
|------|-------------|
| `create_environment(name, python?)` | Create a new environment |
| `remove_environment(env_name)` | Remove an environment (database + disk) |
| `list_environments(label?)` | List all environments (optional label filter) |
| `get_environment_details(env_name)` | Full details: Python version, packages, labels, notes |
| `get_environment_health(env_name)` | Health check: Python binary, CUDA, dependencies |
| `compare_environments(env_names)` | Side-by-side package diff between environments |

### Package Management

| Tool | Description |
|------|-------------|
| `install_packages(env_name, packages, ...)` | Install packages (supports `index_url`, `extra_index_url`, `pre`, `upgrade`, `editable`) |
| `uninstall_packages(env_name, packages)` | Remove packages |
| `run_in_environment(env_name, command)` | Run a command inside an environment |

### Package Discovery

| Tool | Description |
|------|-------------|
| `search_packages(query)` | Find a package across all environments (substring match) |
| `find_package(query)` | Advanced: wildcards (`*torch*`), version matching (`torch==2.10`), CUDA-aware |
| `get_package_details(env_name, package)` | Full metadata: version, installer, source, editable, URL, commit |

### Project Linking

| Tool | Description |
|------|-------------|
| `get_default_environment(project_path)` | Get the default environment for a project |
| `get_project_environments(project_path)` | All environments linked to a project |
| `associate_project(project_path, env_name, tag?, is_default?)` | Link an environment to a project |

### Organization

| Tool | Description |
|------|-------------|
| `add_label(env_name, label)` | Add a label to an environment |
| `remove_label(env_name, label)` | Remove a label |
| `add_environment_note(env_name, note)` | Add a note to an environment |
| `get_environment_notes(env_name)` | Retrieve environment notes |

## How AI Agents Use Zen

A typical agent workflow:

1. **Discover**: `get_project_environments(cwd)` → find the right environment
2. **Inspect**: `get_environment_details(env)` → check what's installed
3. **Install**: `install_packages(env, ["torch", "numpy"])` → add dependencies
4. **Verify**: `run_in_environment(env, ["python", "-c", "import torch"])` → test
5. **Health**: `get_environment_health(env)` → make sure everything is consistent

The agent never needs shell access — everything works through structured tool calls.
