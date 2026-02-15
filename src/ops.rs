// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use crate::types::{Diagnostic, EnvName, HealthDiagnostic, HealthLevel, HealthReport};
use crate::utils;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

/// Main operations layer for Zen.
///
/// This struct coordinates between the database and the filesystem,
/// implementing the core logic for environment creation, package
/// management, parallel scanning, and project context.
pub struct ZenOps<'a> {
    db: &'a Database,
    home: PathBuf,
}

/// Options for package installation (shared by CLI and MCP).
#[derive(Default)]
pub struct InstallOptions {
    pub index_url: Option<String>,
    pub extra_index_url: Option<String>,
    pub pre: bool,
    pub upgrade: bool,
    pub editable: bool,
    pub dry_run: bool,
}

impl<'a> ZenOps<'a> {
    /// Creates a new operational layer instance.
    pub fn new(db: &'a Database, home: PathBuf) -> Self {
        Self { db, home }
    }

    /// Lists all environments from the database
    pub fn list_envs(
        &self,
    ) -> Result<
        Vec<(
            String, // name
            String, // path
            String, // python_version
            String, // updated_at
            bool,   // is_favorite
        )>,
        Box<dyn Error>,
    > {
        self.db.list_envs()
    }

    /// Removes an environment from the database and deletes it from disk.
    pub fn remove_env(&self, name: &EnvName) -> Result<String, Box<dyn Error>> {
        let envs = self.list_envs()?;
        let env = envs.iter().find(|(n, ..)| n == name.as_str());

        if let Some((_, path, ..)) = env {
            let path = PathBuf::from(path);
            if path.exists() {
                std::fs::remove_dir_all(&path)?;
            }
            self.db.delete_env(name)?;
            Ok(format!(
                "{} Environment '{}' removed from disk and registry.",
                "✓".green(),
                name
            ))
        } else {
            // Not in DB — check if orphaned directory exists on disk
            let orphan_path = PathBuf::from(&self.home).join(name.as_str());
            if orphan_path.exists() && orphan_path.is_dir() {
                std::fs::remove_dir_all(&orphan_path)?;
                Ok(format!(
                    "{} Orphaned directory '{}' removed from disk (was not in registry).",
                    "✓".green(),
                    name
                ))
            } else {
                Err(format!("Environment '{}' not found.", name).into())
            }
        }
    }

    /// Removes an environment from the database only, keeping files on disk.
    pub fn untrack_env(&self, name: &EnvName) -> Result<String, Box<dyn Error>> {
        self.db.delete_env(name)?;
        Ok(format!(
            "{} Environment '{}' removed from registry (files kept on disk).",
            "✓".green(),
            name
        ))
    }

    /// Creates a new Python virtual environment and registers it in the database.
    pub fn create_env(
        &self,
        name: &EnvName,
        python: Option<String>,
    ) -> Result<String, Box<dyn Error>> {
        let env_path = self.home.join(name.as_str());
        if env_path.exists() {
            return Err(format!(
                "Environment '{}' already exists at {}",
                name,
                env_path.display()
            )
            .into());
        }

        let py_version = python.unwrap_or_else(|| "3.12".to_string());

        // Simplified creation logic (no templates for MCP MVP yet)
        std::fs::create_dir_all(&self.home)?;

        let status = if let Ok(uv_path) = which::which("uv") {
            std::process::Command::new(uv_path)
                .arg("venv")
                .arg(&env_path)
                .arg("--python")
                .arg(&py_version)
                .output()?
        } else {
            std::process::Command::new("python3")
                .arg("-m")
                .arg("venv")
                .arg(&env_path)
                .output()?
        };

        if !status.status.success() {
            return Err(format!(
                "Failed to create venv: {:?}",
                String::from_utf8_lossy(&status.stderr)
            )
            .into());
        }

        let id = self
            .db
            .register_env(name, env_path.to_str().unwrap(), &py_version)?;
        Ok(format!("Created environment {} (ID: {})", name, id))
    }

    /// Installs packages into an environment using uv or pip.
    ///
    /// Accepts PyPI names, version specs, local wheel paths, and URLs.
    pub fn install_packages(
        &self,
        env_name: &EnvName,
        packages: Vec<String>,
        opts: InstallOptions,
    ) -> Result<String, Box<dyn Error>> {
        let envs = self.db.list_envs()?;
        let (_, env_path, ..) = envs
            .iter()
            .find(|(n, ..)| n == env_name.as_str())
            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;

        let mut args: Vec<String> = vec!["pip".into(), "install".into()];

        if opts.editable {
            args.push("-e".into());
        }
        if opts.pre {
            args.push("--pre".into());
        }
        if opts.upgrade {
            args.push("--upgrade".into());
        }
        if opts.dry_run {
            args.push("--dry-run".into());
        }
        if let Some(ref url) = opts.index_url {
            args.push("--index-url".into());
            args.push(url.clone());
        }
        if let Some(ref url) = opts.extra_index_url {
            args.push("--extra-index-url".into());
            args.push(url.clone());
        }

        for pkg in &packages {
            args.push(pkg.clone());
        }

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let success = if which::which("uv").is_ok() {
            utils::run_in_env_silent(env_path, "uv", &arg_refs)
        } else {
            utils::run_in_env_silent(env_path, "pip", &arg_refs[1..])
        };

        if success {
            let env_id = self.db.get_env_id(env_name)?.unwrap();
            let installed = utils::get_packages(env_path);
            for pkg_name in &packages {
                // Skip file paths and URLs for DB logging — extract base name
                let base_name = if pkg_name.ends_with(".whl") || pkg_name.contains('/') {
                    // Try to find the installed name from the actual packages scan
                    continue;
                } else if pkg_name.starts_with("torch-cu") {
                    "torch"
                } else {
                    pkg_name.as_str()
                };
                if let Some(pkg) = installed.iter().find(|p| p.name == base_name) {
                    let ver = pkg.version.as_deref().unwrap_or("unknown");
                    self.db.log_package(env_id, &pkg.name, ver, "pypi")?;
                }
            }
            Ok(format!("Successfully installed: {:?}", packages))
        } else {
            Err("Installation failed".into())
        }
    }

    /// Uninstalls packages from an environment using uv or pip.
    pub fn uninstall_packages(
        &self,
        env_name: &EnvName,
        packages: Vec<String>,
    ) -> Result<String, Box<dyn Error>> {
        let envs = self.db.list_envs()?;
        let (_, env_path, ..) = envs
            .iter()
            .find(|(n, ..)| n == env_name.as_str())
            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;

        let mut args: Vec<String> = vec!["pip".into(), "uninstall".into()];
        for pkg in &packages {
            args.push(pkg.clone());
        }

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let success = if which::which("uv").is_ok() {
            utils::run_in_env_silent(env_path, "uv", &arg_refs)
        } else {
            // pip needs -y for non-interactive
            let mut pip_args: Vec<String> = vec!["uninstall".into(), "-y".into()];
            for pkg in &packages {
                pip_args.push(pkg.clone());
            }
            let pip_refs: Vec<&str> = pip_args.iter().map(|s| s.as_str()).collect();
            utils::run_in_env_silent(env_path, "pip", &pip_refs)
        };

        if success {
            Ok(format!("Successfully uninstalled: {:?}", packages))
        } else {
            Err("Uninstall failed".into())
        }
    }

    /// Runs a command inside an environment, returning (exit_code, combined_output).
    pub fn run_in_env(
        &self,
        env_name: &EnvName,
        cmd: Vec<String>,
    ) -> Result<(i32, String), Box<dyn Error>> {
        if cmd.is_empty() {
            return Err("No command specified".into());
        }
        let envs = self.db.list_envs()?;
        let (_, env_path, ..) = envs
            .iter()
            .find(|(n, ..)| n == env_name.as_str())
            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;

        let env_path = std::path::Path::new(env_path);
        let bin_path = env_path.join("bin");
        let exe_path = bin_path.join(&cmd[0]);

        let program = if exe_path.exists() {
            exe_path.to_string_lossy().to_string()
        } else {
            cmd[0].clone()
        };

        let path = std::env::var("PATH").unwrap_or_default();
        let output = std::process::Command::new(&program)
            .args(&cmd[1..])
            .env("PATH", format!("{}:{}", bin_path.display(), path))
            .env("VIRTUAL_ENV", env_path)
            .output()?;

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
    }

    /// Associates a project directory with an environment.
    pub fn associate_project(
        &self,
        project_path: &str,
        env_name: &EnvName,
        tag: Option<&str>,
        is_default: bool,
    ) -> Result<String, Box<dyn Error>> {
        self.db
            .associate_project(project_path, env_name, tag, is_default)?;
        Ok(format!(
            "Associated '{}' with project {} (tag: {:?}, default: {})",
            env_name, project_path, tag, is_default
        ))
    }

    /// Returns all environments associated with a project path.
    ///
    /// Returns a vector of (env_name, env_path, tag, is_default) tuples.
    pub fn get_project_envs(
        &self,
        project_path: &str,
    ) -> Result<Vec<(String, String, Option<String>, bool)>, Box<dyn Error>> {
        self.db.get_project_environments(project_path)
    }

    /// Returns the default environment name for a given project path.
    pub fn get_default_env(&self, project_path: &str) -> Result<Option<String>, Box<dyn Error>> {
        self.db.get_default_environment(project_path)
    }

    /// Looks up an environment name by its filesystem path.
    #[allow(dead_code)]
    pub fn get_env_by_path(&self, path_str: &str) -> Result<Option<String>, Box<dyn Error>> {
        self.db.get_env_name_by_path(path_str)
    }

    /// Infers the current environment from the active VIRTUAL_ENV path.
    ///
    /// Checks the VIRTUAL_ENV environment variable and matches it against
    /// registered environments to return the corresponding name.
    pub fn infer_current_env(&self) -> Result<Option<String>, Box<dyn Error>> {
        let venv_path = match utils::get_current_venv_path() {
            Some(p) => p,
            None => return Ok(None),
        };

        // Find env name in DB by path
        let envs = self.db.list_envs()?;
        for (name, path, ..) in envs {
            if path == venv_path {
                return Ok(Some(name));
            }
        }

        Ok(None)
    }

    /// Logs a comment to an environment or the current project.
    pub fn log_comment(
        &self,
        env_name: Option<&EnvName>,
        message: &str,
    ) -> Result<String, Box<dyn Error>> {
        let project_path = std::env::current_dir()?.to_str().unwrap_or(".").to_string();
        let uuid = Uuid::new_v4().to_string();

        let (env_id, tag) = if let Some(name) = env_name {
            (self.db.get_env_id(name)?, format!("Env: {}", name))
        } else {
            (None, "General".to_string())
        };

        self.db
            .add_comment(&uuid, &project_path, env_id, message, Some(&tag))?;

        let msg = if let Some(name) = env_name {
            format!(
                "{} comment logged to {} history (UUID: {}).",
                "✓".green(),
                name.bold().cyan(),
                uuid.dimmed()
            )
        } else {
            format!(
                "{} comment logged to project history (UUID: {}).",
                "✓".green(),
                uuid.dimmed()
            )
        };
        Ok(msg)
    }

    /// Lists all comments for a project or environment.
    pub fn list_comments(
        &self,
        project_path: Option<&str>,
        env_name: Option<&EnvName>,
    ) -> Result<
        Vec<(
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
        )>,
        Box<dyn Error>,
    > {
        let env_id = if let Some(name) = env_name {
            Some(self.db.get_env_id(name)?.ok_or("Environment not found")?)
        } else {
            None
        };

        let db_comments = self.db.list_comments(project_path, env_id)?;
        let mut results = Vec::new();

        for (uuid, pp, eid, msg, tag, ts) in db_comments {
            let env_display_name = if let Some(id) = eid {
                self.db.get_env_name_by_id(id)?
            } else {
                None
            };
            results.push((uuid, pp, env_display_name, msg, tag, ts));
        }

        Ok(results)
    }

    /// Removes a comment by its UUID prefix. Returns count of deleted.
    pub fn remove_comment(&self, uuid_prefix: &str) -> Result<usize, Box<dyn Error>> {
        let deleted = self.db.remove_comment(uuid_prefix)?;
        Ok(deleted)
    }

    /// Adds a note to an environment for tracking purpose/description.
    pub fn add_env_note(&self, env_name: &EnvName, note: &str) -> Result<String, Box<dyn Error>> {
        let env_id = self
            .db
            .get_env_id(env_name)?
            .ok_or("Environment not found")?;
        let uuid = Uuid::new_v4().to_string();
        self.db.add_comment(
            &uuid,
            "", // Empty project_path since this is env-centric
            Some(env_id),
            note,
            Some("note"),
        )?;
        Ok(format!("Note added to environment '{}'", env_name))
    }

    /// Lists all environments and verifies their existence on the local filesystem.
    ///
    /// Returns a tuple of (name, path, python_version, exists, updated_at, is_favorite).
    pub fn list_envs_with_status(
        &self,
        filter: Option<&str>,
        sort_by: Option<&str>,
        limit: Option<usize>,
    ) -> Result<
        Vec<(
            String, // name
            String, // path
            String, // python_version
            bool,   // exists
            String, // updated_at
            bool,   // is_favorite
        )>,
        Box<dyn Error>,
    > {
        let mut envs = self.db.list_envs()?;

        // FILTERING (substring match — consistent with `zen find`)
        if let Some(pattern) = filter {
            let pattern_lower = pattern.to_lowercase();
            envs.retain(|(name, ..)| name.to_lowercase().contains(&pattern_lower));
        }

        // SORTING (Favorites always first, then requested order)
        envs.sort_by(|a, b| {
            // First by favorite status (true comes first)
            match b.4.cmp(&a.4) {
                std::cmp::Ordering::Equal => {
                    // Then by requested field
                    match sort_by {
                        Some("date") => b.3.cmp(&a.3),
                        _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
                    }
                }
                other => other,
            }
        });

        // LIMITING
        if let Some(n) = limit {
            envs.truncate(n);
        }

        let mut results = Vec::new();
        for (name, path, py_ver, updated, is_fav) in envs {
            let exists = Path::new(&path).join("bin").join("python").exists();
            results.push((name, path, py_ver, exists, updated, is_fav));
        }

        Ok(results)
    }

    /// Bulk imports multiple environments with parallel scanning.
    pub fn bulk_import(&self, paths: Vec<PathBuf>) -> Result<String, Box<dyn Error>> {
        let m = MultiProgress::new();
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap();

        let results: Vec<_> = paths
            .par_iter()
            .map(|path| {
                let pb = m.add(ProgressBar::new_spinner());
                pb.set_style(style.clone());
                pb.enable_steady_tick(Duration::from_millis(100));

                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let python_bin = path.join("bin").join("python");
                if !python_bin.exists() {
                    pb.finish_with_message(format!("{} {} (skip — no python)", "⊘".dimmed(), name));
                    return (name, path.clone(), false);
                }

                pb.set_message(format!("Scanning {}", name));

                // Get real python version from pyvenv.cfg
                let py_ver =
                    utils::read_python_version(path).unwrap_or_else(|| "unknown".to_string());

                let path_str = path.to_str().unwrap_or_default();

                match self.db.register_env(&name, path_str, &py_ver) {
                    Ok(_env_id) => {
                        // Full package scan
                        let packages = utils::get_packages(path);
                        let _versions: std::collections::HashMap<String, Option<String>> = packages
                            .iter()
                            .map(|p| (p.name.clone(), p.version.clone()))
                            .collect();

                        let torch_info =
                            if let Some(pkg) = packages.iter().find(|p| p.name == "torch") {
                                let ver = pkg.version.as_deref().unwrap_or("?");
                                format!(" torch={}", ver.green())
                            } else {
                                String::new()
                            };

                        pb.finish_with_message(format!(
                            "{} {} — py {} {} pkgs{}",
                            "✓".green(),
                            name.bold(),
                            py_ver,
                            packages.len(),
                            torch_info
                        ));
                        (name, path.clone(), true)
                    }
                    Err(e) => {
                        pb.finish_with_message(format!("{} {} (error: {})", "✗".red(), name, e));
                        (name, path.clone(), false)
                    }
                }
            })
            .collect();

        let imported = results.iter().filter(|(_, _, ok)| *ok).count();
        let skipped = results.len() - imported;

        Ok(format!(
            "\n{} Imported {} environment{}, skipped {}.",
            "✓".green(),
            imported,
            if imported == 1 { "" } else { "s" },
            skipped
        ))
    }

    /// Generates a full summary of the system state for AI context.
    #[allow(dead_code)]
    pub fn get_system_summary(&self) -> Result<String, Box<dyn Error>> {
        let envs = self.db.list_envs()?;
        let mut out = format!("Zen v{}\n", env!("CARGO_PKG_VERSION"));
        out.push_str(&format!("Registered environments: {}\n", envs.len()));

        for (name, path, py_ver, ..) in &envs {
            let pkg_count = utils::get_packages(path).len();
            out.push_str(&format!(
                "  {} — py {} ({} pkgs)\n",
                name, py_ver, pkg_count
            ));
        }

        // Active env from VIRTUAL_ENV
        if let Some(venv) = utils::get_current_venv_path() {
            out.push_str(&format!("Active: {}\n", venv));
        }

        Ok(out)
    }

    /// Runs a full health check on an environment.
    ///
    /// Checks: python binary, site-packages, CUDA consistency, dependency conflicts.
    pub fn check_health(&self, env_name: &EnvName) -> Result<HealthReport, Box<dyn Error>> {
        let envs = self.db.list_envs()?;
        let (_, path, ..) = envs
            .iter()
            .find(|(n, ..)| n == env_name.as_str())
            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;

        let env_path = std::path::Path::new(path);
        let mut report = HealthReport::default();

        // 1. Python binary exists and is executable
        let python_bin = env_path.join("bin/python");
        if python_bin.exists() {
            if python_bin.is_symlink() {
                match std::fs::read_link(&python_bin) {
                    Ok(target) => {
                        if target.exists() || env_path.join("bin").join(&target).exists() {
                            let ver = utils::read_python_version(env_path)
                                .unwrap_or_else(|| "unknown".to_string());
                            report.push(HealthDiagnostic::PythonOk { version: ver });
                        } else {
                            report.push(HealthDiagnostic::BrokenSymlink { target });
                        }
                    }
                    Err(_) => report.push(HealthDiagnostic::PythonMissing),
                }
            } else {
                let ver =
                    utils::read_python_version(env_path).unwrap_or_else(|| "unknown".to_string());
                report.push(HealthDiagnostic::PythonOk { version: ver });
            }
        } else {
            report.push(HealthDiagnostic::PythonMissing);
        }

        // 2. site-packages directory exists
        if utils::get_site_packages_path(env_path).is_some() {
            report.push(HealthDiagnostic::SitePackagesOk);
        } else {
            report.push(HealthDiagnostic::SitePackagesMissing);
        }

        // 3. Package scan + CUDA version consistency
        let packages = utils::get_packages(path);

        let mut cuda_versions: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for pkg in &packages {
            if let Some(ver) = &pkg.version
                && let Some(plus_pos) = ver.find('+')
            {
                let suffix = &ver[plus_pos + 1..];
                if suffix.starts_with("cu") || suffix == "cpu" {
                    cuda_versions
                        .entry(suffix.to_string())
                        .or_default()
                        .push(format!("{}=={}", pkg.name, ver));
                }
            }
        }

        if cuda_versions.len() > 1 {
            let has_cpu_and_cuda = cuda_versions.contains_key("cpu")
                && cuda_versions.keys().any(|k| k.starts_with("cu"));
            let cuda_only: Vec<_> = cuda_versions
                .keys()
                .filter(|k| k.starts_with("cu"))
                .collect();
            let has_mixed_cuda = cuda_only.len() > 1;

            if has_cpu_and_cuda || has_mixed_cuda {
                let mut detail = String::from("Mixed CUDA versions detected:");
                for (suffix, pkgs) in &cuda_versions {
                    detail.push_str(&format!("\n    +{}: {}", suffix, pkgs.join(", ")));
                }
                if has_mixed_cuda {
                    report.push(HealthDiagnostic::CudaMismatch { details: detail });
                } else {
                    report.push(HealthDiagnostic::CpuCudaConflict { details: detail });
                }
            }
        } else if cuda_versions.len() == 1 {
            let (suffix, _) = cuda_versions.iter().next().unwrap();
            report.push(HealthDiagnostic::CudaConsistent {
                suffix: suffix.clone(),
            });
        }

        // 4. Native dependency check (no subprocess — learned from pip & uv)
        let dep_issues = utils::check_dependencies(env_path);
        if dep_issues.is_empty() {
            report.push(HealthDiagnostic::DependenciesOk);
        } else {
            // Separate missing (info) from incompatible (warn)
            let missing: Vec<_> = dep_issues
                .iter()
                .filter(|i| matches!(i, utils::DepIssue::Missing { .. }))
                .collect();
            let conflicts: Vec<_> = dep_issues
                .iter()
                .filter(|i| !matches!(i, utils::DepIssue::Missing { .. }))
                .collect();

            if !conflicts.is_empty() {
                let mut detail = String::new();
                for (i, issue) in conflicts.iter().take(10).enumerate() {
                    if i > 0 {
                        detail.push('\n');
                    }
                    detail.push_str(&format!("    {}", issue.message()));
                }
                if conflicts.len() > 10 {
                    detail.push_str(&format!("\n    ... and {} more", conflicts.len() - 10));
                }
                report.push(HealthDiagnostic::VersionConflicts {
                    count: conflicts.len(),
                    details: detail,
                });
            }
            if !missing.is_empty() {
                let mut detail = String::new();
                for (i, issue) in missing.iter().take(5).enumerate() {
                    if i > 0 {
                        detail.push('\n');
                    }
                    detail.push_str(&format!("    {}", issue.message()));
                }
                if missing.len() > 5 {
                    detail.push_str(&format!("\n    ... and {} more", missing.len() - 5));
                }
                report.push(HealthDiagnostic::MissingDependencies {
                    count: missing.len(),
                    details: detail,
                });
            }
        }

        Ok(report)
    }
}

/// Quick health check on an environment path — returns just the overall level.
///
/// Used by `zen list` for inline health indicators. No DB access needed.
/// Checks: python binary, CUDA consistency, dependency conflicts.
pub fn check_health_quick(env_path: &std::path::Path) -> HealthLevel {
    // 1. Python binary
    let python_bin = env_path.join("bin/python");
    if !python_bin.exists() {
        return HealthLevel::Fail;
    }
    if python_bin.is_symlink()
        && let Ok(target) = std::fs::read_link(&python_bin)
        && !target.exists()
        && !env_path.join("bin").join(&target).exists()
    {
        return HealthLevel::Fail;
    }

    // 2. site-packages
    if utils::get_site_packages_path(env_path).is_none() {
        return HealthLevel::Fail;
    }

    // 3. CUDA consistency (fast — uses already-scanned packages)
    let packages = utils::get_packages(env_path);
    let mut cuda_suffixes: std::collections::HashSet<String> = std::collections::HashSet::new();
    for pkg in &packages {
        if let Some(ver) = &pkg.version
            && let Some(plus_pos) = ver.find('+')
        {
            let suffix = &ver[plus_pos + 1..];
            if suffix.starts_with("cu") || suffix == "cpu" {
                cuda_suffixes.insert(suffix.to_string());
            }
        }
    }
    let has_mixed_cuda = cuda_suffixes.iter().filter(|s| s.starts_with("cu")).count() > 1;
    let has_cpu_and_cuda =
        cuda_suffixes.contains("cpu") && cuda_suffixes.iter().any(|s| s.starts_with("cu"));

    // 4. Dependency check — categorize by severity
    let dep_issues = utils::check_dependencies(env_path);
    let has_conflicts = dep_issues
        .iter()
        .any(|i| !matches!(i, utils::DepIssue::Missing { .. }));
    let has_missing = dep_issues
        .iter()
        .any(|i| matches!(i, utils::DepIssue::Missing { .. }));

    if has_mixed_cuda || has_cpu_and_cuda || has_conflicts {
        HealthLevel::Warn
    } else if has_missing {
        HealthLevel::Info
    } else {
        HealthLevel::Pass
    }
}
