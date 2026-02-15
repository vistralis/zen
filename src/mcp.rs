// SPDX-License-Identifier: Apache-2.0

//! Model Context Protocol (MCP) server for Zen.
//!
//! This module implements an MCP server using the official rmcp SDK,
//! allowing Zen to interface with AI agents (like Antigravity or Claude Desktop).
//!
//! Environment names are validated at the MCP boundary via `EnvName` deserialization.

use crate::db::Database;
use crate::types::EnvName;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_router,
    transport::stdio,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Redacts a filesystem path for MCP responses.
///
/// Replaces full paths with `~/…/basename` to prevent sensitive directory
/// structures from leaking into LLM provider logs.
fn redact_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| format!("~/…/{}", name.to_string_lossy()))
        .unwrap_or_else(|| "~/…".to_string())
}

/// Input parameter types for MCP tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateEnvironmentParams {
    #[schemars(description = "Name of the environment")]
    pub name: EnvName,
    #[schemars(description = "Python version (e.g., 3.12)")]
    pub python: Option<String>,
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
        description = "Custom PyPI index URL (e.g., https://download.pytorch.org/whl/cu130)"
    )]
    pub index_url: Option<String>,
    #[schemars(description = "Additional PyPI index URL (used alongside default PyPI)")]
    pub extra_index_url: Option<String>,
    #[schemars(description = "Include pre-release/development versions")]
    pub pre: Option<bool>,
    #[schemars(description = "Upgrade existing packages to latest version")]
    pub upgrade: Option<bool>,
    #[schemars(description = "Install in editable/development mode (-e)")]
    pub editable: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UninstallPackagesParams {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(description = "List of packages to uninstall")]
    pub packages: Vec<String>,
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
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListEnvironmentsParams {
    #[schemars(description = "Optional label to filter by (e.g., 'ml', 'dev', 'favorite')")]
    pub label: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EnvNameParam {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectPathParam {
    #[schemars(description = "Absolute path to the project directory")]
    pub project_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AssociateProjectParams {
    #[schemars(description = "Absolute path to the project directory")]
    pub project_path: String,
    #[schemars(description = "Name of the environment to associate")]
    pub env_name: EnvName,
    #[schemars(description = "Optional tag like 'main', 'test', 'experiment'")]
    pub tag: Option<String>,
    #[schemars(description = "Set as default environment for this project")]
    pub is_default: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CompareEnvironmentsParams {
    #[schemars(description = "List of environment names to compare")]
    pub env_names: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddEnvironmentNoteParams {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(description = "The note to record (purpose, description, reminder)")]
    pub note: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchPackagesParams {
    #[schemars(description = "Package name or partial name to search for")]
    pub query: String,
}

/// Parameters for the `find_package` MCP tool.
///
/// Accepts a query string supporting three modes:
/// - Exact match: `torch`
/// - Wildcard: `*torch*` (glob-style contains)
/// - Version pinning: `torch==2.10` (CUDA-aware base version match)
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindPackageParams {
    #[schemars(
        description = "Package name or pattern. Supports wildcards (*torch*) and version pinning (torch==2.10). CUDA-aware: 'torch==2.10' matches '2.10.0+cu130'"
    )]
    pub query: String,
}

/// Parameters for the `get_package_details` MCP tool.
///
/// Retrieves full installation metadata for a single package
/// in a specific environment using L4 filesystem scan.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PackageDetailsParams {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(description = "Package name to get details for")]
    pub package: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LabelParams {
    #[schemars(description = "Name of the environment")]
    pub env_name: EnvName,
    #[schemars(description = "Label to add or remove (e.g., ml, dev, testing)")]
    pub label: String,
}

/// The Zen MCP Server.
#[derive(Clone)]
pub struct ZenMcpServer {
    db: Arc<Mutex<Database>>,
    home: PathBuf,
    tool_router: ToolRouter<Self>,
}

impl ZenMcpServer {
    /// Creates a new ZenMcpServer instance.
    pub fn new(db: Database, home: PathBuf) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            home,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ZenMcpServer {
    #[tool(description = "Get the version of the running Zen server")]
    fn get_version(&self) -> String {
        format!("zen {}", env!("ZEN_VERSION"))
    }

    #[tool(
        description = "List all managed Python environments with their Python versions and paths"
    )]
    fn list_environments(&self, Parameters(params): Parameters<ListEnvironmentsParams>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.list_envs() {
            Ok(envs) => {
                let mut output = String::new();
                for (name, path, py_ver, ..) in &envs {
                    // Filter by label if specified
                    if let Some(ref label) = params.label {
                        let labels = db.get_labels(name).unwrap_or_default();
                        if !labels.iter().any(|l| l == label) {
                            continue;
                        }
                    }
                    output.push_str(&format!(
                        "• {} (Python {}) - {}\n",
                        name,
                        py_ver,
                        redact_path(path)
                    ));
                }
                if output.is_empty() {
                    if let Some(label) = params.label {
                        format!("No environments found with label '{}'", label)
                    } else {
                        "No environments found.".to_string()
                    }
                } else {
                    output
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Create a new Python virtual environment")]
    fn create_environment(
        &self,
        Parameters(params): Parameters<CreateEnvironmentParams>,
    ) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.create_env(&params.name, params.python) {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Install packages into an environment using pip/uv. Supports: PyPI packages ['numpy', 'pandas>=2.0'], local wheels ['/path/to/package.whl'], editable installs (editable=true), CUDA PyTorch (use index_url='https://download.pytorch.org/whl/cu130'), pre-release (pre=true), upgrade (upgrade=true)"
    )]
    fn install_packages(&self, Parameters(params): Parameters<InstallPackagesParams>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

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
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Uninstall packages from an environment")]
    fn uninstall_packages(
        &self,
        Parameters(params): Parameters<UninstallPackagesParams>,
    ) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.uninstall_packages(&params.env_name, params.packages.clone()) {
            Ok(msg) => {
                crate::activity_log::log_activity(
                    "mcp",
                    "uninstall",
                    &format!("{} {}", params.env_name.as_str(), params.packages.join(" ")),
                );
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Remove an environment from the database and delete it from disk")]
    fn remove_environment(&self, Parameters(params): Parameters<EnvNameParam>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match crate::types::EnvName::new(params.env_name.to_string()) {
            Ok(name) => match ops.remove_env(&name) {
                Ok(msg) => {
                    crate::activity_log::log_activity("mcp", "rm", name.as_str());
                    msg
                }
                Err(e) => format!("Error: {}", e),
            },
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Run a command inside an environment without activating it. Returns stdout/stderr output (capped at 10KB). Example: command=['python', '-c', 'import torch; print(torch.__version__)']"
    )]
    fn run_in_environment(&self, Parameters(params): Parameters<RunInEnvironmentParams>) -> String {
        let db = self.db.lock().unwrap();

        // Run in a separate thread with a timeout to prevent blocking the MCP server
        let env_name = params.env_name.clone();
        let command = params.command;

        // Resolve the environment path before spawning the thread
        let envs = match db.list_envs() {
            Ok(e) => e,
            Err(e) => return format!("Error: {}", e),
        };
        let env_entry = envs.iter().find(|(n, ..)| n == env_name.as_str());
        let env_path = match env_entry {
            Some((_, path, ..)) => path.clone(),
            None => return format!("Error: Environment '{}' not found", env_name),
        };
        drop(db); // Release the mutex before spawning

        let timeout_secs = params.timeout.unwrap_or(120);
        let cwd = params.cwd;

        let handle = std::thread::spawn(move || {
            // Build and run the command directly (mirrors ops.run_in_env logic)
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

            // Use spawn + wait for timeout support
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
                // No timeout — wait indefinitely
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
                Ok((exit_code, combined))
            } else {
                // Poll with timeout
                let deadline =
                    std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // Process finished
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
                            return Ok((exit_code, combined));
                        }
                        Ok(None) => {
                            // Still running
                            if std::time::Instant::now() >= deadline {
                                let _ = child.kill();
                                return Err(format!("Command timed out after {}s", timeout_secs));
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(e) => return Err(format!("Error waiting for process: {}", e)),
                    }
                }
            }
        });

        match handle.join() {
            Ok(Ok((code, output))) => {
                let mut result = if output.len() > 10240 {
                    format!("{}\n... (output truncated to 10KB)", &output[..10240])
                } else {
                    output
                };
                if code != 0 {
                    result.push_str(&format!("\n[exit code: {}]", code));
                }
                result
            }
            Ok(Err(e)) => format!("Error: {}", e),
            Err(_) => "Error: Command execution panicked".to_string(),
        }
    }

    #[tool(description = "Link an environment to a project directory for context-aware activation")]
    fn associate_project(&self, Parameters(params): Parameters<AssociateProjectParams>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.associate_project(
            &params.project_path,
            &params.env_name,
            params.tag.as_deref(),
            params.is_default.unwrap_or(false),
        ) {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get the default environment for a project")]
    fn get_default_environment(&self, Parameters(params): Parameters<ProjectPathParam>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.get_default_env(&params.project_path) {
            Ok(Some(env)) => format!("Default environment: {}", env),
            Ok(None) => "No default environment set for this project".to_string(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get all environments associated with a project")]
    fn get_project_environments(&self, Parameters(params): Parameters<ProjectPathParam>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.get_project_envs(&params.project_path) {
            Ok(envs) => {
                let list: Vec<String> = envs
                    .into_iter()
                    .map(|(name, _path, tag, is_default)| {
                        let tag_str = tag.map(|t| format!(" [{}]", t)).unwrap_or_default();
                        let default = if is_default { " (DEFAULT)" } else { "" };
                        format!("• {}{}{}", name, tag_str, default)
                    })
                    .collect();
                if list.is_empty() {
                    "No environments associated with this project".to_string()
                } else {
                    list.join("\n")
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Get detailed information about an environment including Python version, packages, ML frameworks"
    )]
    fn get_environment_details(&self, Parameters(params): Parameters<EnvNameParam>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.list_envs() {
            Ok(envs) => {
                let env = envs.iter().find(|(n, ..)| n == params.env_name.as_str());
                match env {
                    Some((name, path, py_ver, ..)) => {
                        let packages = crate::utils::get_packages(path);

                        let mut details = format!("# Environment: {}\n\n", name);
                        details.push_str(&format!("**Python**: {}\n", py_ver));
                        details.push_str(&format!("**Path**: {}\n", redact_path(path)));
                        details.push_str(&format!("**Packages**: {}\n", packages.len()));
                        if let Some(epoch) = crate::utils::get_env_created_at(path) {
                            use chrono::{Local, TimeZone};
                            if let Some(dt) = Local.timestamp_opt(epoch, 0).single() {
                                details.push_str(&format!(
                                    "**Created**: {}\n",
                                    dt.format("%Y-%m-%d %H:%M")
                                ));
                            }
                        }
                        details.push('\n');

                        // Torch version from version.py (accurate CUDA suffix)
                        if let Some((torch, cuda)) = crate::utils::read_torch_version(path) {
                            details.push_str(&format!("**PyTorch**: {}\n", torch));
                            if let Some(c) = cuda {
                                details.push_str(&format!("**CUDA**: {}\n", c));
                            }
                        }
                        // Other key packages from scan
                        let get_ver = |name: &str| {
                            packages
                                .iter()
                                .find(|p| p.name == name)
                                .and_then(|p| p.version.clone())
                        };
                        if let Some(v) = get_ver("numpy") {
                            details.push_str(&format!("**NumPy**: {}\n", v));
                        }
                        details
                    }
                    None => format!("Environment '{}' not found", params.env_name),
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Check environment health: package conflicts, outdated dependencies")]
    fn get_environment_health(&self, Parameters(params): Parameters<EnvNameParam>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.check_health(&params.env_name) {
            Ok(report) => report.to_text(&params.env_name),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Compare multiple environments side-by-side")]
    fn compare_environments(
        &self,
        Parameters(params): Parameters<CompareEnvironmentsParams>,
    ) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        if params.env_names.len() < 2 {
            return "At least two environment names are required".to_string();
        }

        match ops.list_envs() {
            Ok(all_envs) => {
                let mut comparison = format!("# Comparison: {}\n\n", params.env_names.join(" vs "));

                // Collect package maps for each env
                let mut env_packages: Vec<(
                    String,
                    String,
                    std::collections::HashMap<String, String>,
                )> = Vec::new();
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
                        comparison.push_str(&format!(
                            "## {}\n- Python: {}\n- Packages: {}\n\n",
                            name,
                            py_ver,
                            pkg_map.len()
                        ));
                        env_packages.push((name.clone(), py_ver.clone(), pkg_map));
                    } else {
                        comparison.push_str(&format!("## {}\n- Not found\n\n", name));
                    }
                }

                // Deep diff between first two envs
                if env_packages.len() >= 2 {
                    let (ref n1, _, ref pkgs1) = env_packages[0];
                    let (ref n2, _, ref pkgs2) = env_packages[1];

                    // Common packages with different versions
                    let mut diffs: Vec<(String, String, String)> = Vec::new();
                    for (name, v1) in pkgs1 {
                        if let Some(v2) = pkgs2.get(name)
                            && v1 != v2
                        {
                            diffs.push((name.clone(), v1.clone(), v2.clone()));
                        }
                    }
                    if !diffs.is_empty() {
                        diffs.sort_by(|a, b| a.0.cmp(&b.0));
                        comparison.push_str("## Version differences\n\n");
                        comparison.push_str(&format!("| Package | {} | {} |\n", n1, n2));
                        comparison.push_str("|---------|------|------|\n");
                        for (name, v1, v2) in &diffs {
                            comparison.push_str(&format!("| {} | {} | {} |\n", name, v1, v2));
                        }
                        comparison.push('\n');
                    }

                    // Only in env1
                    let mut only1: Vec<String> = pkgs1
                        .keys()
                        .filter(|k| !pkgs2.contains_key(*k))
                        .cloned()
                        .collect();
                    only1.sort();
                    if !only1.is_empty() {
                        comparison.push_str(&format!(
                            "## Only in {}\n{}\n\n",
                            n1,
                            only1.join(", ")
                        ));
                    }

                    // Only in env2
                    let mut only2: Vec<String> = pkgs2
                        .keys()
                        .filter(|k| !pkgs1.contains_key(*k))
                        .cloned()
                        .collect();
                    only2.sort();
                    if !only2.is_empty() {
                        comparison.push_str(&format!(
                            "## Only in {}\n{}\n\n",
                            n2,
                            only2.join(", ")
                        ));
                    }
                }

                comparison
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get notes attached to an environment (purpose, description, reminders)")]
    fn get_environment_notes(&self, Parameters(params): Parameters<EnvNameParam>) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.list_comments(None, Some(&params.env_name)) {
            Ok(comments) => {
                if comments.is_empty() {
                    return format!("No notes for environment '{}'", params.env_name);
                }
                let mut output = format!("Notes for '{}':\n", params.env_name);
                for (_uuid, _pp, _env, msg, _tag, ts) in comments {
                    output.push_str(&format!("[{}] {}\n", ts, msg));
                }
                output
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Add a note to an environment (purpose, description, reminder)")]
    fn add_environment_note(
        &self,
        Parameters(params): Parameters<AddEnvironmentNoteParams>,
    ) -> String {
        let db = self.db.lock().unwrap();
        let ops = crate::ops::ZenOps::new(&db, self.home.clone());

        match ops.add_env_note(&params.env_name, &params.note) {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Search for a package across all environments (substring match). For wildcards or version matching, use find_package instead."
    )]
    fn search_packages(&self, Parameters(params): Parameters<SearchPackagesParams>) -> String {
        let db = self.db.lock().unwrap();
        match db.list_envs() {
            Ok(envs) => {
                let mut results = Vec::new();
                for (name, path, ..) in &envs {
                    let packages = crate::utils::get_packages(path);
                    for pkg in packages {
                        if pkg
                            .name
                            .to_lowercase()
                            .contains(&params.query.to_lowercase())
                        {
                            let ver = pkg.version.unwrap_or_else(|| "?".to_string());
                            results.push(format!("• {} → {} ({})", name, pkg.name, ver));
                        }
                    }
                }
                if results.is_empty() {
                    format!("No packages matching '{}' found", params.query)
                } else {
                    format!(
                        "Packages matching '{}':\n{}",
                        params.query,
                        results.join("\n")
                    )
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Find a package across all environments. Supports wildcards (*torch*) and version matching (torch==2.10). CUDA-aware: queries without +cuXXX match base version."
    )]
    fn find_package(&self, Parameters(params): Parameters<FindPackageParams>) -> String {
        let db = self.db.lock().unwrap();

        // Split query into name and optional version at "=="
        let (pkg_query, version_query) = if params.query.contains("==") {
            let parts: Vec<&str> = params.query.split("==").collect();
            (
                parts[0].to_string(),
                Some(parts.get(1).unwrap_or(&"").to_string()),
            )
        } else {
            (params.query.clone(), None)
        };

        // Default to substring matching (strip any legacy glob chars)
        // pip treats hyphens and underscores as equivalent
        let normalize = |s: &str| s.to_lowercase().replace('-', "_");
        let pattern = normalize(&pkg_query.replace('*', ""));

        match db.list_envs() {
            Ok(envs) => {
                let mut found = Vec::new();
                for (name, path, ..) in &envs {
                    let packages = crate::utils::get_packages(path);
                    for pkg in packages {
                        let pkg_norm = normalize(&pkg.name);

                        // Substring match by default
                        let name_match = pkg_norm.contains(&pattern);

                        let version_match = match (&version_query, &pkg.version) {
                            (Some(q), Some(v)) => {
                                // Version query with "+" requires exact match (e.g., "2.10.0+cu130")
                                // Without "+", match base version before the CUDA suffix
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
                            let ver = pkg.version.unwrap_or_else(|| "?".to_string());
                            found.push(format!("• {} → {} ({})", name, pkg.name, ver));
                        }
                    }
                }
                if found.is_empty() {
                    format!("No packages matching '{}' found", params.query)
                } else {
                    format!(
                        "Found {} match(es) for '{}':\n{}",
                        found.len(),
                        params.query,
                        found.join("\n")
                    )
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Get detailed info about a specific package in an environment: version, installer (pip/uv), source (pypi/git/local), editable status, source URL, git commit. Similar to pip show."
    )]
    fn get_package_details(&self, Parameters(params): Parameters<PackageDetailsParams>) -> String {
        let db = self.db.lock().unwrap();

        match db.list_envs() {
            Ok(envs) => {
                let env = envs.iter().find(|(n, ..)| n == params.env_name.as_str());
                match env {
                    Some((name, path, ..)) => {
                        // Full package scan: METADATA + INSTALLER + direct_url.json
                        let packages = crate::utils::get_packages(path);
                        let pkg_lower = params.package.to_lowercase();
                        let found = packages
                            .into_iter()
                            .find(|p| p.name.to_lowercase() == pkg_lower);

                        match found {
                            Some(pkg) => {
                                let mut details = format!("# {} in '{}'\n\n", pkg.name, name);
                                details.push_str(&format!(
                                    "**Version**: {}\n",
                                    pkg.version.as_deref().unwrap_or("unknown")
                                ));
                                details.push_str(&format!(
                                    "**Installer**: {}\n",
                                    pkg.installer.as_deref().unwrap_or("unknown")
                                ));
                                details.push_str(&format!(
                                    "**Source**: {}\n",
                                    pkg.install_source.as_deref().unwrap_or("unknown")
                                ));
                                details.push_str(&format!(
                                    "**Editable**: {}\n",
                                    if pkg.is_editable { "yes" } else { "no" }
                                ));
                                if let Some(url) = &pkg.source_url {
                                    details.push_str(&format!("**URL**: {}\n", url));
                                }
                                if let Some(commit) = &pkg.commit_id {
                                    details.push_str(&format!("**Commit**: {}\n", commit));
                                }
                                // Import name: only show when pip name is NOT in top_level.txt
                                if let Some(ref import) = pkg.import_name {
                                    details.push_str(&format!("**Import**: `{}`\n", import));
                                }
                                if let Some(epoch) = pkg.installed_at {
                                    use chrono::{Local, TimeZone};
                                    if let Some(dt) = Local.timestamp_opt(epoch, 0).single() {
                                        details.push_str(&format!(
                                            "**Installed**: {}\n",
                                            dt.format("%Y-%m-%d %H:%M")
                                        ));
                                    }
                                }
                                details
                            }
                            None => format!(
                                "Package '{}' not found in environment '{}'",
                                params.package, name
                            ),
                        }
                    }
                    None => format!("Environment '{}' not found", params.env_name),
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Add a label to an environment (e.g., ml, dev, testing)")]
    fn add_label(&self, Parameters(params): Parameters<LabelParams>) -> String {
        let db = self.db.lock().unwrap();
        match db.add_label(&params.env_name, &params.label) {
            Ok(_) => format!("Added label '{}' to '{}'", params.label, params.env_name),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Remove a label from an environment")]
    fn remove_label(&self, Parameters(params): Parameters<LabelParams>) -> String {
        let db = self.db.lock().unwrap();
        match db.remove_label(&params.env_name, &params.label) {
            Ok(_) => format!(
                "Removed label '{}' from '{}'",
                params.label, params.env_name
            ),
            Err(e) => format!("Error: {}", e),
        }
    }
}

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
