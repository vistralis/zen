// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::ops::ZenOps;
use crate::types::EnvName;

use colored::*;
use std::error::Error;
use std::path::Path;

pub fn run(
    ops: &ZenOps,
    db: &Database,
    name: &EnvName,
    yes: bool,
    cached: bool,
    home: &Path,
) -> Result<(), Box<dyn Error>> {
    // Check existence before prompting
    let envs = db.list_envs()?;
    let in_db = envs.iter().any(|(n, ..)| n.as_str() == name.as_str());
    let on_disk = home.join(name.as_str()).exists();
    if !in_db && !on_disk {
        activity_log::log_activity("cli", "rm:error", &format!("{} - not found", name));
        eprintln!("{} Environment '{}' not found.", "Error:".red(), name);
        return Ok(());
    }
    if !yes {
        use dialoguer::{Confirm, theme::ColorfulTheme};
        let prompt_msg = if cached {
            format!(
                "Untrack environment '{}' from registry? (files kept on disk)",
                name
            )
        } else {
            format!("Are you sure you want to remove environment '{}'?", name)
        };
        let confirmed = match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt_msg)
            .default(false)
            .interact()
        {
            Ok(v) => v,
            Err(_) => {
                // Ctrl+C — exit silently
                println!();
                return Ok(());
            }
        };
        if !confirmed {
            println!("Abort.");
            return Ok(());
        }
    }
    if cached {
        // DB-only removal — keep files on disk
        activity_log::log_activity("cli", "rm:cached", name);
        match ops.untrack_env(name) {
            Ok(resp) => {
                println!("{}", resp);
                // Warn if under ZEN_HOME (auto-discovery will re-add it)
                let env_path = envs
                    .iter()
                    .find(|(n, ..)| n.as_str() == name.as_str())
                    .map(|(_, p, ..)| p.clone());
                if let Some(ep) = env_path {
                    let home_str = home.to_string_lossy();
                    if ep.starts_with(home_str.as_ref()) {
                        eprintln!(
                            "{} This env is under Zen Home and will be re-discovered on next list. Use 'zen rm' to delete it from disk.",
                            "⚠ Note:".truecolor(255, 140, 0)
                        );
                    }
                }
            }
            Err(e) => {
                activity_log::log_activity("cli", "rm:error", &format!("{} - {}", name, e));
                eprintln!("{} {}", "Error:".red(), e);
                return Ok(());
            }
        }
    } else {
        println!("{} {}...", "Removing".magenta().bold(), name);
        activity_log::log_activity("cli", "rm", name);
        match ops.remove_env(name) {
            Ok(resp) => println!("{}", resp),
            Err(e) => {
                activity_log::log_activity("cli", "rm:error", &format!("{} - {}", name, e));
                eprintln!("{} {}", "Error:".red(), e);
                return Ok(());
            }
        }
    }
    Ok(())
}
