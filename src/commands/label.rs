// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use colored::*;
use std::error::Error;

/// Runs `zen label add <env> <label>`.
pub fn add(db: &Database, env: &str, label: &str) -> Result<(), Box<dyn Error>> {
    match db.add_label(env, label) {
        Ok(_) => println!("{} Added label '{}' to '{}'", "✓".green(), label, env),
        Err(e) => eprintln!("{} {}", "Error:".red(), e),
    }
    Ok(())
}

/// Runs `zen label rm <env> <label>`.
pub fn rm(db: &Database, env: &str, label: &str) -> Result<(), Box<dyn Error>> {
    match db.remove_label(env, label) {
        Ok(_) => println!("{} Removed label '{}' from '{}'", "✓".green(), label, env),
        Err(e) => eprintln!("{} {}", "Error:".red(), e),
    }
    Ok(())
}

/// Runs `zen label list [--all | <env>]`.
pub fn list(db: &Database, env: Option<&str>, all: bool) -> Result<(), Box<dyn Error>> {
    if all {
        match db.get_all_labels() {
            Ok(entries) => {
                if entries.is_empty() {
                    println!("No labels found.");
                } else {
                    for (env, labels) in entries {
                        println!("{}: {}", env, labels.join(", "));
                    }
                }
            }
            Err(e) => eprintln!("{} {}", "Error:".red(), e),
        }
    } else if let Some(env) = env {
        match db.get_labels(env) {
            Ok(labels) => {
                if labels.is_empty() {
                    println!("No labels for '{}'", env);
                } else {
                    println!("{}: {}", env, labels.join(", "));
                }
            }
            Err(e) => eprintln!("{} {}", "Error:".red(), e),
        }
    }
    Ok(())
}
