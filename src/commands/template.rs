// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::repl;
use crate::utils;

use colored::*;
use std::error::Error;

/// Interactive REPL for template create/edit sessions.
pub fn template_repl(
    db: &Database,
    template_id: i64,
    template_name: &str,
    template_version: &str,
    env_path: &str,
    is_new: bool,
) -> Result<(), Box<dyn Error>> {
    use rustyline::error::ReadlineError;

    let mut steps = if is_new {
        Vec::new()
    } else {
        repl::load_steps_from_db(db, template_id)
    };

    let prompt = format!("{}:{}> ", template_name, template_version);
    let use_uv = which::which("uv").is_ok();

    let history_path = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/share/zen/repl_history"))
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/zen_repl_history"));

    let mut rl = rustyline::DefaultEditor::new()?;
    let _ = rl.load_history(&history_path);

    // Pre-seed with install commands from bash history
    if let Ok(home) = std::env::var("HOME") {
        let bash_hist = std::path::PathBuf::from(&home).join(".bash_history");
        if let Ok(contents) = std::fs::read_to_string(&bash_hist) {
            let install_patterns = [
                "pip install",
                "uv pip install",
                "zen install",
                "conda install",
            ];
            for line in contents.lines() {
                let line = line.trim();
                if !line.is_empty()
                    && !line.starts_with('#')
                    && install_patterns.iter().any(|p| line.contains(p))
                {
                    let _ = rl.add_history_entry(line);
                }
            }
        }
    }

    let cleanup = |db: &Database, tid: i64| {
        let _ = db.clear_sessions();
        if is_new
            && let Ok(pkgs) = db.get_template_packages(tid)
            && pkgs.is_empty()
        {
            let _ = db.delete_template_by_id(tid);
        }
        let _ = std::fs::remove_dir_all(env_path);
    };

    repl::print_status(&steps, template_name, template_version);

    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                rl.add_history_entry(line)?;

                let line = repl::strip_tool_prefix(line);
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match repl::parse_repl_line(&parts, steps.len()) {
                    Err(msg) => {
                        if !msg.is_empty() {
                            eprintln!("  {} {}", "✗".red(), msg);
                        }
                    }
                    Ok(repl::ReplCmd::Help(topic)) => {
                        repl::handle_help(topic.as_deref());
                    }
                    Ok(repl::ReplCmd::List) => {
                        repl::print_status(&steps, template_name, template_version);
                    }
                    Ok(repl::ReplCmd::Add(args)) => {
                        if let Err(e) = repl::handle_add(&mut steps, args, env_path, use_uv) {
                            eprintln!("  {} {}", "✗".red(), e);
                        }
                        repl::print_status(&steps, template_name, template_version);
                    }
                    Ok(repl::ReplCmd::Drop(target)) => {
                        if let Err(e) = repl::handle_drop(&mut steps, &target) {
                            eprintln!("  {} {}", "✗".red(), e);
                        }
                        repl::print_status(&steps, template_name, template_version);
                    }
                    Ok(repl::ReplCmd::Save) => {
                        if let Err(e) = repl::handle_save(
                            db,
                            template_id,
                            &steps,
                            template_name,
                            template_version,
                        ) {
                            eprintln!("  {} Save failed: {}", "✗".red(), e);
                            continue;
                        }
                        let _ = std::fs::remove_dir_all(env_path);
                        let _ = rl.save_history(&history_path);
                        return Ok(());
                    }
                    Ok(repl::ReplCmd::Quit) => {
                        cleanup(db, template_id);
                        println!("\n  Session discarded.\n");
                        let _ = rl.save_history(&history_path);
                        return Ok(());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                cleanup(db, template_id);
                println!("\n  Session aborted.\n");
                let _ = rl.save_history(&history_path);
                return Ok(());
            }
            Err(ReadlineError::Eof) => {
                cleanup(db, template_id);
                println!("\n  Session aborted.\n");
                let _ = rl.save_history(&history_path);
                return Ok(());
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

pub fn run_create(
    db: &Database,
    name: String,
    user_python: Option<String>,
) -> Result<(), Box<dyn Error>> {
    // Validate inputs
    crate::validation::validate_name(name.split(':').next().unwrap_or(&name), "Template")?;
    if let Some(ref py) = user_python {
        crate::validation::validate_python_version(py)?;
    }

    let python = user_python.unwrap_or_else(|| "3.12".to_string());
    if !db.clear_stale_session()? {
        eprintln!("A recording session is already active. Please save or exit first.");
        return Ok(());
    }

    let mut parts = name.splitn(2, ':');
    let t_name = parts.next().unwrap();
    let t_ver = parts.next().unwrap_or("default");

    let (temp_id, is_new) = db.create_template(t_name, t_ver, &python)?;
    let tmp_env = std::env::temp_dir().join(format!("zen_tpl_{}_{}", t_name, t_ver));
    println!(
        "{}",
        if is_new {
            format!("Creating template environment at {}...", tmp_env.display())
        } else {
            format!(
                "Editing template '{}:{}' (environment at {})...",
                t_name,
                t_ver,
                tmp_env.display()
            )
        }
    );

    let status = if let Ok(uv_path) = which::which("uv") {
        std::process::Command::new(uv_path)
            .arg("venv")
            .arg(&tmp_env)
            .arg("--python")
            .arg(&python)
            .arg("--clear")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?
    } else {
        std::process::Command::new("python3")
            .arg("-m")
            .arg("venv")
            .arg(&tmp_env)
            .arg("--clear")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?
    };

    if status.success() {
        let env_str = tmp_env.to_str().unwrap();
        if which::which("uv").is_ok() {
            utils::run_in_env_silent(env_str, "uv", &["pip", "install", "uv", "setuptools"]);
        } else {
            utils::run_in_env_silent(
                env_str,
                "pip",
                &["install", "--upgrade", "pip", "setuptools"],
            );
        }
        db.start_session(temp_id, env_str)?;
        template_repl(db, temp_id, t_name, t_ver, env_str, is_new)?;
    } else {
        eprintln!("{} Failed to create template environment.", "✗".red());
    }
    Ok(())
}

pub fn run_save(db: &Database) -> Result<(), Box<dyn Error>> {
    if let Some((t_id, path, _)) = db.get_active_session()? {
        let session_pkgs = db.get_template_packages(t_id)?;
        let count = session_pkgs.len();

        if count == 0 {
            eprintln!("No packages were installed during this session. Nothing to save.");
            eprintln!("Use {} to add packages first.", "zen install <pkg>".cyan());
            return Ok(());
        }

        std::fs::remove_dir_all(&path).ok();
        db.clear_sessions()?;

        activity_log::log_activity("cli", "template:save", &format!("{} pkgs", count));
        println!("Template saved successfully ({} packages).", count);
    } else {
        eprintln!("No active recording session found.");
    }
    Ok(())
}

pub fn run_exit(db: &Database) -> Result<(), Box<dyn Error>> {
    if let Some((_, path, _)) = db.get_active_session()? {
        println!("Aborting session. Cleaning up {}...", path);
        std::fs::remove_dir_all(path).ok();
        db.clear_sessions()?;
        println!("Session exited.");
    } else {
        eprintln!("No active recording session found.");
    }
    Ok(())
}

pub fn run_list(db: &Database, name: Option<String>) -> Result<(), Box<dyn Error>> {
    let pattern = name.as_deref();
    let templates = db.get_all_templates_with_packages()?;
    let templates: Vec<_> = if let Some(pat) = pattern {
        let pat_lower = pat.to_lowercase();
        templates
            .into_iter()
            .filter(|(n, _, _, _)| n.to_lowercase().contains(&pat_lower))
            .collect()
    } else {
        templates
    };
    use comfy_table::{
        Attribute, Cell, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
        presets::UTF8_FULL_CONDENSED,
    };
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Name").add_attribute(Attribute::Bold),
        Cell::new("Version").add_attribute(Attribute::Bold),
        Cell::new("Python").add_attribute(Attribute::Bold),
        Cell::new("Packages").add_attribute(Attribute::Bold),
    ]);

    for (n, v, p, pkgs) in templates {
        table.add_row(vec![n, v, p, pkgs.len().to_string()]);
    }
    println!("{}", table);
    Ok(())
}

pub fn run_rm(db: &Database, name: &str) -> Result<(), Box<dyn Error>> {
    let mut parts = name.splitn(2, ':');
    let t_name = parts.next().unwrap();
    let t_ver = parts.next();

    if let Some(ver) = t_ver {
        // Specific version: zen template rm numpy:1
        match db.get_template_id(t_name, ver)? {
            Some(id) => {
                db.delete_template_by_id(id)?;
                activity_log::log_activity("cli", "template:rm", &format!("{}:{}", t_name, ver));
                println!("{} Template '{}:{}' deleted.", "✓".green(), t_name, ver);
            }
            None => {
                println!("{} Template '{}:{}' not found.", "✗".red(), t_name, ver);
            }
        }
    } else {
        // No version: zen template rm numpy (deletes all versions)
        if db.delete_template(t_name)? {
            activity_log::log_activity("cli", "template:rm", t_name);
            println!("{} Template '{}' deleted.", "✓".green(), t_name);
        } else {
            println!("{} Template '{}' not found.", "✗".red(), t_name);
        }
    }
    Ok(())
}

pub fn run_rm_all(db: &Database) -> Result<(), Box<dyn Error>> {
    let count = db.delete_all_templates()?;
    if count > 0 {
        activity_log::log_activity("cli", "template:rm:all", &format!("{} templates", count));
        println!("{} Deleted all templates ({} removed).", "✓".green(), count);
    } else {
        println!("No templates to delete.");
    }
    Ok(())
}

pub fn run_inspect(db: &Database, name: &str) -> Result<(), Box<dyn Error>> {
    let mut parts = name.splitn(2, ':');
    let t_name = parts.next().unwrap();
    let t_ver = parts.next().unwrap_or("default");

    let t_id = db.get_template_id(t_name, t_ver)?;
    match t_id {
        None => {
            eprintln!("{} Template '{}:{}' not found.", "✗".red(), t_name, t_ver);
        }
        Some(id) => {
            let packages = db.get_template_packages(id)?;
            let meta = db.get_template_by_id(id)?;
            let py_ver = meta.as_ref().map(|(_, _, p)| p.as_str()).unwrap_or("?");

            let mut steps: std::collections::BTreeMap<
                i64,
                Vec<&(String, String, bool, String, Option<String>, i64)>,
            > = std::collections::BTreeMap::new();
            for pkg in &packages {
                steps.entry(pkg.5).or_default().push(pkg);
            }

            println!(
                "\n{} {}:{} — Python {} — {} step(s), {} package(s)\n",
                "●".bold(),
                t_name.bold(),
                t_ver,
                py_ver,
                steps.len(),
                packages.len()
            );

            for (step_num, step_pkgs) in &steps {
                let step_args = step_pkgs
                    .first()
                    .and_then(|p| p.4.as_deref())
                    .filter(|a| !a.is_empty());

                if let Some(args) = step_args {
                    if step_pkgs.first().map(|p| p.3.as_str()) != Some("wheel") {
                        println!(
                            "  {} {} ─ {}",
                            format!("Step {}", step_num).bold(),
                            "".dimmed(),
                            args.dimmed()
                        );
                    } else {
                        println!("  {}", format!("Step {}", step_num).bold());
                    }
                } else {
                    println!("  {}", format!("Step {}", step_num).bold());
                }

                for pkg in step_pkgs {
                    let name_col = format!("    {:<24}", pkg.0);
                    let ver_col = format!("{:<20}", pkg.1);
                    let type_col = &pkg.3;

                    if type_col == "wheel" {
                        let wheel_path = pkg.4.as_deref().unwrap_or("(unknown path)");
                        let exists = std::path::Path::new(wheel_path).exists();
                        if exists {
                            println!(
                                "{}{}{}  {}",
                                name_col,
                                ver_col,
                                "wheel".cyan(),
                                wheel_path.green()
                            );
                        } else {
                            println!(
                                "{}{}{}  {} {}",
                                name_col,
                                ver_col,
                                "wheel".cyan(),
                                wheel_path.red(),
                                "← missing".red().bold()
                            );
                        }
                    } else {
                        println!("{}{}{}", name_col, ver_col, type_col.dimmed());
                    }
                }
                println!();
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_edit(
    db: &Database,
    name: &str,
    action: Option<String>,
    args: Vec<String>,
    step: Option<i64>,
    wheel: Option<String>,
    index_url: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let mut parts = name.splitn(2, ':');
    let t_name = parts.next().unwrap();
    let t_ver = parts.next().unwrap_or("default");

    let t_id = match db.get_template_id(t_name, t_ver)? {
        Some(id) => id,
        None => {
            eprintln!("{} Template '{}:{}' not found.", "✗".red(), t_name, t_ver);
            return Ok(());
        }
    };

    match action.as_deref() {
        Some("drop") => {
            if args.is_empty() {
                eprintln!("Usage: zen template edit <name> drop <package_name|step_number>");
                return Ok(());
            }
            let target = &args[0];
            if let Ok(step_num) = target.parse::<i64>() {
                let removed = db.remove_template_step(t_id, step_num)?;
                if removed > 0 {
                    println!(
                        "{} Removed step {} ({} package(s)) from '{}:{}'.",
                        "✓".green(),
                        step_num,
                        removed,
                        t_name,
                        t_ver
                    );
                } else {
                    eprintln!(
                        "{} Step {} not found in '{}:{}'.",
                        "✗".red(),
                        step_num,
                        t_name,
                        t_ver
                    );
                }
            } else if db.remove_template_package(t_id, target)? {
                println!(
                    "{} Removed '{}' from '{}:{}'.",
                    "✓".green(),
                    target,
                    t_name,
                    t_ver
                );
            } else {
                eprintln!(
                    "{} Package '{}' not found in '{}:{}'.",
                    "✗".red(),
                    target,
                    t_name,
                    t_ver
                );
            }
        }
        Some("add") => {
            if args.is_empty() {
                eprintln!(
                    "Usage: zen template edit <name> add <packages...> [--step N] [--wheel /path] [--index-url URL]"
                );
                return Ok(());
            }
            let target_step = if let Some(s) = step {
                s
            } else {
                db.get_next_step(t_id)?
            };

            let install_args = if let Some(ref url) = index_url {
                Some(format!("--index-url {}", url))
            } else if step.is_some() {
                let pkgs = db.get_template_packages(t_id)?;
                pkgs.iter()
                    .find(|p| p.5 == target_step)
                    .and_then(|p| p.4.clone())
            } else {
                None
            };

            for pkg_name in &args {
                let (name, ver, itype, iargs) = if let Some(ref whl) = wheel {
                    let whl_name =
                        utils::normalize_wheel_name(whl).unwrap_or_else(|| pkg_name.clone());
                    (whl_name, "0.0.0".to_string(), "wheel", Some(whl.clone()))
                } else {
                    (
                        pkg_name.clone(),
                        "default".to_string(),
                        "pypi",
                        install_args.clone(),
                    )
                };
                db.add_template_package(
                    t_id,
                    &name,
                    &ver,
                    true,
                    itype,
                    iargs.as_deref(),
                    target_step,
                )?;
                println!(
                    "{} Added '{}' to '{}:{}' step {}.",
                    "✓".green(),
                    name,
                    t_name,
                    t_ver,
                    target_step
                );
            }
        }
        Some(other) => {
            eprintln!(
                "{} Unknown action '{}'. Use 'add' or 'drop'.",
                "✗".red(),
                other
            );
        }
        None => {
            // Interactive mode
            if !db.clear_stale_session()? {
                eprintln!("A recording session is already active. Please save or exit first.");
                return Ok(());
            }

            let meta = db.get_template_by_id(t_id)?;
            let python = meta
                .as_ref()
                .map(|(_, _, p)| p.clone())
                .unwrap_or_else(|| "3.12".to_string());

            let tmp_env = std::env::temp_dir().join(format!("zen_tpl_edit_{}_{}", t_name, t_ver));

            println!("Rebuilding environment for '{}:{}'...", t_name, t_ver);

            let status = if let Ok(uv_path) = which::which("uv") {
                std::process::Command::new(uv_path)
                    .arg("venv")
                    .arg(&tmp_env)
                    .arg("--python")
                    .arg(&python)
                    .arg("--clear")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()?
            } else {
                std::process::Command::new("python3")
                    .arg("-m")
                    .arg("venv")
                    .arg(&tmp_env)
                    .arg("--clear")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()?
            };

            if !status.success() {
                eprintln!("{} Failed to create edit environment.", "✗".red());
                return Ok(());
            }

            let env_str = tmp_env.to_str().unwrap();
            let use_uv = which::which("uv").is_ok();

            if use_uv {
                utils::run_in_env_silent(env_str, "uv", &["pip", "install", "uv", "setuptools"]);
            } else {
                utils::run_in_env_silent(
                    env_str,
                    "pip",
                    &["install", "--upgrade", "pip", "setuptools"],
                );
            }

            // Replay existing template steps
            let packages = db.get_template_packages(t_id)?;
            if !packages.is_empty() {
                let mut replay_steps: std::collections::BTreeMap<
                    i64,
                    (Option<String>, Vec<(String, String)>),
                > = std::collections::BTreeMap::new();
                for (p_name, _p_ver, _pinned, itype, iargs, step) in &packages {
                    let entry = replay_steps
                        .entry(*step)
                        .or_insert_with(|| (iargs.clone(), Vec::new()));
                    entry.1.push((p_name.clone(), itype.clone()));
                }

                for (step_num, (install_args_opt, pkgs)) in &replay_steps {
                    let mut cmd_args: Vec<String> = if use_uv {
                        vec!["pip".to_string(), "install".to_string()]
                    } else {
                        vec!["install".to_string()]
                    };

                    if let Some(ia) = install_args_opt {
                        let ia_parts: Vec<&str> = ia.split_whitespace().collect();
                        for a in &ia_parts {
                            cmd_args.push(a.to_string());
                        }
                    }

                    for (pkg_name, itype) in pkgs {
                        if itype == "wheel"
                            && let Some(ia) = install_args_opt
                        {
                            if !cmd_args.iter().any(|a| a.ends_with(".whl")) {
                                cmd_args.push(ia.clone());
                            }
                        } else {
                            cmd_args.push(pkg_name.clone());
                        }
                    }

                    let installer = if use_uv { "uv" } else { "pip" };
                    let args_ref: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();

                    print!("  Replaying step {}...", step_num);
                    let ok = utils::run_in_env_silent(env_str, installer, &args_ref);
                    if ok {
                        println!(" {}", "✓".green());
                    } else {
                        println!(" {}", "✗".red());
                        eprintln!(
                            "  {} Step {} replay failed. Entering REPL anyway — use 'drop {}' to remove it.",
                            "⚠".yellow(),
                            step_num,
                            step_num
                        );
                    }
                }
            }

            db.start_session(t_id, env_str)?;
            template_repl(db, t_id, t_name, t_ver, env_str, false)?;
        }
    }
    Ok(())
}

pub fn run_drop(db: &Database, target: &str) -> Result<(), Box<dyn Error>> {
    match db.get_active_session()? {
        None => {
            eprintln!(
                "{} No active session. Use this during {} or {}.",
                "✗".red(),
                "zen template create".cyan(),
                "zen template edit".cyan()
            );
        }
        Some((t_id, _path, _)) => {
            if let Ok(step_num) = target.parse::<i64>() {
                let removed = db.remove_template_step(t_id, step_num)?;
                if removed > 0 {
                    println!(
                        "{} Dropped step {} ({} package(s)).",
                        "✓".green(),
                        step_num,
                        removed
                    );
                } else {
                    eprintln!(
                        "{} Step {} not found in current session.",
                        "✗".red(),
                        step_num
                    );
                }
            } else if db.remove_template_package(t_id, target)? {
                println!("{} Dropped '{}' from session.", "✓".green(), target);
            } else {
                eprintln!(
                    "{} Package '{}' not found in current session.",
                    "✗".red(),
                    target
                );
            }
        }
    }
    Ok(())
}

pub fn run_export_tpl(
    db: &Database,
    name: &str,
    output: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let mut parts = name.splitn(2, ':');
    let t_name = parts.next().unwrap();
    let t_ver = parts.next().unwrap_or("default");

    let t_id = match db.get_template_id(t_name, t_ver)? {
        Some(id) => id,
        None => {
            eprintln!("{} Template '{}:{}' not found.", "✗".red(), t_name, t_ver);
            return Ok(());
        }
    };

    let meta = db.get_template_by_id(t_id)?;
    let py_ver = meta.as_ref().map(|(_, _, p)| p.as_str()).unwrap_or("3.12");
    let packages = db.get_template_packages(t_id)?;

    // Group packages by step
    let mut steps: std::collections::BTreeMap<i64, (Option<String>, Vec<toml::Value>)> =
        std::collections::BTreeMap::new();
    for (p_name, p_ver, _pinned, itype, iargs, step) in &packages {
        let entry = steps
            .entry(*step)
            .or_insert_with(|| (iargs.clone(), Vec::new()));
        let mut pkg = toml::map::Map::new();
        pkg.insert("name".to_string(), toml::Value::String(p_name.clone()));
        pkg.insert("version".to_string(), toml::Value::String(p_ver.clone()));
        if itype != "pypi" {
            pkg.insert("type".to_string(), toml::Value::String(itype.clone()));
        }
        if itype == "wheel"
            && let Some(path) = iargs
        {
            pkg.insert("path".to_string(), toml::Value::String(path.clone()));
        }
        entry.1.push(toml::Value::Table(pkg));
    }

    // Build TOML structure
    let mut doc = toml::map::Map::new();

    let mut tpl = toml::map::Map::new();
    tpl.insert("name".to_string(), toml::Value::String(t_name.to_string()));
    tpl.insert(
        "version".to_string(),
        toml::Value::String(t_ver.to_string()),
    );
    tpl.insert(
        "python".to_string(),
        toml::Value::String(py_ver.to_string()),
    );
    doc.insert("template".to_string(), toml::Value::Table(tpl));

    let mut step_arr = Vec::new();
    for (install_args, pkgs) in steps.values() {
        let mut step_table = toml::map::Map::new();
        if let Some(ia) = install_args {
            let ia_parts: Vec<&str> = ia.split_whitespace().collect();
            for i in 0..ia_parts.len() {
                if ia_parts[i] == "--index-url"
                    && let Some(url) = ia_parts.get(i + 1)
                {
                    step_table.insert(
                        "index_url".to_string(),
                        toml::Value::String(url.to_string()),
                    );
                }
                if ia_parts[i] == "--extra-index-url"
                    && let Some(url) = ia_parts.get(i + 1)
                {
                    step_table.insert(
                        "extra_index_url".to_string(),
                        toml::Value::String(url.to_string()),
                    );
                }
            }
        }
        step_table.insert("packages".to_string(), toml::Value::Array(pkgs.clone()));
        step_arr.push(toml::Value::Table(step_table));
    }
    doc.insert("step".to_string(), toml::Value::Array(step_arr));

    let toml_str = toml::to_string_pretty(&toml::Value::Table(doc))?;

    let out_path = output.unwrap_or_else(|| format!("{}.toml", t_name));
    std::fs::write(&out_path, &toml_str)?;
    println!(
        "{} Exported '{}:{}' → {}",
        "✓".green(),
        t_name,
        t_ver,
        out_path.cyan()
    );
    Ok(())
}

pub fn run_import_tpl(db: &Database, file: &str) -> Result<(), Box<dyn Error>> {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Cannot read '{}': {}", "✗".red(), file, e);
            return Ok(());
        }
    };
    let doc: toml::Value = match content.parse() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} TOML parse error: {}", "✗".red(), e);
            return Ok(());
        }
    };

    let tpl = match doc.get("template") {
        Some(t) => t,
        None => {
            eprintln!("{} Missing [template] section in TOML.", "✗".red());
            return Ok(());
        }
    };
    let t_name = match tpl.get("name").and_then(|v: &toml::Value| v.as_str()) {
        Some(n) => n,
        None => {
            eprintln!("{} Missing template.name in TOML.", "✗".red());
            return Ok(());
        }
    };
    let t_ver = tpl
        .get("version")
        .and_then(|v: &toml::Value| v.as_str())
        .unwrap_or("default");
    let py_ver = tpl
        .get("python")
        .and_then(|v: &toml::Value| v.as_str())
        .unwrap_or("3.12");

    if let Some(existing_id) = db.get_template_id(t_name, t_ver)? {
        db.delete_template_by_id(existing_id)?;
    }

    let (t_id, _) = db.create_template(t_name, t_ver, py_ver)?;

    let steps = match doc.get("step").and_then(|v: &toml::Value| v.as_array()) {
        Some(s) => s,
        None => {
            eprintln!("{} Missing [[step]] array in TOML.", "✗".red());
            return Ok(());
        }
    };

    let mut total_pkgs = 0usize;
    for (step_num, step_val) in steps.iter().enumerate() {
        let step_tbl = step_val.as_table();
        let mut install_parts = Vec::new();
        if let Some(tbl) = step_tbl {
            if let Some(url) = tbl.get("index_url").and_then(|v| v.as_str()) {
                install_parts.push(format!("--index-url {}", url));
            }
            if let Some(url) = tbl.get("extra_index_url").and_then(|v| v.as_str()) {
                install_parts.push(format!("--extra-index-url {}", url));
            }
        }
        let install_args = if install_parts.is_empty() {
            None
        } else {
            Some(install_parts.join(" "))
        };

        if let Some(tbl) = step_tbl
            && let Some(pkgs) = tbl.get("packages").and_then(|v| v.as_array())
        {
            for pkg in pkgs {
                let pkg_tbl = match pkg.as_table() {
                    Some(t) => t,
                    None => continue,
                };
                let name = pkg_tbl
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let version = pkg_tbl
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let itype = pkg_tbl
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pypi");
                let iargs = if itype == "wheel" {
                    pkg_tbl
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    install_args.clone()
                };

                db.add_template_package(
                    t_id,
                    name,
                    version,
                    true,
                    itype,
                    iargs.as_deref(),
                    step_num as i64,
                )?;
                total_pkgs += 1;
            }
        }
    }

    println!(
        "{} Imported '{}:{}' from {} ({} package(s), {} step(s)).",
        "✓".green(),
        t_name,
        t_ver,
        file.cyan(),
        total_pkgs,
        steps.len()
    );
    Ok(())
}
