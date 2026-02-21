// SPDX-License-Identifier: Apache-2.0
//! Interactive REPL for template create/edit.
//!
//! Architecture: the REPL loop in `template_repl` dispatches to pure-data
//! parsing (`parse_repl_line`) and side-effecting handlers (`handle_*`).
//! Parsing returns `Result<ReplCmd, String>` — errors are just strings that
//! the loop prints.  No nested loops exist in the dispatch path, eliminating
//! the class of infinite-print bugs caused by `continue` targeting the wrong
//! loop.

use colored::Colorize;

use crate::db::Database;
use crate::utils;

// ────────────────────────────────────────────────────────────────────
// Types
// ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Pkg {
    pub name: String,
    pub version: String,
    pub install_type: String, // "pypi" or "wheel"
}

#[derive(Clone)]
pub struct Step {
    pub install_args: Option<String>, // e.g. "--index-url https://..."
    pub packages: Vec<Pkg>,
}

#[derive(Debug)]
pub enum StepMode {
    At(usize),
    After(usize),
    Before(usize),
    New,
}

#[derive(Debug)]
pub struct AddArgs {
    pub packages: Vec<String>,
    pub wheel_path: Option<String>,
    pub index_url: Option<String>,
    pub step_mode: StepMode,
}

#[derive(Debug)]
pub enum ReplCmd {
    Help(Option<String>),
    List,
    Add(AddArgs),
    Drop(String),
    Save,
    Quit,
}

// ────────────────────────────────────────────────────────────────────
// Parsing  (pure — no side effects, no loops that can spin)
// ────────────────────────────────────────────────────────────────────

/// Strip tool prefixes so e.g. "zen install X" → "install X".
pub fn strip_tool_prefix(line: &str) -> &str {
    line.strip_prefix("zen ")
        .or_else(|| line.strip_prefix("pip "))
        .or_else(|| line.strip_prefix("uv pip "))
        .or_else(|| line.strip_prefix("uv "))
        .or_else(|| line.strip_prefix("conda "))
        .unwrap_or(line)
}

/// Resolve a signed index relative to `num_steps`.
///
/// Negative: -1 = last, -2 = second-to-last, etc.
/// Returns `Err` with a user-facing message on out-of-range.
fn resolve_index(n: isize, num_steps: usize, _flag: &str) -> Result<usize, String> {
    if n < 0 {
        let pos = num_steps as isize + n;
        if pos < 0 {
            Err(format!(
                "Index {} is out of range (only {} steps).",
                n, num_steps
            ))
        } else {
            Ok(pos as usize)
        }
    } else {
        Ok(n as usize)
    }
}

/// Parse a step-positioning flag value (--at, --after, --before).
///
/// Handles quote stripping, isize parsing, and negative-index resolution.
fn parse_step_flag(val: &str, num_steps: usize, flag: &str) -> Result<usize, String> {
    let val = val.trim_matches('"').trim_matches('\'');
    let n: isize = val
        .parse()
        .map_err(|_| format!("{} requires a numeric step index.", flag))?;
    resolve_index(n, num_steps, flag)
}

/// Parse a REPL input line into a `ReplCmd`.
///
/// `parts` is the whitespace-split input (first element is the command).
/// `num_steps` is the current number of in-memory steps (for index resolution).
///
/// Returns `Err(message)` on any parse failure — the caller just prints it.
pub fn parse_repl_line(parts: &[&str], num_steps: usize) -> Result<ReplCmd, String> {
    if parts.is_empty() {
        return Err(String::new()); // caller silently continues
    }

    let cmd = parts[0].to_lowercase();
    match cmd.as_str() {
        "help" | "h" | "?" => {
            let topic = parts.get(1).map(|s| s.to_lowercase());
            Ok(ReplCmd::Help(topic))
        }
        "list" | "ls" => Ok(ReplCmd::List),
        "save" => Ok(ReplCmd::Save),
        "quit" | "exit" | "q" => Ok(ReplCmd::Quit),
        "drop" | "remove" | "rm" => {
            if parts.len() < 2 {
                return Err("Usage: drop <package_name | step_number>".into());
            }
            Ok(ReplCmd::Drop(parts[1].to_string()))
        }
        "add" | "install" => parse_add_args(parts, num_steps),
        other => Err(format!(
            "Unknown command '{}'. Type {} for help.",
            other, "help"
        )),
    }
}

/// Parse the arguments for the `add` / `install` command.
fn parse_add_args(parts: &[&str], num_steps: usize) -> Result<ReplCmd, String> {
    if parts.len() < 2 {
        return Err("Usage: add <package> [package...] [--index-url URL] [--at N]".into());
    }

    let mut packages: Vec<String> = Vec::new();
    let mut index_url: Option<String> = None;
    let mut wheel_path: Option<String> = None;
    let mut step_mode = StepMode::New;

    let mut i = 1;
    while i < parts.len() {
        match parts[i] {
            "--index-url" | "--index" | "-i" => {
                if i + 1 >= parts.len() {
                    return Err("--index-url requires a URL argument.".into());
                }
                index_url = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--wheel" | "-w" => {
                if i + 1 >= parts.len() {
                    return Err("--wheel requires a path argument.".into());
                }
                wheel_path = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--at" | "--step" | "-s" => {
                if i + 1 >= parts.len() {
                    return Err("--at requires a step index.".into());
                }
                let resolved = parse_step_flag(parts[i + 1], num_steps, "--at")?;
                step_mode = StepMode::At(resolved);
                i += 2;
            }
            "--after" => {
                if i + 1 >= parts.len() {
                    return Err("--after requires a step index.".into());
                }
                let resolved = parse_step_flag(parts[i + 1], num_steps, "--after")?;
                step_mode = StepMode::After(resolved);
                i += 2;
            }
            "--before" => {
                if i + 1 >= parts.len() {
                    return Err("--before requires a step index.".into());
                }
                let resolved = parse_step_flag(parts[i + 1], num_steps, "--before")?;
                step_mode = StepMode::Before(resolved);
                i += 2;
            }
            flag if flag.starts_with("--") => {
                return Err(format!(
                    "Unknown flag '{}'. See {} for usage.",
                    flag, "help add"
                ));
            }
            pkg => {
                packages.push(pkg.to_string());
                i += 1;
            }
        }
    }

    if packages.is_empty() && wheel_path.is_none() {
        return Err("No packages specified. Usage: add <pkg...>".into());
    }

    Ok(ReplCmd::Add(AddArgs {
        packages,
        wheel_path,
        index_url,
        step_mode,
    }))
}

// ────────────────────────────────────────────────────────────────────
// Command handlers
// ────────────────────────────────────────────────────────────────────

/// Print help text.
pub fn handle_help(topic: Option<&str>) {
    match topic {
        Some("add") | Some("install") => {
            println!(
                "  Usage: {} [--index-url URL] [--wheel /path] [--at/--after/--before N]",
                "add <pkg...>".cyan()
            );
            println!();
            println!("  Installs package(s) into the template environment.");
            println!("  Each add creates a new step unless a positioning flag is used:");
            println!();
            println!(
                "    {}   Append to existing step N (inherits its --index-url)",
                "--at N / --step N".bold()
            );
            println!(
                "    {}      Insert as new step after N (shifts later steps)",
                "--after N".bold()
            );
            println!(
                "    {}     Insert as new step before N (shifts later steps)",
                "--before N".bold()
            );
            println!();
            println!("  Examples:");
            println!("    add numpy pandas");
            println!(
                "    add torch torchvision --index-url https://download.pytorch.org/whl/cu130"
            );
            println!("    add torchaudio --at 0         (appends to step 0, inherits its index)");
            println!("    add my-pkg --wheel /path/to/my_pkg.whl");
        }
        Some("drop") | Some("remove") | Some("rm") => {
            println!(
                "  Usage: {} or {}",
                "drop <package>".cyan(),
                "drop <step_number>".cyan()
            );
            println!("  Removes a package by name, or an entire step by number.");
        }
        _ => {
            println!("  {}       Show current packages", "list / ls".bold());
            println!(
                "  {}    Install package(s) (--index-url, --wheel supported)",
                "add <pkg...>".bold()
            );
            println!(
                "  {}  Remove package or entire step",
                "drop <pkg|step>".bold()
            );
            println!("  {}           Save template and exit", "save".bold());
            println!("  {}     Discard changes and exit", "quit / exit".bold());
            println!();
            println!("  Type {} for detailed usage.", "help <command>".bold());
        }
    }
}

/// Print current step/package status.
pub fn print_status(steps: &[Step], template_name: &str, template_version: &str) {
    let total_pkgs: usize = steps.iter().map(|s| s.packages.len()).sum();
    println!(
        "\n  {}  {}:{}  —  {} step(s), {} package(s)\n",
        "●".bold(),
        template_name.bold(),
        template_version,
        steps.len(),
        total_pkgs
    );
    for (i, step) in steps.iter().enumerate() {
        if let Some(args) = &step.install_args {
            println!("  Step {}  ─ {}", i, args.dimmed());
        } else {
            println!("  Step {}", i);
        }
        for pkg in &step.packages {
            println!(
                "    {:<24}{:<20}{}",
                pkg.name,
                pkg.version,
                pkg.install_type.dimmed()
            );
        }
    }
    if total_pkgs > 0 {
        println!();
    }
}

/// Execute the `add` command: install packages, update in-memory steps.
pub fn handle_add(
    steps: &mut Vec<Step>,
    args: AddArgs,
    env_path: &str,
    use_uv: bool,
) -> Result<(), String> {
    let AddArgs {
        packages: pkgs_to_install,
        wheel_path,
        mut index_url,
        step_mode,
    } = args;

    // Determine target step index and inherit index_url for --at
    let target_idx = match &step_mode {
        StepMode::At(n) => {
            if *n >= steps.len() {
                return Err(format!(
                    "Step {} not found (max: {}).",
                    n,
                    steps.len().saturating_sub(1)
                ));
            }
            // Inherit index_url from existing step if not explicitly set
            if index_url.is_none()
                && let Some(existing) = steps.get(*n)
            {
                index_url = existing.install_args.as_ref().and_then(|args| {
                    let a: Vec<&str> = args.split_whitespace().collect();
                    for (j, part) in a.iter().enumerate() {
                        if (*part == "--index-url" || *part == "--index") && j + 1 < a.len() {
                            return Some(a[j + 1].to_string());
                        }
                    }
                    None
                });
            }
            *n
        }
        StepMode::After(n) => {
            if *n >= steps.len() {
                return Err(format!(
                    "Step {} not found (max: {}).",
                    n,
                    steps.len().saturating_sub(1)
                ));
            }
            n + 1
        }
        StepMode::Before(n) => {
            if *n > steps.len() {
                return Err(format!("Step {} not found.", n));
            }
            *n
        }
        StepMode::New => steps.len(),
    };

    // Build install_args string for metadata
    let install_args_str = index_url.as_ref().map(|url| format!("--index-url {}", url));

    // Perform the install in the temp env
    let mut cmd_args: Vec<String> = if use_uv {
        vec!["pip".to_string(), "install".to_string()]
    } else {
        vec!["install".to_string()]
    };
    if let Some(ref url) = index_url {
        cmd_args.push("--index-url".to_string());
        cmd_args.push(url.clone());
    }

    if let Some(ref whl) = wheel_path {
        cmd_args.push(whl.clone());
    } else {
        for p in &pkgs_to_install {
            cmd_args.push(p.clone());
        }
    }

    let cmd_str_args: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
    let bin = if use_uv { "uv" } else { "pip" };
    let ok = utils::run_in_env(env_path, bin, &cmd_str_args);

    if !ok {
        return Err("Install failed.".into());
    }

    // Resolve installed versions via `pip show`
    let mut new_pkgs: Vec<Pkg> = Vec::new();

    if let Some(ref whl) = wheel_path {
        let pkg_name = std::path::Path::new(whl)
            .file_stem()
            .map(|n| {
                n.to_string_lossy()
                    .split('-')
                    .next()
                    .unwrap_or("unknown")
                    .to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());
        let show_args: Vec<&str> = if use_uv {
            vec!["pip", "show", &pkg_name]
        } else {
            vec!["show", &pkg_name]
        };
        let (_ok, stdout, _stderr) =
            utils::run_in_env_capture(env_path, if use_uv { "uv" } else { "pip" }, &show_args);
        let version = stdout
            .lines()
            .find(|l: &&str| l.starts_with("Version:"))
            .map(|l: &str| l.trim_start_matches("Version:").trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {} {} {}", "✓".green(), pkg_name, version.dimmed());
        new_pkgs.push(Pkg {
            name: pkg_name,
            version,
            install_type: "wheel".to_string(),
        });
    } else {
        for pkg in &pkgs_to_install {
            let base_name = pkg
                .split(&['>', '<', '=', '!', '~'][..])
                .next()
                .unwrap_or(pkg);
            let show_args: Vec<&str> = if use_uv {
                vec!["pip", "show", base_name]
            } else {
                vec!["show", base_name]
            };
            let (_ok, stdout, _stderr) =
                utils::run_in_env_capture(env_path, if use_uv { "uv" } else { "pip" }, &show_args);
            let version = stdout
                .lines()
                .find(|l: &&str| l.starts_with("Version:"))
                .map(|l: &str| l.trim_start_matches("Version:").trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            println!("  {} {} {}", "✓".green(), base_name, version.dimmed());
            new_pkgs.push(Pkg {
                name: base_name.to_string(),
                version,
                install_type: "pypi".to_string(),
            });
        }
    }

    // Deduplicate: remove these packages from other steps
    let new_names: Vec<String> = new_pkgs.iter().map(|p| p.name.clone()).collect();
    for (si, step) in steps.iter_mut().enumerate() {
        // For --at mode, skip the target step itself
        if matches!(step_mode, StepMode::At(_)) && si == target_idx {
            continue;
        }
        step.packages.retain(|p| !new_names.contains(&p.name));
    }
    steps.retain(|s| !s.packages.is_empty());

    // Recalculate target_idx after cleanup (steps may have shifted)
    let target_idx = match &step_mode {
        StepMode::At(n) => (*n).min(steps.len().saturating_sub(1)),
        StepMode::After(n) => (n + 1).min(steps.len()),
        StepMode::Before(n) => (*n).min(steps.len()),
        StepMode::New => steps.len(),
    };

    // Insert into in-memory steps
    match step_mode {
        StepMode::At(_) => {
            if let Some(step) = steps.get_mut(target_idx) {
                // Remove existing entries with the same name before appending
                step.packages.retain(|p| !new_names.contains(&p.name));
                step.packages.extend(new_pkgs);
            }
        }
        _ => {
            steps.insert(
                target_idx,
                Step {
                    install_args: install_args_str,
                    packages: new_pkgs,
                },
            );
        }
    }

    Ok(())
}

/// Execute the `drop` command: remove a package by name or an entire step.
pub fn handle_drop(steps: &mut Vec<Step>, target: &str) -> Result<(), String> {
    if let Ok(step_idx) = target.parse::<usize>() {
        if step_idx < steps.len() {
            let removed = steps.remove(step_idx);
            println!(
                "  {} Dropped step {} ({} package(s)).",
                "✓".green(),
                step_idx,
                removed.packages.len()
            );
        } else {
            return Err(format!("Step {} not found.", step_idx));
        }
    } else {
        let mut found = false;
        for step in steps.iter_mut() {
            if let Some(pos) = step.packages.iter().position(|p| p.name == target) {
                step.packages.remove(pos);
                found = true;
                break;
            }
        }
        steps.retain(|s| !s.packages.is_empty());
        if found {
            println!("  {} Dropped '{}'.", "✓".green(), target);
        } else {
            return Err(format!("'{}' not found.", target));
        }
    }
    Ok(())
}

/// Flush in-memory steps to the database.
pub fn handle_save(
    db: &Database,
    template_id: i64,
    steps: &[Step],
    template_name: &str,
    template_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Clear existing, then write fresh
    if let Ok(existing) = db.get_template_packages(template_id) {
        for (pname, ..) in &existing {
            let _ = db.remove_template_package(template_id, pname);
        }
    }
    for (step_idx, step) in steps.iter().enumerate() {
        for pkg in &step.packages {
            db.add_template_package(
                template_id,
                &pkg.name,
                &pkg.version,
                true,
                &pkg.install_type,
                step.install_args.as_deref(),
                step_idx as i64,
            )?;
        }
    }
    db.clear_sessions()?;
    let total: usize = steps.iter().map(|s| s.packages.len()).sum();
    println!(
        "\n  {} Template '{}:{}' saved ({} package(s)).\n",
        "✓".green(),
        template_name,
        template_version,
        total
    );
    Ok(())
}

/// Load existing packages from DB into in-memory steps.
pub fn load_steps_from_db(db: &Database, template_id: i64) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    if let Ok(packages) = db.get_template_packages(template_id) {
        let mut current_step: i64 = -1;
        for (p_name, p_ver, _pinned, itype, iargs, step_num) in &packages {
            if *step_num != current_step {
                current_step = *step_num;
                steps.push(Step {
                    install_args: iargs.clone(),
                    packages: Vec::new(),
                });
            }
            if let Some(s) = steps.last_mut() {
                s.packages.push(Pkg {
                    name: p_name.clone(),
                    version: p_ver.clone(),
                    install_type: itype.clone(),
                });
            }
        }
    }
    steps
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_add_basic() {
        let parts = vec!["add", "numpy", "pandas"];
        let cmd = parse_repl_line(&parts, 0).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert_eq!(args.packages, vec!["numpy", "pandas"]);
                assert!(args.index_url.is_none());
                assert!(args.wheel_path.is_none());
                assert!(matches!(args.step_mode, StepMode::New));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_add_with_index_url() {
        let parts = vec![
            "add",
            "torch",
            "--index-url",
            "https://download.pytorch.org/whl/cu130",
        ];
        let cmd = parse_repl_line(&parts, 0).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert_eq!(args.packages, vec!["torch"]);
                assert_eq!(
                    args.index_url.unwrap(),
                    "https://download.pytorch.org/whl/cu130"
                );
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_add_at_positive() {
        let parts = vec!["add", "scipy", "--at", "1"];
        let cmd = parse_repl_line(&parts, 3).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert!(matches!(args.step_mode, StepMode::At(1)));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_add_at_negative() {
        // -1 with 3 steps → step 2
        let parts = vec!["add", "scipy", "--at", "-1"];
        let cmd = parse_repl_line(&parts, 3).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert!(matches!(args.step_mode, StepMode::At(2)));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_add_at_negative_out_of_range() {
        // -1 with 0 steps → error
        let parts = vec!["add", "scipy", "--at", "-1"];
        let result = parse_repl_line(&parts, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));
    }

    #[test]
    fn test_parse_add_at_missing_value() {
        let parts = vec!["add", "scipy", "--at"];
        let result = parse_repl_line(&parts, 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a step index"));
    }

    #[test]
    fn test_parse_add_no_packages() {
        let parts = vec!["add"];
        let result = parse_repl_line(&parts, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_add_after_negative() {
        // --after -2 with 3 steps → after step 1
        let parts = vec!["add", "scipy", "--after", "-2"];
        let cmd = parse_repl_line(&parts, 3).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert!(matches!(args.step_mode, StepMode::After(1)));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_add_before_negative() {
        // --before -1 with 3 steps → before step 2
        let parts = vec!["add", "scipy", "--before", "-1"];
        let cmd = parse_repl_line(&parts, 3).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert!(matches!(args.step_mode, StepMode::Before(2)));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_add_quoted_index() {
        let parts = vec!["add", "scipy", "--at", "'2'"];
        let cmd = parse_repl_line(&parts, 5).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert!(matches!(args.step_mode, StepMode::At(2)));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_add_unknown_flag() {
        let parts = vec!["add", "numpy", "--foobar"];
        let result = parse_repl_line(&parts, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown flag"));
    }

    #[test]
    fn test_parse_drop() {
        let parts = vec!["drop", "numpy"];
        let cmd = parse_repl_line(&parts, 0).unwrap();
        match cmd {
            ReplCmd::Drop(target) => assert_eq!(target, "numpy"),
            _ => panic!("Expected Drop"),
        }
    }

    #[test]
    fn test_parse_drop_no_target() {
        let parts = vec!["drop"];
        let result = parse_repl_line(&parts, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_help() {
        let parts = vec!["help"];
        let cmd = parse_repl_line(&parts, 0).unwrap();
        assert!(matches!(cmd, ReplCmd::Help(None)));
    }

    #[test]
    fn test_parse_help_add() {
        let parts = vec!["help", "add"];
        let cmd = parse_repl_line(&parts, 0).unwrap();
        match cmd {
            ReplCmd::Help(Some(t)) => assert_eq!(t, "add"),
            _ => panic!("Expected Help(Some)"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        let parts = vec!["foobar"];
        let result = parse_repl_line(&parts, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown command"));
    }

    #[test]
    fn test_strip_tool_prefix() {
        assert_eq!(strip_tool_prefix("zen install numpy"), "install numpy");
        assert_eq!(strip_tool_prefix("pip install numpy"), "install numpy");
        assert_eq!(strip_tool_prefix("uv pip install numpy"), "install numpy");
        assert_eq!(strip_tool_prefix("uv install numpy"), "install numpy");
        assert_eq!(strip_tool_prefix("conda install numpy"), "install numpy");
        assert_eq!(strip_tool_prefix("add numpy"), "add numpy");
    }

    #[test]
    fn test_parse_wheel() {
        let parts = vec!["add", "mypkg", "--wheel", "/tmp/mypkg.whl"];
        let cmd = parse_repl_line(&parts, 0).unwrap();
        match cmd {
            ReplCmd::Add(args) => {
                assert_eq!(args.wheel_path.unwrap(), "/tmp/mypkg.whl");
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_resolve_index_positive() {
        assert_eq!(resolve_index(2, 5, "--at").unwrap(), 2);
    }

    #[test]
    fn test_resolve_index_negative_valid() {
        assert_eq!(resolve_index(-1, 5, "--at").unwrap(), 4);
        assert_eq!(resolve_index(-3, 5, "--at").unwrap(), 2);
        assert_eq!(resolve_index(-5, 5, "--at").unwrap(), 0);
    }

    #[test]
    fn test_resolve_index_negative_out_of_range() {
        assert!(resolve_index(-6, 5, "--at").is_err());
        assert!(resolve_index(-1, 0, "--at").is_err());
    }
}
