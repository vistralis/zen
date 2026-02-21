// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::types::EnvName;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;

pub fn run(db: &Database, old: &EnvName, new: &EnvName) -> Result<(), Box<dyn Error>> {
    // Verify old exists
    if db.get_env_id(old)?.is_none() {
        eprintln!("{} Environment '{}' not found.", "Error:".red(), old);
        return Ok(());
    }

    // Verify new doesn't exist
    if db.get_env_id(new)?.is_some() {
        eprintln!("{} Environment '{}' already exists.", "Error:".red(), new);
        return Ok(());
    }

    if db.rename_environment(old, new)? {
        activity_log::log_activity("cli", "rename", &format!("{} -> {}", old, new));
        println!(
            "{} Renamed '{}' → '{}'",
            "✓".green(),
            old.as_str().dimmed(),
            new.to_string().bold()
        );
    } else {
        eprintln!("{} Rename failed.", "Error:".red());
    }
    Ok(())
}
