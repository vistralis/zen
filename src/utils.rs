// SPDX-License-Identifier: Apache-2.0

//! Utility functions for Zen — environment inspection and package scanning.
//!
//! Public API:
//!   - `get_packages(path)`       → Full package list with source/editable info
//!   - `read_python_version(path)` → Python version from pyvenv.cfg
//!   - `read_torch_version(path)`  → Torch version + CUDA from version.py
//!   - `normalize_package_name(s)` → pip-compatible name normalization

use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// PACKAGE SCANNING
// =============================================================================

/// Returns all installed packages in an environment with full metadata.
///
/// Reads `.dist-info` directories in site-packages to extract:
/// - Name and version (from METADATA)
/// - Installer (from INSTALLER — pip/uv)
/// - Source info (from direct_url.json — pypi/git/local, editable, commit)
///
/// Typical speed: ~4ms for 200 packages.
pub fn get_packages(env_path: impl AsRef<Path>) -> Vec<crate::db::PackageMetadata> {
    let mut result = Vec::new();
    let site_packages = match get_site_packages_path(env_path.as_ref()) {
        Some(p) => p,
        None => return result,
    };

    if let Ok(entries) = std::fs::read_dir(&site_packages) {
        for entry in entries.flatten() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if !dir_name.ends_with(".dist-info") {
                continue;
            }

            let dist_info = entry.path();

            // Install timestamp from .dist-info directory mtime
            let installed_at = std::fs::metadata(&dist_info)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);

            // Name + Version from METADATA
            let (pkg_name, pkg_version) = match std::fs::read_to_string(dist_info.join("METADATA"))
            {
                Ok(content) => parse_metadata(&content),
                Err(_) => continue,
            };
            let pkg_name = match pkg_name {
                Some(n) => n.to_lowercase(),
                None => continue,
            };

            // Installer (pip / uv)
            let installer = std::fs::read_to_string(dist_info.join("INSTALLER"))
                .ok()
                .map(|s| s.trim().to_string());

            // Source info from direct_url.json
            let (install_source, is_editable, source_url, commit_id) =
                match std::fs::read_to_string(dist_info.join("direct_url.json")) {
                    Ok(content) => parse_direct_url(&content),
                    Err(_) => (Some("pypi".to_string()), false, None, None),
                };

            // Primary import name from top_level.txt (only if it differs from pip name)
            let normalized_pip = pkg_name.replace('-', "_").to_lowercase();
            let import_name = std::fs::read_to_string(dist_info.join("top_level.txt"))
                .ok()
                .and_then(|content| {
                    let entries: Vec<String> = content
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect();
                    // If pip name is already in top_level.txt, no surprise — skip
                    let pip_present = entries.iter().any(|e| e.to_lowercase() == normalized_pip);
                    if pip_present {
                        return None;
                    }
                    // Pick first non-underscore entry as primary import
                    entries.into_iter().find(|e| !e.starts_with('_'))
                });

            result.push(crate::db::PackageMetadata {
                name: pkg_name,
                version: pkg_version,
                installer,
                install_source,
                is_editable,
                source_url,
                commit_id,
                import_name,
                installed_at,
            });
        }
    }

    // Override torch version with version.py (includes accurate +cuXXX suffix)
    if let Some(torch_pkg) = result.iter_mut().find(|p| p.name == "torch")
        && let Some((accurate_ver, _)) = read_torch_version(env_path.as_ref())
    {
        torch_pkg.version = Some(accurate_ver);
    }

    result
}

// =============================================================================
// ENVIRONMENT HELPERS
// =============================================================================

/// Read Python version from pyvenv.cfg (instant, no subprocess).
pub fn read_python_version(env_path: impl AsRef<Path>) -> Option<String> {
    let content = std::fs::read_to_string(env_path.as_ref().join("pyvenv.cfg")).ok()?;
    content
        .lines()
        .find(|line| line.trim().starts_with("version"))
        .and_then(|line| line.split_once('='))
        .map(|(_, v)| v.trim().to_string())
}

/// Returns the environment creation timestamp (epoch seconds) from pyvenv.cfg mtime.
pub fn get_env_created_at(env_path: impl AsRef<Path>) -> Option<i64> {
    let cfg = env_path.as_ref().join("pyvenv.cfg");
    std::fs::metadata(&cfg)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

/// Read torch version + CUDA info from `torch/version.py`.
/// Returns `(torch_version, cuda_version)` with accurate `+cuXXX` suffix.
pub fn read_torch_version(env_path: impl AsRef<Path>) -> Option<(String, Option<String>)> {
    let site_packages = get_site_packages_path(env_path.as_ref())?;
    let content = std::fs::read_to_string(site_packages.join("torch/version.py")).ok()?;

    let torch = content
        .lines()
        .find(|l| l.starts_with("__version__"))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim().trim_matches('\'').trim_matches('"').to_string())?;

    let cuda = content
        .lines()
        .find(|l| l.starts_with("cuda"))
        .and_then(|l| {
            let v = l.split('=').nth(1)?.trim();
            if v == "None" {
                return None;
            }
            Some(v.trim_matches('\'').trim_matches('"').to_string())
        });

    Some((torch, cuda))
}

/// Normalize a pip package name: lowercase + hyphens → underscores.
/// pip treats `tag-detector` and `tag_detector` as the same package.
pub fn normalize_package_name(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

// =============================================================================
// NATIVE DEPENDENCY CHECKER (learned from pip & uv)
// =============================================================================

/// A dependency issue found during environment health checking.
#[derive(Debug)]
pub enum DepIssue {
    /// A required package is not installed.
    Missing { package: String, requires: String },
    /// Installed version doesn't satisfy the requirement specifier.
    Incompatible {
        package: String,
        requires: String,
        installed_version: String,
    },
    /// Multiple .dist-info directories for the same normalized package name.
    Duplicate { package: String, count: usize },
}

impl crate::types::Diagnostic for DepIssue {
    fn message(&self) -> String {
        match self {
            DepIssue::Missing { package, requires } => {
                format!("{} requires {}, which is not installed", package, requires)
            }
            DepIssue::Incompatible {
                package,
                requires,
                installed_version,
            } => {
                format!(
                    "{} requires {}, but {} is installed",
                    package, requires, installed_version
                )
            }
            DepIssue::Duplicate { package, count } => {
                format!("{} has {} duplicate .dist-info entries", package, count)
            }
        }
    }

    fn level(&self) -> crate::types::HealthLevel {
        match self {
            DepIssue::Missing { .. } => crate::types::HealthLevel::Info,
            DepIssue::Incompatible { .. } | DepIssue::Duplicate { .. } => {
                crate::types::HealthLevel::Warn
            }
        }
    }
}

/// Check all dependency constraints in an environment. Pure filesystem, no subprocess.
///
/// Algorithm (from pip): build {name → version} index, then for each package
/// check that all its Requires-Dist entries are satisfied.
/// Typical speed: ~5ms for 200 packages.
pub fn check_dependencies(env_path: impl AsRef<Path>) -> Vec<DepIssue> {
    let mut issues = Vec::new();
    let site_packages = match get_site_packages_path(env_path.as_ref()) {
        Some(p) => p,
        None => return issues,
    };

    // Detect Python version from site-packages path (e.g., ".../python3.12/site-packages")
    let env_python_version = site_packages
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("python"))
        .unwrap_or("3.12")
        .to_string();

    // Phase 1: Build package index {normalized_name → (version, dist_info_path)}
    let mut index: std::collections::HashMap<String, (String, PathBuf)> =
        std::collections::HashMap::new();
    let mut duplicates: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    let entries: Vec<_> = match std::fs::read_dir(&site_packages) {
        Ok(rd) => rd.flatten().collect(),
        Err(_) => return issues,
    };

    for entry in &entries {
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if !dir_name.ends_with(".dist-info") {
            continue;
        }
        let meta_path = entry.path().join("METADATA");
        let content = match std::fs::read_to_string(&meta_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (name, version) = parse_metadata(&content);
        let (Some(name), Some(version)) = (name, version) else {
            continue;
        };
        let norm = normalize_package_name(&name);

        // Track duplicates
        let count = duplicates.entry(norm.clone()).or_insert(0);
        *count += 1;

        index.insert(norm, (version, entry.path()));
    }

    // Report duplicates
    for (name, count) in &duplicates {
        if *count > 1 {
            issues.push(DepIssue::Duplicate {
                package: name.clone(),
                count: *count,
            });
        }
    }

    // Phase 2: Check each package's Requires-Dist against the index
    for entry in &entries {
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if !dir_name.ends_with(".dist-info") {
            continue;
        }
        let meta_path = entry.path().join("METADATA");
        let content = match std::fs::read_to_string(&meta_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (pkg_name, _) = parse_metadata(&content);
        let Some(pkg_name) = pkg_name else { continue };
        let pkg_display = pkg_name.clone();

        // Parse all Requires-Dist lines (skip extras and inapplicable markers)
        for line in content.lines() {
            let Some(req_str) = line.strip_prefix("Requires-Dist: ") else {
                continue;
            };

            // Skip extra-only dependencies ("; extra == ...")
            if req_str.contains("extra ==") || req_str.contains("extra==\"") {
                continue;
            }

            // Evaluate markers if present
            if let Some(marker_part) = req_str.split(';').nth(1) {
                let m = marker_part.trim();

                // Skip OS/platform-specific deps
                if m.contains("sys_platform")
                    || m.contains("platform_system")
                    || m.contains("os_name")
                    || m.contains("implementation_name")
                    || m.contains("platform_machine")
                {
                    continue;
                }

                // Evaluate python_version markers — this is the big noise reducer
                if marker_excludes_python(m, &env_python_version) {
                    continue;
                }
            }

            // Parse name and specifier from requirement string
            // Format: "name (>=1.0,<2.0)" or "name>=1.0,<2.0" or "name"
            let req_no_marker = req_str.split(';').next().unwrap_or(req_str).trim();

            // Skip URL/VCS requirements (e.g., "diffusers @ git+https://...")
            // We can't verify these — the package is installed but from a URL, not PyPI
            if req_no_marker.contains(" @ ") {
                continue;
            }

            let (dep_name, specifier) = parse_requirement_name_and_spec(req_no_marker);
            let dep_norm = normalize_package_name(&dep_name);

            match index.get(&dep_norm) {
                None => {
                    issues.push(DepIssue::Missing {
                        package: pkg_display.clone(),
                        requires: dep_name.to_string(),
                    });
                }
                Some((installed_ver, _)) => {
                    if !specifier.is_empty()
                        && !version_satisfies_specifier(installed_ver, &specifier)
                    {
                        issues.push(DepIssue::Incompatible {
                            package: pkg_display.clone(),
                            requires: format!("{}{}", dep_name, specifier),
                            installed_version: installed_ver.clone(),
                        });
                    }
                }
            }
        }
    }

    issues
}

/// Evaluate whether a marker expression excludes the given Python version.
///
/// Handles common patterns like:
///   - `python_version < "3.11"`
///   - `python_version >= "3.8" and python_version < "3.11"`
///   - `python_full_version < "3.11.0"`
///
/// Returns true if the dep should be SKIPPED (marker evaluates to false).
fn marker_excludes_python(marker: &str, env_py_version: &str) -> bool {
    // Handle compound "and" expressions — ALL clauses must be true
    if marker.contains(" and ") {
        let clauses: Vec<&str> = marker.split(" and ").collect();
        // If any python_version clause evaluates to false, skip the dep
        let mut has_py_clause = false;
        for clause in &clauses {
            let clause = clause.trim();
            if clause.contains("python_version") || clause.contains("python_full_version") {
                has_py_clause = true;
                if !eval_single_python_marker(clause, env_py_version) {
                    return true; // One clause is false → whole AND is false → skip
                }
            }
        }
        // If we had python clauses and they all passed, don't exclude
        if has_py_clause {
            return false;
        }
    }

    // Handle compound "or" expressions — ANY clause true means include
    if marker.contains(" or ") {
        let clauses: Vec<&str> = marker.split(" or ").collect();
        let mut has_py_clause = false;
        let mut any_true = false;
        for clause in &clauses {
            let clause = clause.trim();
            if clause.contains("python_version") || clause.contains("python_full_version") {
                has_py_clause = true;
                if eval_single_python_marker(clause, env_py_version) {
                    any_true = true;
                }
            }
        }
        if has_py_clause && !any_true {
            return true; // All python clauses are false → skip
        }
        if has_py_clause {
            return false;
        }
    }

    // Single clause
    if marker.contains("python_version") || marker.contains("python_full_version") {
        return !eval_single_python_marker(marker, env_py_version);
    }

    false // Don't exclude if we can't evaluate
}

/// Evaluate a single python_version comparison clause.
/// e.g., `python_version < "3.11"` or `python_full_version >= "3.9.0"`
fn eval_single_python_marker(clause: &str, env_py_version: &str) -> bool {
    // Extract operator and version value
    let clause = clause.trim().trim_start_matches('(').trim_end_matches(')');

    // Find the operator
    let (op, ver_str) = if let Some(pos) = clause.find(">=") {
        (">=", &clause[pos + 2..])
    } else if let Some(pos) = clause.find("<=") {
        ("<=", &clause[pos + 2..])
    } else if let Some(pos) = clause.find("!=") {
        ("!=", &clause[pos + 2..])
    } else if let Some(pos) = clause.find("==") {
        ("==", &clause[pos + 2..])
    } else if let Some(pos) = clause.find('>') {
        (">", &clause[pos + 1..])
    } else if let Some(pos) = clause.find('<') {
        ("<", &clause[pos + 1..])
    } else {
        return true; // Can't parse → assume true (include dep)
    };

    // Extract version string from quotes
    let ver_str = ver_str.trim();
    let ver_str = ver_str.trim_matches('"').trim_matches('\'').trim();
    if ver_str.is_empty() {
        return true;
    }

    let cmp = compare_versions(env_py_version, ver_str);

    match op {
        ">=" => cmp >= 0,
        "<=" => cmp <= 0,
        ">" => cmp > 0,
        "<" => cmp < 0,
        "==" => cmp == 0,
        "!=" => cmp != 0,
        _ => true,
    }
}
/// Handles formats: "name (>=1.0,<2.0)", "name>=1.0", "name[extra]>=1.0", "name"
fn parse_requirement_name_and_spec(req: &str) -> (String, String) {
    let req = req.trim();

    // Handle parenthesized specifiers: "name (>=1.0,<2.0)"
    if let Some(paren_start) = req.find('(') {
        let name = req[..paren_start].trim();
        // Strip extras from name: "name[extra]" → "name"
        let name = name.split('[').next().unwrap_or(name).trim();
        let spec = req[paren_start..]
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim();
        return (name.to_string(), spec.to_string());
    }

    // Handle inline specifiers: "name>=1.0,<2.0"
    let spec_start = req.find(['>', '<', '=', '!', '~']);
    match spec_start {
        Some(pos) => {
            let name = req[..pos].trim();
            let name = name.split('[').next().unwrap_or(name).trim();
            let spec = req[pos..].trim();
            (name.to_string(), spec.to_string())
        }
        None => {
            let name = req.split('[').next().unwrap_or(req).trim();
            (name.to_string(), String::new())
        }
    }
}

/// Strip PEP 440 local version suffix (+cuXXX, +cpu, etc.) for comparison.
fn strip_local_version(version: &str) -> &str {
    version.split('+').next().unwrap_or(version)
}

/// Check if an installed version satisfies a specifier string like ">=1.0,<2.0,!=1.5".
///
/// Supports: >=, <=, >, <, ==, !=, ~=
/// Strips local version suffixes (+cuXXX) before comparison.
fn version_satisfies_specifier(installed: &str, specifier: &str) -> bool {
    let installed_clean = strip_local_version(installed);

    for constraint in specifier.split(',') {
        let constraint = constraint.trim();
        if constraint.is_empty() {
            continue;
        }

        let (op, ver_str) = if let Some(rest) = constraint.strip_prefix("~=") {
            ("~=", rest.trim())
        } else if let Some(rest) = constraint.strip_prefix(">=") {
            (">=", rest.trim())
        } else if let Some(rest) = constraint.strip_prefix("<=") {
            ("<=", rest.trim())
        } else if let Some(rest) = constraint.strip_prefix("!=") {
            ("!=", rest.trim())
        } else if let Some(rest) = constraint.strip_prefix("==") {
            ("==", rest.trim())
        } else if let Some(rest) = constraint.strip_prefix('>') {
            (">", rest.trim())
        } else if let Some(rest) = constraint.strip_prefix('<') {
            ("<", rest.trim())
        } else {
            continue;
        };

        let req_clean = strip_local_version(ver_str);
        // Handle wildcard == (e.g., "==1.*")
        if op == "==" && req_clean.ends_with(".*") {
            let prefix = &req_clean[..req_clean.len() - 2];
            if !installed_clean.starts_with(prefix)
                && !installed_clean.starts_with(&format!("{}.", prefix))
                && installed_clean != prefix
            {
                return false;
            }
            continue;
        }

        let cmp = compare_versions(installed_clean, req_clean);

        let satisfied = match op {
            ">=" => cmp >= 0,
            "<=" => cmp <= 0,
            ">" => cmp > 0,
            "<" => cmp < 0,
            "==" => cmp == 0,
            "!=" => cmp != 0,
            "~=" => {
                // Compatible release: ~=X.Y means >=X.Y, <(X+1).0
                cmp >= 0 && {
                    let parts: Vec<&str> = req_clean.split('.').collect();
                    if parts.len() >= 2 {
                        let major = parts[..parts.len() - 1].join(".");
                        installed_clean.starts_with(&major)
                            || installed_clean.starts_with(&format!("{}.", major))
                    } else {
                        true
                    }
                }
            }
            _ => true,
        };

        if !satisfied {
            return false;
        }
    }

    true
}

/// Compare two version strings numerically segment by segment.
/// Returns -1, 0, or 1 like strcmp.
fn compare_versions(a: &str, b: &str) -> i32 {
    let parse_num = |s: &str| -> u64 {
        s.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u64>()
            .unwrap_or(0)
    };

    let parts_a: Vec<&str> = a.split('.').collect();
    let parts_b: Vec<&str> = b.split('.').collect();
    let max_len = parts_a.len().max(parts_b.len());

    for i in 0..max_len {
        let va = parts_a.get(i).map(|s| parse_num(s)).unwrap_or(0);
        let vb = parts_b.get(i).map(|s| parse_num(s)).unwrap_or(0);

        if va < vb {
            return -1;
        }
        if va > vb {
            return 1;
        }
    }

    0
}

/// Locate site-packages for an environment.
pub fn get_site_packages_path(env_path: &Path) -> Option<PathBuf> {
    let lib_path = env_path.join("lib");
    let python_dir = std::fs::read_dir(&lib_path)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("python"))?
        .path();

    let site_packages = python_dir.join("site-packages");
    site_packages.exists().then_some(site_packages)
}

/// Parse Name and Version from METADATA file content.
/// Scans through the header section (until first blank line) to find Name: and Version:.
/// Some packages (e.g., protobuf) have many Classifier lines pushing Version: past line 10.
fn parse_metadata(content: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut version = None;

    for line in content.lines() {
        // Empty line = end of headers, start of description body
        if line.trim().is_empty() && name.is_some() {
            break;
        }
        if let Some(val) = line.strip_prefix("Name: ") {
            name = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("Version: ") {
            version = Some(val.to_string());
        }
        if name.is_some() && version.is_some() {
            break;
        }
    }

    (name, version)
}

/// Parse direct_url.json for install source information.
fn parse_direct_url(content: &str) -> (Option<String>, bool, Option<String>, Option<String>) {
    let is_editable =
        content.contains("\"editable\": true") || content.contains("\"editable\":true");
    let is_git = content.contains("\"vcs\": \"git\"") || content.contains("\"vcs\":\"git\"");

    let source_url = extract_json_string(content, "url");
    let commit_id = extract_json_string(content, "commit_id");

    let install_source = if is_git {
        Some("git".to_string())
    } else if source_url
        .as_ref()
        .is_some_and(|u| u.starts_with("file://"))
    {
        Some("local".to_string())
    } else {
        Some("pypi".to_string())
    };

    (install_source, is_editable, source_url, commit_id)
}

/// Extract a string value from JSON by key (simple, regex-free).
fn extract_json_string(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let start = content.find(&pattern)?;
    let rest = &content[start + pattern.len()..];
    let quote_start = rest.find('"')?;
    let rest = &rest[quote_start + 1..];
    let quote_end = rest.find('"')?;
    Some(rest[..quote_end].to_string())
}

// =============================================================================
// SHELL INTERACTION
// =============================================================================

/// Executes a command within a Zen environment context.
///
/// Sets the environment's `bin` directory at the front of PATH
/// and sets `VIRTUAL_ENV` for standard tool compatibility.
pub fn run_in_env(env_path: impl AsRef<Path>, cmd: &str, args: &[&str]) -> bool {
    let env_path = env_path.as_ref();
    let bin_path = env_path.join("bin");
    let exe_path = bin_path.join(cmd);

    let mut command = Command::new(if exe_path.exists() {
        exe_path.to_str().unwrap()
    } else {
        cmd
    });

    command.args(args);
    let path = std::env::var("PATH").unwrap_or_default();
    command.env("PATH", format!("{}:{}", bin_path.display(), path));
    command.env("VIRTUAL_ENV", env_path);

    command.status().map(|s| s.success()).unwrap_or(false)
}

/// Like `run_in_env`, but captures stdout/stderr to suppress output.
pub fn run_in_env_silent(env_path: impl AsRef<Path>, cmd: &str, args: &[&str]) -> bool {
    let env_path = env_path.as_ref();
    let bin_path = env_path.join("bin");
    let exe_path = bin_path.join(cmd);

    let mut command = Command::new(if exe_path.exists() {
        exe_path.to_str().unwrap()
    } else {
        cmd
    });

    command.args(args);
    let path = std::env::var("PATH").unwrap_or_default();
    command.env("PATH", format!("{}:{}", bin_path.display(), path));
    command.env("VIRTUAL_ENV", env_path);

    command
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Like `run_in_env_silent`, but returns captured (success, stdout, stderr).
#[allow(dead_code)]
pub fn run_in_env_capture(
    env_path: impl AsRef<Path>,
    cmd: &str,
    args: &[&str],
) -> (bool, String, String) {
    let env_path = env_path.as_ref();
    let bin_path = env_path.join("bin");
    let exe_path = bin_path.join(cmd);

    let mut command = Command::new(if exe_path.exists() {
        exe_path.to_str().unwrap()
    } else {
        cmd
    });

    command.args(args);
    let path = std::env::var("PATH").unwrap_or_default();
    command.env("PATH", format!("{}:{}", bin_path.display(), path));
    command.env("VIRTUAL_ENV", env_path);

    match command.output() {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ),
        Err(e) => (false, String::new(), e.to_string()),
    }
}

// =============================================================================
// MISCELLANEOUS
// =============================================================================

pub struct TemplatePart {
    pub name: String,
    pub version: String,
}

/// Parses a template string (`name:version|name:version`).
pub fn parse_template_string(template_str: &str) -> Vec<TemplatePart> {
    template_str
        .split('|')
        .map(|part| {
            let mut subparts = part.splitn(2, ':');
            let name = subparts.next().unwrap_or_default().to_string();
            let version = subparts.next().unwrap_or("latest").to_string();
            TemplatePart { name, version }
        })
        .collect()
}

/// Returns the PyTorch wheel index URL for a given CUDA version.
pub fn get_torch_index_url(cuda_version: &str) -> Option<&'static str> {
    match cuda_version {
        "11.8" => Some("https://download.pytorch.org/whl/cu118"),
        "12.1" => Some("https://download.pytorch.org/whl/cu121"),
        "12.4" => Some("https://download.pytorch.org/whl/cu124"),
        "12.8" => Some("https://download.pytorch.org/whl/cu128"),
        "13.0" => Some("https://download.pytorch.org/whl/cu130"),
        _ => None,
    }
}

/// Attempts to identify the currently active virtual environment path.
///
/// Checks `VIRTUAL_ENV` first, then falls back to runtime prefix introspection.
pub fn get_current_venv_path() -> Option<String> {
    if let Ok(venv) = std::env::var("VIRTUAL_ENV")
        && !venv.is_empty()
    {
        return Some(venv);
    }

    let output = Command::new("python3")
        .arg("-c")
        .arg("import sys; print(sys.prefix)")
        .output()
        .ok()?;
    if output.status.success() {
        let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if prefix != "/usr" && prefix != "/usr/local" && !prefix.starts_with("/System") {
            return Some(prefix);
        }
    }

    None
}

/// Discovers all virtual environments within a directory tree (max depth 3).
pub fn discover_venvs(base_path: &Path) -> Vec<PathBuf> {
    let mut venvs = Vec::new();
    if !base_path.is_dir() {
        return venvs;
    }

    fn scan_recursive(path: &Path, depth: usize, results: &mut Vec<PathBuf>) {
        if depth > 3 {
            return;
        }
        if path.join("bin/python").exists() {
            results.push(path.to_path_buf());
            return;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "node_modules" && name != "target" {
                        scan_recursive(&p, depth + 1, results);
                    }
                }
            }
        }
    }

    scan_recursive(base_path, 0, &mut venvs);
    venvs
}
