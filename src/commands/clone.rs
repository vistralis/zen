// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::types::EnvName;
use crate::utils;

use colored::*;
use std::error::Error;
use std::path::Path;

pub fn run(
    db: &Database,
    source: &EnvName,
    name: &EnvName,
    home: &Path,
) -> Result<(), Box<dyn Error>> {
    let envs = db.list_envs()?;
    let found = envs.iter().find(|(n, ..)| n.as_str() == source.as_str());
    let (_, source_path, source_py, ..) = match found {
        Some(e) => e,
        None => {
            eprintln!(
                "{} Source environment '{}' not found.",
                "Error:".red(),
                source
            );
            return Ok(());
        }
    };

    // Check if target already exists
    if envs.iter().any(|(n, ..)| n.as_str() == name.as_str()) {
        activity_log::log_activity(
            "cli",
            "clone:error",
            &format!("{} -> {} - target exists", source, name),
        );
        eprintln!("{} Environment '{}' already exists.", "Error:".red(), name);
        return Ok(());
    }

    println!("Cloning '{}' → '{}'...", source, name);

    // Create target path using configured home
    let target_path = home.join(name.as_str());

    // Copy the entire directory
    let copy_result = std::process::Command::new("cp")
        .args(["-r", source_path, target_path.to_str().unwrap()])
        .status();

    if copy_result.is_err() || !copy_result.unwrap().success() {
        activity_log::log_activity(
            "cli",
            "clone:error",
            &format!("{} -> {} - copy failed", source, name),
        );
        eprintln!("{} Failed to copy environment directory.", "Error:".red());
        return Ok(());
    }

    // Register the new environment
    let new_id = db.register_env(name, target_path.to_str().unwrap(), source_py)?;

    // Copy package metadata from filesystem
    let packages = utils::get_packages(target_path.to_str().unwrap());
    for pkg in packages {
        let ver = pkg.version.as_deref().unwrap_or("unknown");
        db.log_package(new_id, &pkg.name, ver, "pypi")?;
    }

    activity_log::log_activity("cli", "clone", &format!("{} -> {}", source, name));
    println!("✓ Environment '{}' cloned successfully!", name);
    println!("  Project: {} ({})", source, source_path);
    println!("  Target: {} ({})", name, target_path.display());
    Ok(())
}
