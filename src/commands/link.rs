// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;

use colored::*;
use std::error::Error;

/// Formats and prints a single link entry with activation metadata.
fn print_link_entry(
    env_name: &str,
    env_path: &str,
    tag: &Option<String>,
    is_default: bool,
    link_type: &str,
    count: i64,
    last_at: &Option<String>,
) {
    let default_marker = if is_default {
        " [default]".green().to_string()
    } else {
        String::new()
    };
    let tag_str = tag
        .as_ref()
        .map(|t| format!(" ({})", t))
        .unwrap_or_default();
    let type_icon = if link_type == "user" { " ★" } else { "" };
    let stats = if count > 0 {
        let last_str = last_at
            .as_ref()
            .map(|t| format!(", last: {}", &t[..10]))
            .unwrap_or_default();
        format!(" [{}x{}]", count, last_str)
    } else {
        String::new()
    };
    println!(
        "  • {}{}{}{} → {}{}",
        env_name.bold(),
        type_icon,
        tag_str,
        default_marker,
        env_path.dimmed(),
        stats.dimmed()
    );
}

pub fn run_add(db: &Database, name: &str, path: Option<String>) -> Result<(), Box<dyn Error>> {
    let envs = db.list_envs()?;
    let env = envs.iter().find(|(n, ..)| n == name);
    if let Some((_, _path, ..)) = env {
        let project_path = match path {
            Some(p) => std::path::Path::new(&p)
                .canonicalize()
                .map_err(|e| format!("Invalid path '{}': {}", p, e))?
                .to_string_lossy()
                .to_string(),
            None => std::env::current_dir()?
                .canonicalize()?
                .to_string_lossy()
                .to_string(),
        };

        db.associate_project(&project_path, name, None, true)?;
        activity_log::log_activity("cli", "link:add", &format!("{} -> {}", name, project_path));
        println!("Linked '{}' to this project.", name.cyan());
    } else {
        eprintln!(
            "Environment '{}' not found. Run 'zen list' to see available environments.",
            name
        );
    }
    Ok(())
}

pub fn run_rm(db: &Database, name: &str, path: Option<String>) -> Result<(), Box<dyn Error>> {
    let project_path = match path {
        Some(p) => std::path::Path::new(&p)
            .canonicalize()
            .map_err(|e| format!("Invalid path '{}': {}", p, e))?
            .to_string_lossy()
            .to_string(),
        None => std::env::current_dir()?
            .canonicalize()?
            .to_string_lossy()
            .to_string(),
    };

    if let Some(env_id) = db.get_env_id(name)? {
        db.remove_project_association(&project_path, env_id)?;
        activity_log::log_activity("cli", "link:rm", &format!("{} -> {}", name, project_path));
        println!("Unlinked '{}' from this project.", name.yellow());
    } else {
        activity_log::log_activity("cli", "link:rm:error", &format!("{} - not found", name));
        eprintln!("Environment '{}' not found.", name);
    }
    Ok(())
}

pub fn run_list(db: &Database, path: Option<String>) -> Result<(), Box<dyn Error>> {
    let project_path = match path {
        Some(p) => std::path::Path::new(&p)
            .canonicalize()
            .map_err(|e| format!("Invalid path '{}': {}", p, e))?
            .to_string_lossy()
            .to_string(),
        None => std::env::current_dir()?
            .canonicalize()?
            .to_string_lossy()
            .to_string(),
    };

    let links = db.get_project_links_with_stats(&project_path)?;

    if links.is_empty() {
        // Check for inherited (parent path prefix match)
        let all_projects = db.get_all_project_paths()?;
        let inherited: Vec<_> = all_projects
            .iter()
            .filter(|p| project_path.starts_with(*p) && *p != &project_path)
            .collect();

        if !inherited.is_empty() {
            let parent = inherited.iter().max_by_key(|p| p.len()).unwrap();
            let parent_links = db.get_project_links_with_stats(parent)?;
            if !parent_links.is_empty() {
                println!(
                    "{} (inherited from {}):",
                    "Linked environments".cyan(),
                    parent
                );
                for (env_name, env_path, tag, is_default, link_type, count, last_at) in parent_links
                {
                    print_link_entry(
                        &env_name, &env_path, &tag, is_default, &link_type, count, &last_at,
                    );
                }
                return Ok(());
            }
        }
        println!("No environments linked. Use 'zen link add <env>' to link one.");
    } else {
        println!("{}:", "Linked environments".cyan());
        for (env_name, env_path, tag, is_default, link_type, count, last_at) in links {
            print_link_entry(
                &env_name, &env_path, &tag, is_default, &link_type, count, &last_at,
            );
        }
    }
    Ok(())
}

pub fn run_prune(db: &Database) -> Result<(), Box<dyn Error>> {
    let pruned = db.prune_stale_links()?;
    if pruned.is_empty() {
        println!("No stale links found. All links are valid.");
    } else {
        println!("Pruned {} stale link(s):", pruned.len());
        for (project_path, env_name, reason) in &pruned {
            println!(
                "  {} '{}' at {} ({})",
                "✗".red(),
                env_name,
                project_path.dimmed(),
                reason.dimmed()
            );
        }
    }
    Ok(())
}

pub fn run_reset(
    db: &Database,
    path: Option<String>,
    activations: bool,
    _history: bool,
    older_than: Option<u32>,
) -> Result<(), Box<dyn Error>> {
    if let Some(p) = path {
        // Remove ALL links for a specific path
        let resolved = if p == "." {
            std::env::current_dir()?
                .canonicalize()?
                .to_string_lossy()
                .to_string()
        } else {
            std::path::Path::new(&p)
                .canonicalize()
                .unwrap_or_else(|_| std::path::PathBuf::from(&p))
                .to_string_lossy()
                .to_string()
        };
        let count = db.remove_links_for_path(&resolved)?;
        if count == 0 {
            println!("No links found for '{}'", resolved);
        } else {
            activity_log::log_activity(
                "cli",
                "link:reset",
                &format!("path:{} ({})", resolved, count),
            );
            println!(
                "{} Removed {} link(s) for '{}'",
                "✓".green(),
                count,
                resolved
            );
        }
    } else if activations {
        let count = db.remove_activation_links(older_than)?;
        if count == 0 {
            println!("No auto-created activation links to remove.");
        } else {
            println!("{} Removed {} auto-created link(s).", "✓".green(), count);
        }
    } else {
        // history or default: clear activation history
        let count = db.reset_activation_history(older_than)?;
        if count == 0 {
            println!("No activation history to clear.");
        } else {
            println!(
                "{} Cleared activation history for {} link(s).",
                "✓".green(),
                count
            );
        }
    }
    Ok(())
}
