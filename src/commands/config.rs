// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use colored::*;
use std::error::Error;

pub fn run(
    db: &Database,
    key: Option<String>,
    value: Option<String>,
) -> Result<(), Box<dyn Error>> {
    match (key, value) {
        (Some(k), Some(v)) => {
            db.set_config(&k, &v)?;
            activity_log::log_activity("cli", "config", &format!("{} = {}", k, v));
            println!("{} Config updated: {} = {}", "✓".green(), k, v);
        }
        (Some(k), None) => {
            let v = db.get_config(&k)?.unwrap_or_else(|| "not set".to_string());
            println!("{} = {}", k, v);
        }
        (None, _) => {
            let configs = db.list_all_config()?;
            if configs.is_empty() {
                println!("No configuration values set.");
            } else {
                println!("{}:", "Configuration".cyan());
                for (k, v) in configs {
                    println!("  {} = {}", k.bold(), v);
                }
            }
        }
    }
    Ok(())
}
