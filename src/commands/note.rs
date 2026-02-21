// SPDX-License-Identifier: Apache-2.0

use crate::ops::ZenOps;
use crate::types::EnvName;
use colored::*;
use std::error::Error;

/// Runs `zen note add <env> <message>`.
pub fn add(ops: &ZenOps, env_name: &EnvName, message: &str) -> Result<(), Box<dyn Error>> {
    match ops.log_comment(Some(env_name), message) {
        Ok(resp) => println!("{}", resp),
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(())
}

/// Runs `zen note list [--all | <env>]`.
pub fn list(ops: &ZenOps, env_filter: Option<&EnvName>) -> Result<(), Box<dyn Error>> {
    let show_env_col = env_filter.is_none();
    match ops.list_comments(None, env_filter) {
        Ok(comments) => {
            if comments.is_empty() {
                if show_env_col {
                    println!("No notes found.");
                } else {
                    println!("No notes for '{}'", env_filter.unwrap());
                }
            } else {
                use comfy_table::{Cell, Color};
                let mut table = crate::table::new_table();
                if show_env_col {
                    table.set_header(vec!["UUID", "Env", "Note", "Timestamp"]);
                } else {
                    table.set_header(vec!["UUID", "Note", "Timestamp"]);
                }
                for (uuid, _pp, env_name, msg, _tag, ts) in comments {
                    let short_uuid = if uuid.len() > 8 {
                        format!("{}…", &uuid[..8])
                    } else {
                        uuid.clone()
                    };
                    if show_env_col {
                        table.add_row(vec![
                            Cell::new(short_uuid).fg(Color::DarkGrey),
                            Cell::new(env_name.unwrap_or_else(|| "-".into())).fg(Color::Cyan),
                            Cell::new(msg),
                            Cell::new(ts).fg(Color::DarkGrey),
                        ]);
                    } else {
                        table.add_row(vec![
                            Cell::new(short_uuid).fg(Color::DarkGrey),
                            Cell::new(msg),
                            Cell::new(ts).fg(Color::DarkGrey),
                        ]);
                    }
                }
                println!("{}", table);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(())
}

/// Runs `zen note rm <uuid>`.
pub fn rm(ops: &ZenOps, uuid: &str) -> Result<(), Box<dyn Error>> {
    match ops.remove_comment(uuid) {
        Ok(0) => eprintln!("{} No note found matching '{}'", "✗".red(), uuid),
        Ok(1) => println!("{} Note {} removed.", "✓".green(), uuid),
        Ok(n) => println!("{} {} notes removed (prefix '{}')", "⚠".yellow(), n, uuid),
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(())
}
