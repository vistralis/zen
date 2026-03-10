// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::types::EnvName;
use crate::utils;

use colored::*;
use std::error::Error;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn run(
    db: &Database,
    ops: &crate::ops::ZenOps,
    name: EnvName,
    user_python: Option<String>,
    template: Option<String>,
    strict: bool,
    rm: bool,
    rest: Vec<String>,
    home: &Path,
) -> Result<(), Box<dyn Error>> {
    // Typo detection: catch reversed command order
    if name.as_str() == "template" {
        let hint = if let Some(actual_name) = rest.first() {
            format!("zen template create {}", actual_name)
        } else {
            "zen template create <name>".to_string()
        };
        eprintln!("{} Did you mean {}?", "Hint:".yellow().bold(), hint.cyan());
        std::process::exit(1);
    }
    // Validate inputs
    crate::validation::validate_name(&name, "Environment")?;
    if let Some(ref py) = user_python {
        crate::validation::validate_python_version(py)?;
    }

    let mut python = user_python.clone().unwrap_or_else(|| "3.12".to_string());
    let env_path = home.join(name.as_str());

    // Guard: check if environment already exists
    let existing = db.list_envs()?;
    if existing.iter().any(|(n, ..)| n.as_str() == name.as_str()) {
        if rm {
            println!(
                "Removing existing environment '{}'...",
                name.as_str().dimmed()
            );
            if let Err(e) = ops.remove_env(&name) {
                eprintln!("{} {}", "Error:".red(), e);
                return Ok(());
            }
        } else {
            eprintln!(
                "{} Environment '{}' already exists. Use {} or {} to replace it.",
                "Error:".red(),
                name,
                "zen rm".bold(),
                "--rm".bold()
            );
            return Ok(());
        }
    }
    if env_path.exists() && !rm {
        eprintln!(
            "{} Directory '{}' already exists. Remove it or choose a different name.",
            "Error:".red(),
            env_path.display()
        );
        return Ok(());
    } else if env_path.exists() && rm {
        std::fs::remove_dir_all(&env_path)?;
    }

    // Validate templates before starting creation
    let mut templates_to_apply = Vec::new();
    let mut first_tpl_python: Option<String> = None;
    if let Some(t_str) = template {
        let parts = utils::parse_template_string(&t_str);
        for part in parts {
            if let Some(t_id) = db.get_template_id(&part.name, &part.version)? {
                if user_python.is_none()
                    && let Ok(all_tpls) = db.list_templates()
                    && let Some(t_info) = all_tpls
                        .iter()
                        .find(|t| t.0 == part.name && t.1 == part.version)
                {
                    if first_tpl_python.is_none() {
                        python = t_info.2.clone();
                        first_tpl_python = Some(t_info.2.clone());
                    } else if first_tpl_python.as_deref() != Some(&t_info.2) {
                        eprintln!(
                            "  {} Template '{}:{}' uses Python {} but first template uses Python {} — using {}",
                            "⚠".yellow(),
                            part.name,
                            part.version,
                            t_info.2,
                            first_tpl_python.as_deref().unwrap_or("?"),
                            first_tpl_python.as_deref().unwrap_or("?")
                        );
                    }
                }
                templates_to_apply.push((t_id, part.name, part.version));
            } else {
                eprintln!(
                    "{} Template '{}:{}' not found. Use {} to see available templates.",
                    "Error:".red(),
                    part.name,
                    part.version,
                    "zen template list".bold()
                );
                std::process::exit(1);
            }
        }
    }

    // Deduplicate
    templates_to_apply.dedup_by(|a, b| a.1 == b.1 && a.2 == b.2);

    println!("Creating environment '{}'...", name.cyan());

    std::fs::create_dir_all(home)?;

    // Ordering: Python -> NumPy -> Torch -> others
    templates_to_apply.sort_by_key(|(_, name, _)| match name.to_lowercase().as_str() {
        "python" | "py" => 0,
        "numpy" => 1,
        "torch" | "pytorch" => 2,
        _ => 3,
    });

    // If a python template is present, use its version
    for (_, name, _) in &templates_to_apply {
        if name.to_lowercase() == "python" || name.to_lowercase() == "py" {
            // Reserved for future python version inheritance
        }
    }

    // Try to use uv if available, otherwise fallback to venv
    let status = if let Ok(uv_path) = which::which("uv") {
        std::process::Command::new(uv_path)
            .arg("venv")
            .arg(&env_path)
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
            .arg(&env_path)
            .arg("--clear")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?
    };

    if status.success() {
        let env_str = env_path.to_str().unwrap();

        // Silent bootstrap
        if let Ok(_uv_path) = which::which("uv") {
            utils::run_in_env_silent(env_str, "uv", &["pip", "install", "uv", "setuptools"]);
        } else {
            utils::run_in_env_silent(
                env_str,
                "pip",
                &["install", "--upgrade", "pip", "setuptools"],
            );
        }

        // Save template info for logging
        let tpl_log_info: String = if !templates_to_apply.is_empty() {
            format!(
                " --template {}",
                templates_to_apply
                    .iter()
                    .map(|(_, n, v)| format!("{}:{}", n, v))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        } else {
            String::new()
        };

        // Apply templates
        let mut installed_pkgs: std::collections::HashMap<
            String,
            (String, String, Option<String>),
        > = std::collections::HashMap::new();

        for (t_id, t_name, t_ver) in templates_to_apply {
            println!("Applying template '{}:{}'...", t_name, t_ver);
            let packages = db.get_template_packages(t_id)?;

            // Detect conflicts
            for (p_name, p_ver, _, _, pkg_install_args, _step) in &packages {
                let pkg_lower = p_name.to_lowercase();
                if let Some((prev_ver, prev_tpl, prev_args)) = installed_pkgs.get(&pkg_lower) {
                    if prev_args != pkg_install_args {
                        eprintln!(
                            "  {} '{}' will be reinstalled from a different index (was in '{}', now in '{}:{}').",
                            "⚠ Index conflict:".yellow().bold(),
                            p_name,
                            prev_tpl,
                            t_name,
                            t_ver
                        );
                    } else if prev_ver != p_ver {
                        eprintln!(
                            "  {} '{}' {}→{} (was in '{}', overridden by '{}:{}')",
                            "⚠ Override:".yellow(),
                            p_name,
                            prev_ver.dimmed(),
                            p_ver.yellow(),
                            prev_tpl,
                            t_name,
                            t_ver
                        );
                    }
                }
            }

            // Group packages by install_args
            let mut pkg_groups: std::collections::HashMap<Option<String>, Vec<String>> =
                std::collections::HashMap::new();

            for (p_name, p_ver, is_pinned, itype, pkg_install_args, _step) in packages {
                // Wheel path validation
                if itype == "wheel"
                    && let Some(ref wheel_path) = pkg_install_args
                    && !std::path::Path::new(wheel_path).exists()
                {
                    eprintln!(
                        "  {} Wheel file for '{}' not found: {}",
                        "✗".red(),
                        p_name,
                        wheel_path.red()
                    );
                    eprintln!(
                        "    Fix with: {} or {}",
                        format!("zen template edit {}:{} drop {}", t_name, t_ver, p_name).cyan(),
                        format!(
                            "zen template edit {}:{} add {} --wheel /new/path.whl",
                            t_name, t_ver, p_name
                        )
                        .cyan()
                    );
                    continue;
                }

                let pkg_spec = if itype == "wheel" {
                    pkg_install_args.clone().unwrap_or_else(|| {
                        if strict || is_pinned {
                            format!("{}=={}", p_name, p_ver)
                        } else {
                            p_name.clone()
                        }
                    })
                } else if strict || is_pinned {
                    format!("{}=={}", p_name, p_ver)
                } else {
                    p_name.clone()
                };
                installed_pkgs.insert(
                    p_name.to_lowercase(),
                    (
                        p_ver,
                        format!("{}:{}", t_name, t_ver),
                        pkg_install_args.clone(),
                    ),
                );
                let group_key = if itype == "wheel" {
                    None
                } else {
                    pkg_install_args
                };
                pkg_groups.entry(group_key).or_default().push(pkg_spec);
            }

            // Install each group
            for (group_args, group_pkgs) in pkg_groups {
                if group_pkgs.is_empty() {
                    continue;
                }
                let mut cmd_args = vec!["pip", "install"];
                if let Some(ref args_str) = group_args {
                    for arg in args_str.split_whitespace() {
                        cmd_args.push(arg);
                    }
                }
                for pkg in &group_pkgs {
                    cmd_args.push(pkg);
                }
                if which::which("uv").is_ok() {
                    utils::run_in_env(env_str, "uv", &cmd_args);
                } else {
                    utils::run_in_env(env_str, "pip", &cmd_args[1..]);
                }
            }
        }

        let py_ver = utils::read_python_version(env_path.to_str().unwrap()).unwrap_or(python);

        let _env_id = db.register_env(&name, env_path.to_str().unwrap(), &py_ver)?;

        println!(
            "{} Environment '{}' created. (Python {})",
            "✓".green(),
            name.cyan(),
            py_ver.dimmed()
        );
        println!(
            "  Activate: {} ({})",
            format!("zen activate {}", name).bold(),
            format!("za {}", name).dimmed()
        );
        activity_log::log_activity(
            "cli",
            "create",
            &format!("{} (Python {}){}", name, py_ver, tpl_log_info),
        );
    } else {
        eprintln!("Failed to create environment.");
    }
    Ok(())
}
