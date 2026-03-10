// SPDX-License-Identifier: Apache-2.0

//! Model Context Protocol (MCP) server for Zen.
//!
//! This module implements an MCP server using the official rmcp SDK,
//! allowing Zen to interface with AI agents (like Antigravity or Claude Desktop).
//!
//! ## Tool Surface (v0.7.0)
//!
//! 11 tools total: 6 standalone + 5 action-dispatch.
//! All tools return structured JSON for programmatic agent reasoning.
//!
//! | Tool | Type | Actions |
//! |------|------|---------|
//! | `get_version` | standalone | — |
//! | `list_environments` | standalone | label filter |
//! | `run_in_environment` | standalone | — |
//! | `compare_environments` | standalone | — |
//! | `install_packages` | standalone | install with full options |
//! | `uninstall_packages` | standalone | simple package removal |
//! | `manage_environment` | dispatch | create, remove, rename, track, untrack |
//! | `inspect_environment` | dispatch | details, health |
//! | `find_package` | inferred | env_name present → details mode |
//! | `manage_project` | dispatch | link, get_default, list |
//! | `manage_metadata` | dispatch | add_note, get_notes, add_label, remove_label |

use crate::db::Database;
use crate::types::{Diagnostic, EnvName};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_router,
    transport::stdio,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Redacts a filesystem path for MCP responses.
fn redact_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| format!("~/…/{}", name.to_string_lossy()))
        .unwrap_or_else(|| "~/…".to_string())
}

// =============================================================================
// RESPONSE TYPES — Structured JSON for all tools
// =============================================================================

/// Standard success/error wrapper for all MCP responses.
#[derive(Serialize)]
struct McpResponse<T: Serialize> {
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Serialize)]
struct McpError {
    code: String,
    message: String,
    retriable: bool,
}

impl<T: Serialize> McpResponse<T> {
    fn ok(result: T) -> String {
        serde_json::to_string(&McpResponse {
            result: Some(result),
            error: None,
        })
        .unwrap_or_else(|_| {
            r#"{"error":{"code":"serialize","message":"serialization failed","retriable":false}}"#
                .to_string()
        })
    }
}

fn mcp_err(code: &str, message: impl Into<String>, retriable: bool) -> String {
    serde_json::to_string(&McpResponse::<()> {
        result: None,
        error: Some(McpError {
            code: code.to_string(),
            message: message.into(),
            retriable,
        }),
    })
    .unwrap_or_else(|_| {
        r#"{"error":{"code":"unknown","message":"serialization failed","retriable":false}}"#
            .to_string()
    })
}

fn mcp_not_found(entity: &str, name: &str) -> String {
    mcp_err(
        "not_found",
        format!("{} '{}' not found", entity, name),
        false,
    )
}

fn mcp_invalid(message: impl Into<String>) -> String {
    mcp_err("invalid_params", message, false)
}

fn mcp_sys_err(e: impl std::fmt::Display) -> String {
    mcp_err("system", e.to_string(), true)
}

// --- Response data structs (shared with CLI --json) ---
use crate::output::*;

// =============================================================================
// PARAMETER TYPES
// =============================================================================

// --- Standalone params ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListEnvironmentsParams {
    #[schemars(description = "Optional label to filter by (e.g., 'ml', 'dev', 'favorite')")]
    pub label: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunInEnvironmentParams {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(
        description = "Command and arguments to run, e.g. ['python', '-c', 'import torch; print(torch.__version__)']"
    )]
    pub command: Vec<String>,
    #[schemars(description = "Timeout in seconds. Default 120. Set to 0 for no timeout.")]
    pub timeout: Option<u64>,
    #[schemars(
        description = "Working directory for the command. Defaults to home directory if not specified."
    )]
    pub cwd: Option<String>,
    #[schemars(
        description = "Path to save full command output (stdout+stderr). When provided, the complete untruncated output is written to this file and the path is returned in 'log_file'. The inline 'output' field still contains the truncated preview."
    )]
    pub log_path: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CompareEnvironmentsParams {
    #[schemars(description = "List of environment names to compare")]
    pub env_names: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InstallPackagesParams {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(
        description = "Packages to install. Accepts PyPI names (numpy), version specs (numpy>=2.0), local wheel paths (/path/to/pkg.whl), and URLs"
    )]
    pub packages: Vec<String>,
    #[schemars(
        description = "Custom PyPI index URL (e.g., https://download.pytorch.org/whl/cu130). Install only"
    )]
    pub index_url: Option<String>,
    #[schemars(
        description = "Additional PyPI index URL (used alongside default PyPI). Install only"
    )]
    pub extra_index_url: Option<String>,
    #[schemars(description = "Include pre-release/development versions. Install only")]
    pub pre: Option<bool>,
    #[schemars(description = "Upgrade existing packages to latest version. Install only")]
    pub upgrade: Option<bool>,
    #[schemars(description = "Install in editable/development mode (-e). Install only")]
    pub editable: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UninstallPackagesParams {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(description = "Package names to uninstall")]
    pub packages: Vec<String>,
}

// --- Dispatch params ---

/// Parameters for `manage_environment` — lifecycle operations.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ManageEnvironmentParams {
    #[schemars(
        description = "Action to perform: 'create', 'remove', 'rename', 'track', or 'untrack'"
    )]
    pub action: String,
    #[schemars(description = "Environment name (required for create, remove, untrack)")]
    pub name: Option<EnvName>,
    #[schemars(description = "Python version (e.g., 3.12). Used with action 'create'")]
    pub python: Option<String>,
    #[schemars(description = "Absolute path to the virtual environment. Used with action 'track'")]
    pub path: Option<String>,
    #[schemars(description = "Current name. Used with action 'rename'")]
    pub old_name: Option<String>,
    #[schemars(description = "New name. Used with action 'rename'")]
    pub new_name: Option<String>,
}

/// Parameters for `inspect_environment` — read environment state.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectEnvironmentParams {
    #[schemars(description = "Action: 'details' or 'health'")]
    pub action: String,
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
}

/// Parameters for `find_package` — search across envs or get details in one env.
///
/// When `env_name` is provided, returns detailed info for the package in that env.
/// Otherwise, searches across all environments (substring match, wildcards, version pinning).
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindPackageParams {
    #[schemars(
        description = "Package name or pattern. Supports wildcards (*torch*) and version pinning (torch==2.10). CUDA-aware: 'torch==2.10' matches '2.10.0+cu130'"
    )]
    pub query: String,
    #[schemars(
        description = "Optional: environment name. When provided, returns detailed package info (version, installer, source, editable status) instead of cross-environment search"
    )]
    pub env_name: Option<EnvName>,
}

/// Parameters for `manage_project` — project-environment links.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ManageProjectParams {
    #[schemars(description = "Action: 'link', 'get_default', or 'list'")]
    pub action: String,
    #[schemars(description = "Absolute path to the project directory")]
    pub project_path: String,
    #[schemars(description = "Environment to link. Required for action 'link'")]
    pub env_name: Option<EnvName>,
    #[schemars(description = "Optional tag like 'main', 'test', 'experiment'. Used with 'link'")]
    pub tag: Option<String>,
    #[schemars(description = "Set as default environment for this project. Used with 'link'")]
    pub is_default: Option<bool>,
}

/// Parameters for `manage_metadata` — notes and labels.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ManageMetadataParams {
    #[schemars(description = "Action: 'add_note', 'get_notes', 'add_label', or 'remove_label'")]
    pub action: String,
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(description = "The note text. Required for action 'add_note'")]
    pub note: Option<String>,
    #[schemars(
        description = "Label to add or remove (e.g., ml, dev, testing). Required for 'add_label' and 'remove_label'"
    )]
    pub label: Option<String>,
}

// =============================================================================
// MCP SERVER
// =============================================================================

/// The Zen MCP Server.
#[derive(Clone)]
pub struct ZenMcpServer {
    db: Arc<Mutex<Database>>,
    home: PathBuf,
    tool_router: ToolRouter<Self>,
}

impl ZenMcpServer {
    pub fn new(db: Database, home: PathBuf) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            home,
            tool_router: Self::tool_router(),
        }
    }
}

// =============================================================================
// TOOL IMPLEMENTATIONS — 11 tools
// =============================================================================

#[tool_router]
impl ZenMcpServer {
    // -------------------------------------------------------------------------
    // 1. get_version (standalone)
    // -------------------------------------------------------------------------
    #[tool(description = "Get the version of the running Zen server")]
    fn get_version(&self) -> String {
        McpResponse::ok(VersionResponse {
            version: format!("zen {}", env!("ZEN_VERSION")),
        })
    }

    // -------------------------------------------------------------------------
    // 2. list_environments (standalone)
    // -------------------------------------------------------------------------
    #[tool(
        description = "List all managed Python environments with their Python versions and paths"
    )]
    fn list_environments(&self, Parameters(params): Parameters<ListEnvironmentsParams>) -> String {
        let db = self.db.lock().unwrap();

        // Auto-discover ZEN_HOME environments (same as CLI zen list).
        // Directory is truth — scan disk first, then query DB.
        if self.home.exists()
            && let Ok(entries) = std::fs::read_dir(&self.home)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && (path.join("bin/python").exists() || path.join("bin/python3").exists())
                {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if db.get_env_id(&name).ok().flatten().is_none() {
                        let path_str = path.to_string_lossy().to_string();
                        let py_ver = crate::utils::read_python_version(&path_str)
                            .unwrap_or_else(|| "unknown".to_string());
                        if let Err(e) = db.register_env(&name, &path_str, &py_ver) {
                            eprintln!("Warning: failed to register env '{}': {}", name, e);
                        }
                    }
                    // Collision detection is silent in MCP (no stderr),
                    // but the tracked alias takes precedence by design.
                }
            }
        }

        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.list_envs() {
            Ok(envs) => {
                let mut result: Vec<EnvSummary> = Vec::new();
                for (name, path, py_ver, ..) in &envs {
                    if let Some(ref label) = params.label {
                        let labels = db.get_labels(name).unwrap_or_default();
                        if !labels.iter().any(|l| l == label) {
                            continue;
                        }
                    }
                    result.push(EnvSummary {
                        name: name.clone(),
                        python: py_ver.clone(),
                        path: redact_path(path),
                    });
                }
                McpResponse::ok(result)
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // -------------------------------------------------------------------------
    // 3. run_in_environment (standalone)
    // -------------------------------------------------------------------------
    #[tool(
        description = "Run a command inside an environment without activating it. Returns stdout/stderr output (capped at 10KB). Use 'log_path' to save full untruncated output to a file when output may exceed 10KB. Example: command=['python', '-c', 'import torch; print(torch.__version__)']"
    )]
    fn run_in_environment(&self, Parameters(params): Parameters<RunInEnvironmentParams>) -> String {
        let db = self.db.lock().unwrap();

        let env_name = params.env_name.clone();
        let command = params.command;

        let envs = match db.list_envs() {
            Ok(e) => e,
            Err(e) => return mcp_sys_err(e),
        };
        let env_entry = envs.iter().find(|(n, ..)| n == env_name.as_str());
        let env_path = match env_entry {
            Some((_, path, ..)) => path.clone(),
            None => return mcp_not_found("Environment", env_name.as_str()),
        };
        drop(db);

        let timeout_secs = params.timeout.unwrap_or(120);
        let cwd = params.cwd;
        let log_path = params.log_path;

        let handle = std::thread::spawn(move || {
            if command.is_empty() {
                return Err("No command specified".to_string());
            }
            let env_p = std::path::Path::new(&env_path);
            let bin_path = env_p.join("bin");
            let exe_path = bin_path.join(&command[0]);
            let program = if exe_path.exists() {
                exe_path.to_string_lossy().to_string()
            } else {
                command[0].clone()
            };
            let path_var = std::env::var("PATH").unwrap_or_default();

            let mut cmd = std::process::Command::new(&program);
            cmd.args(&command[1..])
                .env("PATH", format!("{}:{}", bin_path.display(), path_var))
                .env("VIRTUAL_ENV", env_p)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            if let Some(ref dir) = cwd {
                cmd.current_dir(dir);
            }
            let mut child = cmd
                .spawn()
                .map_err(|e| format!("Failed to execute: {}", e))?;

            if timeout_secs == 0 {
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("Failed to wait: {}", e))?;
                let exit_code = output.status.code().unwrap_or(-1);
                let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str(&stderr);
                }
                Ok((exit_code, combined, false))
            } else {
                let deadline =
                    std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let mut stdout_buf = Vec::new();
                            let mut stderr_buf = Vec::new();
                            use std::io::Read;
                            if let Some(ref mut out) = child.stdout {
                                let _ = out.read_to_end(&mut stdout_buf);
                            }
                            if let Some(ref mut err) = child.stderr {
                                let _ = err.read_to_end(&mut stderr_buf);
                            }
                            let exit_code = status.code().unwrap_or(-1);
                            let mut combined = String::from_utf8_lossy(&stdout_buf).to_string();
                            let stderr = String::from_utf8_lossy(&stderr_buf);
                            if !stderr.is_empty() {
                                if !combined.is_empty() {
                                    combined.push('\n');
                                }
                                combined.push_str(&stderr);
                            }
                            return Ok((exit_code, combined, false));
                        }
                        Ok(None) => {
                            if std::time::Instant::now() >= deadline {
                                let _ = child.kill();
                                let _ = child.wait(); // Reap zombie
                                return Ok((
                                    -1,
                                    format!("Command timed out after {}s", timeout_secs),
                                    true,
                                ));
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(e) => return Err(format!("Error waiting for process: {}", e)),
                    }
                }
            }
        });

        match handle.join() {
            Ok(Ok((code, output, timed_out))) => {
                // Write full output to log file if requested
                let log_file = log_path.and_then(|path| match std::fs::write(&path, &output) {
                    Ok(()) => Some(path),
                    Err(e) => {
                        eprintln!("Warning: failed to write log to {}: {}", path, e);
                        None
                    }
                });

                let truncated = output.len() > 10240;
                let output = if truncated {
                    output[..10240].to_string()
                } else {
                    output
                };
                McpResponse::ok(RunResult {
                    exit_code: code,
                    output,
                    truncated: if truncated { Some(true) } else { None },
                    timed_out: if timed_out { Some(true) } else { None },
                    log_file,
                })
            }
            Ok(Err(e)) => mcp_err("exec_failed", e, true),
            Err(_) => mcp_err("panic", "Command execution panicked", false),
        }
    }

    // -------------------------------------------------------------------------
    // 4. compare_environments (standalone)
    // -------------------------------------------------------------------------
    #[tool(description = "Compare two environments side-by-side")]
    fn compare_environments(
        &self,
        Parameters(params): Parameters<CompareEnvironmentsParams>,
    ) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        if params.env_names.len() != 2 {
            return mcp_invalid("Exactly two environment names are required");
        }

        match ops.list_envs() {
            Ok(all_envs) => {
                let mut environments = Vec::new();
                let mut env_packages: Vec<(String, std::collections::HashMap<String, String>)> =
                    Vec::new();

                for name in &params.env_names {
                    let env = all_envs.iter().find(|(n, ..)| n == name);
                    if let Some((_, path, py_ver, ..)) = env {
                        let packages = crate::utils::get_packages(path);
                        let pkg_map: std::collections::HashMap<String, String> = packages
                            .into_iter()
                            .map(|p| {
                                (
                                    p.name.to_lowercase(),
                                    p.version.unwrap_or_else(|| "?".into()),
                                )
                            })
                            .collect();
                        environments.push(EnvCompare {
                            name: name.clone(),
                            python: py_ver.clone(),
                            packages: pkg_map.len(),
                        });
                        env_packages.push((name.clone(), pkg_map));
                    } else {
                        return mcp_not_found("Environment", name);
                    }
                }

                let mut version_diffs = Vec::new();
                let mut only_in = Vec::new();

                {
                    let (ref n1, ref pkgs1) = env_packages[0];
                    let (ref n2, ref pkgs2) = env_packages[1];

                    for (name, v1) in pkgs1 {
                        if let Some(v2) = pkgs2.get(name)
                            && v1 != v2
                        {
                            version_diffs.push(VersionDiff {
                                package: name.clone(),
                                versions: vec![v1.clone(), v2.clone()],
                            });
                        }
                    }
                    version_diffs.sort_by(|a, b| a.package.cmp(&b.package));

                    let mut o1: Vec<String> = pkgs1
                        .keys()
                        .filter(|k| !pkgs2.contains_key(*k))
                        .cloned()
                        .collect();
                    o1.sort();
                    if !o1.is_empty() {
                        only_in.push(OnlyIn {
                            env: n1.clone(),
                            packages: o1,
                        });
                    }

                    let mut o2: Vec<String> = pkgs2
                        .keys()
                        .filter(|k| !pkgs1.contains_key(*k))
                        .cloned()
                        .collect();
                    o2.sort();
                    if !o2.is_empty() {
                        only_in.push(OnlyIn {
                            env: n2.clone(),
                            packages: o2,
                        });
                    }
                }

                McpResponse::ok(ComparisonResult {
                    environments,
                    version_diffs,
                    only_in,
                })
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // -------------------------------------------------------------------------
    // 5. install_packages (standalone)
    // -------------------------------------------------------------------------
    #[tool(
        description = "Install packages into an environment. Supports PyPI names, version specs, wheels, editable installs, custom index URLs, pre-release, and upgrade"
    )]
    fn install_packages(&self, Parameters(params): Parameters<InstallPackagesParams>) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        let opts = crate::ops::InstallOptions {
            index_url: params.index_url,
            extra_index_url: params.extra_index_url,
            pre: params.pre.unwrap_or(false),
            upgrade: params.upgrade.unwrap_or(false),
            editable: params.editable.unwrap_or(false),
            dry_run: false,
        };

        match ops.install_packages(&params.env_name, params.packages.clone(), opts) {
            Ok(msg) => {
                crate::activity_log::log_activity(
                    "mcp",
                    "install",
                    &format!("{} {}", params.env_name.as_str(), params.packages.join(" ")),
                );
                McpResponse::ok(ActionResult { message: msg })
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // -------------------------------------------------------------------------
    // 6. uninstall_packages (standalone)
    // -------------------------------------------------------------------------
    #[tool(description = "Remove packages from an environment")]
    fn uninstall_packages(
        &self,
        Parameters(params): Parameters<UninstallPackagesParams>,
    ) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.uninstall_packages(&params.env_name, params.packages.clone()) {
            Ok(msg) => {
                crate::activity_log::log_activity(
                    "mcp",
                    "uninstall",
                    &format!("{} {}", params.env_name.as_str(), params.packages.join(" ")),
                );
                McpResponse::ok(ActionResult { message: msg })
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // -------------------------------------------------------------------------
    // 7. manage_environment (dispatch: create/remove/rename/track/untrack)
    // -------------------------------------------------------------------------
    #[tool(
        description = "Manage environment lifecycle. Actions: 'create' (name, python), 'remove' (name), 'rename' (old_name, new_name), 'track' (path, name?), 'untrack' (name)"
    )]
    fn manage_environment(
        &self,
        Parameters(params): Parameters<ManageEnvironmentParams>,
    ) -> String {
        match params.action.as_str() {
            "create" => self.do_create_environment(params),
            "remove" => self.do_remove_environment(params),
            "rename" => self.do_rename_environment(params),
            "track" => self.do_track_environment(params),
            "untrack" => self.do_untrack_environment(params),
            _ => mcp_invalid(format!(
                "Unknown action '{}'. Use: create, remove, rename, track, untrack",
                params.action
            )),
        }
    }

    // -------------------------------------------------------------------------
    // 8. inspect_environment (dispatch: details/health)
    // -------------------------------------------------------------------------
    #[tool(
        description = "Inspect an environment. Actions: 'details' (Python version, packages, ML frameworks, creation date), 'health' (package conflicts, missing dependencies, CUDA consistency)"
    )]
    fn inspect_environment(
        &self,
        Parameters(params): Parameters<InspectEnvironmentParams>,
    ) -> String {
        match params.action.as_str() {
            "details" => self.do_get_details(&params.env_name),
            "health" => self.do_get_health(&params.env_name),
            _ => mcp_invalid(format!(
                "Unknown action '{}'. Use: details, health",
                params.action
            )),
        }
    }

    // -------------------------------------------------------------------------
    // 9. find_package (inferred: search or details based on env_name)
    // -------------------------------------------------------------------------
    #[tool(
        description = "Find a package across all environments, or get detailed info for a specific package in one environment. Supports wildcards (*torch*) and version pinning (torch==2.10). CUDA-aware: queries without +cuXXX match base version. When env_name is provided, returns detailed package info (version, installer, source, editable status, git commit)"
    )]
    fn find_package(&self, Parameters(params): Parameters<FindPackageParams>) -> String {
        if let Some(ref env_name) = params.env_name {
            self.do_get_package_details(env_name, &params.query)
        } else {
            self.do_find_package_cross_env(&params.query)
        }
    }

    // -------------------------------------------------------------------------
    // 10. manage_project (dispatch: link/get_default/list)
    // -------------------------------------------------------------------------
    #[tool(
        description = "Manage project-environment links. Actions: 'link' (project_path, env_name, tag?, is_default?), 'get_default' (project_path), 'list' (project_path)"
    )]
    fn manage_project(&self, Parameters(params): Parameters<ManageProjectParams>) -> String {
        match params.action.as_str() {
            "link" => self.do_associate_project(params),
            "get_default" => self.do_get_default_environment(&params.project_path),
            "list" => self.do_get_project_environments(&params.project_path),
            _ => mcp_invalid(format!(
                "Unknown action '{}'. Use: link, get_default, list",
                params.action
            )),
        }
    }

    // -------------------------------------------------------------------------
    // 11. manage_metadata (dispatch: add_note/get_notes/add_label/remove_label)
    // -------------------------------------------------------------------------
    #[tool(
        description = "Manage environment metadata (notes and labels). Actions: 'add_note' (env_name, note), 'get_notes' (env_name), 'add_label' (env_name, label), 'remove_label' (env_name, label)"
    )]
    fn manage_metadata(&self, Parameters(params): Parameters<ManageMetadataParams>) -> String {
        match params.action.as_str() {
            "add_note" => self.do_add_note(&params.env_name, params.note.as_deref()),
            "get_notes" => self.do_get_notes(&params.env_name),
            "add_label" => self.do_add_label(&params.env_name, params.label.as_deref()),
            "remove_label" => self.do_remove_label(&params.env_name, params.label.as_deref()),
            _ => mcp_invalid(format!(
                "Unknown action '{}'. Use: add_note, get_notes, add_label, remove_label",
                params.action
            )),
        }
    }
}

// =============================================================================
// PRIVATE DISPATCH IMPLEMENTATIONS
// =============================================================================

impl ZenMcpServer {
    // --- manage_environment dispatches ---

    fn do_create_environment(&self, params: ManageEnvironmentParams) -> String {
        let name = match params.name {
            Some(n) => n,
            None => return mcp_invalid("'name' is required for action 'create'"),
        };
        let db = self.db.lock().unwrap();

        // Guard: reject if name already exists in DB (prevents duplicates after rename)
        match db.get_env_id(&name) {
            Ok(Some(_)) => {
                return mcp_err(
                    "already_exists",
                    format!("Environment '{}' already exists", name),
                    false,
                );
            }
            Ok(None) => {}
            Err(e) => return mcp_sys_err(e),
        }

        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);
        match ops.create_env(&name, params.python) {
            Ok(msg) => McpResponse::ok(ActionResult { message: msg }),
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_remove_environment(&self, params: ManageEnvironmentParams) -> String {
        let name = match params.name {
            Some(n) => n,
            None => return mcp_invalid("'name' is required for action 'remove'"),
        };
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);
        match ops.remove_env(&name) {
            Ok(msg) => {
                crate::activity_log::log_activity("mcp", "rm", name.as_str());
                McpResponse::ok(ActionResult { message: msg })
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_rename_environment(&self, params: ManageEnvironmentParams) -> String {
        let old_name_str = match params.old_name {
            Some(n) => n,
            None => return mcp_invalid("'old_name' is required for action 'rename'"),
        };
        let new_name_str = match params.new_name {
            Some(n) => n,
            None => return mcp_invalid("'new_name' is required for action 'rename'"),
        };

        let old = match crate::types::EnvName::new(&old_name_str) {
            Ok(n) => n,
            Err(e) => return mcp_invalid(e.to_string()),
        };
        let new = match crate::types::EnvName::new(&new_name_str) {
            Ok(n) => n,
            Err(e) => return mcp_invalid(e.to_string()),
        };

        let db = self.db.lock().unwrap();

        // Check old name exists
        match db.get_env_id(&old) {
            Ok(Some(_)) => {}
            Ok(None) => return mcp_not_found("Environment", old.as_str()),
            Err(e) => return mcp_sys_err(e),
        }
        // Check new name doesn't collide
        match db.get_env_id(&new) {
            Ok(Some(_)) => {
                return mcp_err(
                    "already_exists",
                    format!("Environment '{}' already exists", new),
                    false,
                );
            }
            Ok(None) => {}
            Err(e) => return mcp_sys_err(e),
        }

        // Get current path to determine managed vs tracked
        let current_path = match db.get_env_path(old.as_str()) {
            Ok(Some(p)) => p,
            Ok(None) => return mcp_not_found("Environment", old.as_str()),
            Err(e) => return mcp_sys_err(e),
        };

        let path = std::path::Path::new(&current_path);
        let is_managed = path.starts_with(&self.home);

        if is_managed {
            // Managed env: rename directory on disk + update name and path in DB
            let new_path = self.home.join(new.as_str());
            if new_path.exists() {
                return mcp_err(
                    "already_exists",
                    format!(
                        "Directory '{}' already exists on disk",
                        redact_path(&new_path.to_string_lossy())
                    ),
                    false,
                );
            }

            if let Err(e) = std::fs::rename(path, &new_path) {
                return mcp_err("system", format!("Failed to rename directory: {}", e), true);
            }

            let new_path_str = new_path.to_string_lossy().to_string();
            match db.rename_environment_with_path(&old_name_str, &new_name_str, &new_path_str) {
                Ok(true) => {
                    crate::activity_log::log_activity(
                        "mcp",
                        "rename",
                        &format!("{} -> {} (managed, dir moved)", old_name_str, new_name_str),
                    );
                    McpResponse::ok(ActionResult {
                        message: format!(
                            "Renamed '{}' → '{}' (managed — directory moved)",
                            old_name_str, new_name_str
                        ),
                    })
                }
                Ok(false) => mcp_err("rename_failed", "DB rename failed after dir move", true),
                Err(e) => mcp_sys_err(e),
            }
        } else {
            // Tracked env: only change alias in DB, path stays unchanged
            match db.rename_environment(&old_name_str, &new_name_str) {
                Ok(true) => {
                    crate::activity_log::log_activity(
                        "mcp",
                        "rename",
                        &format!("{} -> {} (tracked, alias only)", old_name_str, new_name_str),
                    );
                    McpResponse::ok(ActionResult {
                        message: format!(
                            "Renamed '{}' → '{}' (tracked — alias updated, path unchanged)",
                            old_name_str, new_name_str
                        ),
                    })
                }
                Ok(false) => mcp_err("rename_failed", "Rename failed", true),
                Err(e) => mcp_sys_err(e),
            }
        }
    }

    fn do_track_environment(&self, params: ManageEnvironmentParams) -> String {
        let path_str = match params.path {
            Some(p) => p,
            None => return mcp_invalid("'path' is required for action 'track'"),
        };
        let db = self.db.lock().unwrap();
        let path = std::path::PathBuf::from(&path_str);

        let fname = path.file_name().unwrap_or_default().to_string_lossy();
        let parent_is_bin = path
            .parent()
            .and_then(|p| p.file_name())
            .is_some_and(|n| n == "bin");
        let resolved = if parent_is_bin && (fname.starts_with("python") || fname == "activate") {
            path.parent()
                .and_then(|p| p.parent())
                .unwrap_or(&path)
                .to_path_buf()
        } else {
            path.clone()
        };

        let resolved = match resolved.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                return mcp_not_found("Path", &resolved.to_string_lossy());
            }
        };

        if !resolved.join("bin/python").exists() {
            return mcp_invalid(format!(
                "Not a valid virtual environment (no bin/python): {}",
                resolved.display()
            ));
        }

        // Guard: reject tracking envs inside ZEN_HOME — they are managed by auto-discovery.
        // Allowing aliases for ZEN_HOME envs would break the directory-is-truth invariant.
        if resolved.starts_with(&self.home) {
            return mcp_invalid(format!(
                "Cannot track environments inside ZEN_HOME ({}). Environments there are managed automatically.",
                redact_path(&self.home.to_string_lossy())
            ));
        }

        let env_name_str = params.name.map(|n| n.to_string()).unwrap_or_else(|| {
            resolved
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        let env_name = match crate::types::EnvName::new(&env_name_str) {
            Ok(n) => n,
            Err(e) => return mcp_invalid(e.to_string()),
        };

        if db.get_env_id(&env_name).ok().flatten().is_some() {
            return mcp_err(
                "already_exists",
                format!("Environment '{}' already registered", env_name),
                false,
            );
        }
        let resolved_str = resolved.to_string_lossy().to_string();
        if let Ok(Some(existing)) = db.get_env_name_by_path(&resolved_str) {
            return mcp_err(
                "already_exists",
                format!("Path already registered as '{}'", existing),
                false,
            );
        }

        let py_ver =
            crate::utils::read_python_version(&resolved).unwrap_or_else(|| "unknown".to_string());

        match db.register_env(&env_name_str, &resolved_str, &py_ver) {
            Ok(_) => {
                crate::activity_log::log_activity(
                    "mcp",
                    "add",
                    &format!("{} -> {}", env_name, redact_path(&resolved_str)),
                );
                McpResponse::ok(ActionResult {
                    message: format!(
                        "Registered '{}' (Python {}) at {}",
                        env_name,
                        py_ver,
                        redact_path(&resolved_str)
                    ),
                })
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_untrack_environment(&self, params: ManageEnvironmentParams) -> String {
        let name = match params.name {
            Some(n) => n,
            None => return mcp_invalid("'name' is required for action 'untrack'"),
        };
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);
        match ops.untrack_env(&name) {
            Ok(msg) => {
                crate::activity_log::log_activity("mcp", "rm:cached", name.as_str());
                McpResponse::ok(ActionResult { message: msg })
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // --- inspect_environment dispatches ---

    fn do_get_details(&self, env_name: &EnvName) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.list_envs() {
            Ok(envs) => {
                let env = envs.iter().find(|(n, ..)| n == env_name.as_str());
                match env {
                    Some((name, path, py_ver, ..)) => {
                        let packages = crate::utils::get_packages(path);

                        let created = crate::utils::get_env_created_at(path).and_then(|epoch| {
                            use chrono::{Local, TimeZone};
                            Local
                                .timestamp_opt(epoch, 0)
                                .single()
                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        });

                        let (torch, cuda) = crate::utils::read_torch_version(path)
                            .map(|(t, c)| (Some(t), c))
                            .unwrap_or((None, None));

                        let numpy = packages
                            .iter()
                            .find(|p| p.name == "numpy")
                            .and_then(|p| p.version.clone());

                        let is_protected = match db.is_protected(name) {
                            Ok(p) => p,
                            Err(e) => return mcp_sys_err(e),
                        };

                        McpResponse::ok(EnvDetails {
                            name: name.clone(),
                            python: py_ver.clone(),
                            path: redact_path(path),
                            packages: packages.len(),
                            created,
                            torch,
                            cuda,
                            numpy,
                            is_protected,
                        })
                    }
                    None => mcp_not_found("Environment", env_name.as_str()),
                }
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_get_health(&self, env_name: &EnvName) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.check_health(env_name) {
            Ok(report) => {
                let overall = report.overall();
                let checks: Vec<HealthCheck> = report
                    .items
                    .iter()
                    .map(|item| HealthCheck {
                        check: item.message(),
                        status: format!("{}", item.level()),
                        message: item.message(),
                    })
                    .collect();

                McpResponse::ok(HealthResponse {
                    env_name: env_name.to_string(),
                    overall: format!("{}", overall),
                    checks,
                })
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // --- find_package dispatches ---

    fn do_get_package_details(&self, env_name: &EnvName, query: &str) -> String {
        let db = self.db.lock().unwrap();

        match db.list_envs() {
            Ok(envs) => {
                let env = envs.iter().find(|(n, ..)| n == env_name.as_str());
                match env {
                    Some((name, path, ..)) => {
                        let packages = crate::utils::get_packages(path);
                        let pkg_lower = query.to_lowercase();
                        let found = packages
                            .into_iter()
                            .find(|p| p.name.to_lowercase() == pkg_lower);

                        match found {
                            Some(pkg) => {
                                let installed_at = pkg.installed_at.and_then(|epoch| {
                                    use chrono::{Local, TimeZone};
                                    Local
                                        .timestamp_opt(epoch, 0)
                                        .single()
                                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                });

                                McpResponse::ok(PackageDetails {
                                    name: pkg.name,
                                    env: name.clone(),
                                    version: pkg.version.unwrap_or_else(|| "unknown".to_string()),
                                    installer: pkg
                                        .installer
                                        .unwrap_or_else(|| "unknown".to_string()),
                                    source: pkg
                                        .install_source
                                        .unwrap_or_else(|| "unknown".to_string()),
                                    editable: pkg.is_editable,
                                    url: pkg.source_url,
                                    commit: pkg.commit_id,
                                    import_name: pkg.import_name,
                                    installed_at,
                                })
                            }
                            None => mcp_not_found(
                                "Package",
                                &format!("{} in environment '{}'", query, name),
                            ),
                        }
                    }
                    None => mcp_not_found("Environment", env_name.as_str()),
                }
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_find_package_cross_env(&self, query: &str) -> String {
        let db = self.db.lock().unwrap();

        let (pkg_query, version_query) = if query.contains("==") {
            let parts: Vec<&str> = query.split("==").collect();
            (
                parts[0].to_string(),
                Some(parts.get(1).unwrap_or(&"").to_string()),
            )
        } else {
            (query.to_string(), None)
        };

        let normalize = |s: &str| s.to_lowercase().replace('-', "_");
        let pattern = normalize(&pkg_query.replace('*', ""));

        match db.list_envs() {
            Ok(envs) => {
                let mut found = Vec::new();
                for (name, path, ..) in &envs {
                    let packages = crate::utils::get_packages(path);
                    for pkg in packages {
                        let pkg_norm = normalize(&pkg.name);
                        let name_match = pkg_norm.contains(&pattern);

                        let version_match = match (&version_query, &pkg.version) {
                            (Some(q), Some(v)) => {
                                if q.contains('+') {
                                    v == q
                                } else {
                                    let base_ver = v.split('+').next().unwrap_or(v);
                                    base_ver.starts_with(q.as_str())
                                }
                            }
                            (Some(_), None) => false,
                            (None, _) => true,
                        };

                        if name_match && version_match {
                            found.push(PackageMatch {
                                env: name.clone(),
                                package: pkg.name,
                                version: pkg.version.unwrap_or_else(|| "?".to_string()),
                            });
                        }
                    }
                }
                McpResponse::ok(found)
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // --- manage_project dispatches ---

    fn do_associate_project(&self, params: ManageProjectParams) -> String {
        let env_name = match params.env_name {
            Some(n) => n,
            None => return mcp_invalid("'env_name' is required for action 'link'"),
        };
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.associate_project(
            &params.project_path,
            &env_name,
            params.tag.as_deref(),
            params.is_default.unwrap_or(false),
        ) {
            Ok(msg) => McpResponse::ok(ActionResult { message: msg }),
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_get_default_environment(&self, project_path: &str) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.get_default_env(project_path) {
            Ok(Some(env)) => McpResponse::ok(ActionResult { message: env }),
            Ok(None) => McpResponse::ok(ActionResult {
                message: "No default environment set".to_string(),
            }),
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_get_project_environments(&self, project_path: &str) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.get_project_envs(project_path) {
            Ok(envs) => {
                let links: Vec<ProjectLink> = envs
                    .into_iter()
                    .map(|(name, _path, tag, is_default)| ProjectLink {
                        env: name,
                        tag,
                        is_default,
                    })
                    .collect();
                McpResponse::ok(links)
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    // --- manage_metadata dispatches ---

    fn do_add_note(&self, env_name: &EnvName, note: Option<&str>) -> String {
        let note = match note {
            Some(n) => n,
            None => return mcp_invalid("'note' is required for action 'add_note'"),
        };
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.add_env_note(env_name, note) {
            Ok(msg) => McpResponse::ok(ActionResult { message: msg }),
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_get_notes(&self, env_name: &EnvName) -> String {
        let db = self.db.lock().unwrap();
        let ops =
            crate::ops::ZenOps::new(&db, self.home.clone(), crate::context::OutputMode::Plain);

        match ops.list_comments(None, Some(env_name)) {
            Ok(comments) => {
                let notes: Vec<NoteEntry> = comments
                    .into_iter()
                    .map(|(_uuid, _pp, _env, msg, _tag, ts)| NoteEntry {
                        timestamp: ts,
                        text: msg,
                    })
                    .collect();
                McpResponse::ok(notes)
            }
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_add_label(&self, env_name: &EnvName, label: Option<&str>) -> String {
        let label = match label {
            Some(l) => l,
            None => return mcp_invalid("'label' is required for action 'add_label'"),
        };
        let db = self.db.lock().unwrap();
        match db.add_label(env_name, label) {
            Ok(_) => McpResponse::ok(ActionResult {
                message: format!("Added label '{}' to '{}'", label, env_name),
            }),
            Err(e) => mcp_sys_err(e),
        }
    }

    fn do_remove_label(&self, env_name: &EnvName, label: Option<&str>) -> String {
        let label = match label {
            Some(l) => l,
            None => return mcp_invalid("'label' is required for action 'remove_label'"),
        };
        let db = self.db.lock().unwrap();
        match db.remove_label(env_name, label) {
            Ok(_) => McpResponse::ok(ActionResult {
                message: format!("Removed label '{}' from '{}'", label, env_name),
            }),
            Err(e) => mcp_sys_err(e),
        }
    }
}

// =============================================================================
// SERVER HANDLER
// =============================================================================

#[rmcp::tool_handler]
impl ServerHandler for ZenMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Zen - manage Python environments, packages, and project associations".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Starts the MCP server on stdio transport.
pub async fn run_server(db: Database, home: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::serve_server;

    eprintln!("Zen MCP Server v{} starting...", env!("CARGO_PKG_VERSION"));

    let server = ZenMcpServer::new(db, home);
    let service = serve_server(server, stdio())
        .await
        .inspect_err(|e| eprintln!("Server error: {}", e))?;

    service.waiting().await?;
    Ok(())
}
