// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::utils;
use colored::*;
use std::error::Error;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn run(
    db: &Database,
    packages: &[String],
    env: Option<String>,
    cli_index_url: Option<String>,
    extra_index_url: Option<String>,
    editable: bool,
    pre: bool,
    upgrade: bool,
    dry_run: bool,
    resolve_env_name_fn: impl FnOnce() -> Result<String, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let (target_id, target_path, is_session) = if let Some(session) = db.get_active_session()? {
        (Some(session.0), session.1, true)
    } else if let Some(env_name) = env {
        let envs = db.list_envs()?;
        let e = envs
            .iter()
            .find(|(n, ..)| n == &env_name)
            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;
        let id = db
            .get_env_id(&env_name)?
            .ok_or_else(|| format!("Environment '{}' not found in database", env_name))?;
        (Some(id), e.1.clone(), false)
    } else {
        // Fall back: try to resolve from $VIRTUAL_ENV
        let resolved = resolve_env_name_fn()?;
        let envs = db.list_envs()?;
        let e = envs
            .iter()
            .find(|(n, ..)| n == &resolved)
            .ok_or_else(|| format!("Environment '{}' not found", resolved))?;
        let id = db
            .get_env_id(&resolved)?
            .ok_or_else(|| format!("Environment '{}' not found in database", resolved))?;
        (Some(id), e.1.clone(), false)
    };

    println!("Installing packages in {}...", target_path);

    let mut final_args = Vec::new();
    let mut index_url = cli_index_url.clone();

    for pkg in packages {
        if pkg.starts_with("torch-cu") {
            let cuda_ver = pkg.trim_start_matches("torch-cu");
            // Map common aliases (e.g. 130 -> 13.0)
            let normalized_cuda = if cuda_ver.len() == 3 {
                format!("{}.{}", &cuda_ver[0..2], &cuda_ver[2..])
            } else {
                cuda_ver.to_string()
            };

            if let Some(url) = utils::get_torch_index_url(&normalized_cuda) {
                index_url = Some(url.to_string());
                final_args.push("torch".to_string());
                final_args.push("torchvision".to_string());
                final_args.push("torchaudio".to_string());
            } else {
                final_args.push(pkg.clone());
            }
        } else {
            final_args.push(pkg.clone());
        }
    }

    let mut cmd_args = vec!["pip", "install"];

    // Add pip-compatible flags
    if editable {
        cmd_args.push("-e");
    }
    if pre {
        cmd_args.push("--pre");
    }
    if upgrade {
        cmd_args.push("--upgrade");
    }
    if dry_run {
        cmd_args.push("--dry-run");
    }
    if let Some(ref url) = index_url {
        cmd_args.push("--index-url");
        cmd_args.push(url);
    }
    if let Some(ref url) = extra_index_url {
        cmd_args.push("--extra-index-url");
        cmd_args.push(url);
    }

    for pkg in &final_args {
        cmd_args.push(pkg);
    }

    let success = if which::which("uv").is_ok() {
        utils::run_in_env(&target_path, "uv", &cmd_args)
    } else {
        utils::run_in_env(&target_path, "pip", &cmd_args[1..])
    };

    // Record packages to session or audit log.
    // BUG FIX: Always scan even on partial failure — some packages
    // may have installed successfully before the batch failed.
    if is_session {
        let t_id = target_id.ok_or("Missing template ID for session")?;
        let installed = utils::get_packages(&target_path);
        let step = db.get_next_step(t_id)?;

        // Capture install_args (e.g., --index-url, --extra-index-url)
        let install_args_str: Option<String> = {
            let mut parts = Vec::new();
            if let Some(ref url) = index_url {
                parts.push(format!("--index-url {}", url));
            }
            if let Some(ref url) = extra_index_url {
                parts.push(format!("--extra-index-url {}", url));
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        };

        let mut recorded = 0usize;
        for pkg_name in packages {
            // Resolve the pip name for matching
            let (base_name, is_wheel, wheel_path) = if pkg_name.starts_with("torch-cu") {
                ("torch".to_string(), false, None)
            } else if pkg_name.ends_with(".whl") || pkg_name.contains(".whl") {
                // Wheel file — extract distribution name from PEP 427 filename
                match utils::normalize_wheel_name(pkg_name) {
                    Some(name) => (name, true, Some(pkg_name.clone())),
                    None => (pkg_name.clone(), false, None),
                }
            } else {
                (pkg_name.clone(), false, None)
            };

            // Match against installed packages (normalize both sides)
            let norm_base = utils::normalize_package_name(&base_name);
            if let Some(pkg) = installed
                .iter()
                .find(|p| utils::normalize_package_name(&p.name) == norm_base)
            {
                let ver = pkg.version.as_deref().unwrap_or("unknown");
                let (itype, iargs) = if is_wheel {
                    ("wheel", wheel_path.as_deref())
                } else if pkg.is_editable {
                    ("edit", install_args_str.as_deref())
                } else {
                    ("pypi", install_args_str.as_deref())
                };
                db.add_template_package(t_id, &pkg.name, ver, true, itype, iargs, step)?;
                recorded += 1;
            }
        }

        if !success && recorded > 0 {
            eprintln!(
                "  {} Some packages failed, but {} successfully-installed package(s) were recorded.",
                "⚠".yellow(),
                recorded
            );
        }
    } else if success {
        let e_id = target_id.ok_or("Missing environment ID")?;
        let installed = utils::get_packages(&target_path);
        for pkg_name in packages {
            let base_name = if pkg_name.starts_with("torch-cu") {
                "torch".to_string()
            } else if pkg_name.ends_with(".whl") || pkg_name.contains(".whl") {
                utils::normalize_wheel_name(pkg_name).unwrap_or_else(|| pkg_name.clone())
            } else {
                pkg_name.clone()
            };
            let norm_base = utils::normalize_package_name(&base_name);
            if let Some(pkg) = installed
                .iter()
                .find(|p| utils::normalize_package_name(&p.name) == norm_base)
            {
                let ver = pkg.version.as_deref().unwrap_or("unknown");
                db.log_package(e_id, &pkg.name, ver, "pypi")?;
            }
        }
    }

    if success {
        println!("Installation complete.");
        let log_env = Path::new(&target_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| target_path.clone());
        activity_log::log_activity(
            "cli",
            "install",
            &format!("{} {}", log_env, packages.join(" ")),
        );
    } else {
        let log_env = Path::new(&target_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| target_path.clone());
        activity_log::log_activity(
            "cli",
            "install:error",
            &format!("{} {}", log_env, packages.join(" ")),
        );
        eprintln!(
            "{} Package installation failed. Check the error message above.",
            "Error:".red()
        );
        std::process::exit(1);
    }
    Ok(())
}
