// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::types;
use crate::utils;

use colored::*;
use std::error::Error;
use std::path::PathBuf;

pub fn run(db: &Database, path: PathBuf, name: Option<String>) -> Result<(), Box<dyn Error>> {
    // Resolve path: accept venv root, bin/python*, or bin/activate
    let fname = path.file_name().unwrap_or_default().to_string_lossy();
    let parent_is_bin = path
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n == "bin");
    let resolved = if parent_is_bin && (fname.starts_with("python") || fname == "activate") {
        // Go up from bin/<file> → venv root
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
            eprintln!(
                "{} Path does not exist: {}",
                "Error:".red(),
                resolved.display()
            );
            return Ok(());
        }
    };

    // Validate it's a venv
    let python_bin = resolved.join("bin/python");
    if !python_bin.exists() {
        eprintln!(
            "{} Not a valid virtual environment (no bin/python): {}",
            "Error:".red(),
            resolved.display()
        );
        return Ok(());
    }

    // Derive name from directory
    let env_name_str = if let Some(n) = name {
        n
    } else {
        let basename = resolved
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // If it's a generic venv name, suggest a better one
        if utils::is_generic_venv_name(&basename) {
            if let Some(suggested) = utils::suggest_env_name(&resolved) {
                use std::io::{self, Write};
                print!(
                    "  Name '{}' is generic. Suggested: {} [enter to accept, or type a name]: ",
                    basename.dimmed(),
                    suggested.cyan().bold()
                );
                io::stdout().flush().ok();
                let mut input = String::new();
                io::stdin().read_line(&mut input).ok();
                let input = input.trim();
                if input.is_empty() {
                    suggested
                } else {
                    input.to_string()
                }
            } else {
                basename
            }
        } else {
            basename
        }
    };
    let env_name = types::EnvName::new(&env_name_str).map_err(|e| e.to_string())?;

    // Check for duplicates
    if db.get_env_id(&env_name)?.is_some() {
        eprintln!(
            "{} Environment '{}' already registered.",
            "Error:".red(),
            env_name
        );
        return Ok(());
    }
    let path_str = resolved.to_string_lossy().to_string();
    if let Ok(Some(existing)) = db.get_env_name_by_path(&path_str) {
        eprintln!(
            "{} Path already registered as '{}'.",
            "Error:".red(),
            existing
        );
        return Ok(());
    }

    // Read python version
    let py_ver = utils::read_python_version(&resolved).unwrap_or_else(|| "unknown".to_string());

    db.register_env(&env_name_str, &path_str, &py_ver)?;
    activity_log::log_activity("cli", "add", &format!("{} -> {}", env_name, path_str));
    println!(
        "{} Registered '{}' (Python {})",
        "✓".green(),
        env_name.bold(),
        py_ver
    );
    println!("  {}", path_str.dimmed());
    Ok(())
}
