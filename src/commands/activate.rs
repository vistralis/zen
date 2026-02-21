// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;

use colored::*;
use std::error::Error;

pub fn run(
    db: &Database,
    name: Option<String>,
    path_only: bool,
    last: bool,
) -> Result<(), Box<dyn Error>> {
    // zen activate --last: re-activate most recently used env
    if last {
        match db.get_last_activated()? {
            Some((env_name, env_path)) => {
                if !std::path::Path::new(&env_path).exists() {
                    eprintln!(
                        "Last activated env '{}' no longer exists on disk.",
                        env_name
                    );
                    std::process::exit(1);
                }
                if let Ok(cwd) = std::env::current_dir() {
                    let cwd_str = cwd
                        .canonicalize()
                        .unwrap_or(cwd)
                        .to_string_lossy()
                        .to_string();
                    let _ = db.record_activation(&cwd_str, &env_name);
                    activity_log::log_activity("cli", "activate", &env_name);
                }
                if path_only {
                    println!("{}", env_path);
                } else {
                    eprintln!("✓ Last activated: {}", env_name);
                }
            }
            None => {
                eprintln!("No activation history found.");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // zen activate <name>: explicit environment name
    if let Some(ref env_name) = name {
        let envs = db.list_envs()?;
        let env = envs.iter().find(|(n, ..)| n == env_name);

        if let Some((_, path, ..)) = env {
            if let Ok(cwd) = std::env::current_dir() {
                let cwd_str = cwd
                    .canonicalize()
                    .unwrap_or(cwd)
                    .to_string_lossy()
                    .to_string();
                let _ = db.record_activation(&cwd_str, env_name);
                activity_log::log_activity("cli", "activate", env_name);
            }
            if path_only {
                println!("{}", path);
            } else {
                eprintln!(
                    "Shell hook not detected. To enable 'zen activate', add to your shell config:"
                );
                eprintln!("  eval \"$(zen hook zsh)\"   # for zsh");
                eprintln!("  eval \"$(zen hook bash)\"  # for bash");
            }
        } else {
            activity_log::log_activity(
                "cli",
                "activate:error",
                &format!("{} - not found", env_name),
            );
            eprintln!("Environment '{}' not found.", env_name);
            std::process::exit(1);
        }
        return Ok(());
    }

    // zen activate (no args): smart selection from project hierarchy
    let cwd = std::env::current_dir()?
        .canonicalize()?
        .to_string_lossy()
        .to_string();

    // === Bidirectional activation search ===
    let home_dir = std::env::var("HOME").unwrap_or_default();
    let stop_dirs: Vec<&str> = vec!["/", "/tmp", "/home", "/root"];

    // 1. Downward: subfolder links
    let mut all_candidates = db.get_activation_candidates(std::slice::from_ref(&cwd))?;
    let subfolder_candidates = db.get_subfolder_candidates(&cwd, 2)?;
    all_candidates.extend(subfolder_candidates);

    // 2. Upward: exact ancestor match (max 2 levels)
    let mut current = std::path::Path::new(&cwd).to_path_buf();
    let root_path = std::path::Path::new("/");
    let home_path = std::path::Path::new(&home_dir);
    let mut up_depth = 0;
    while let Some(parent) = current.parent() {
        let parent_str = parent.to_string_lossy().to_string();
        if parent_str == home_dir || stop_dirs.contains(&parent_str.as_str()) {
            break;
        }
        if parent.parent() == Some(root_path) || parent.parent() == Some(home_path) {
            break;
        }
        up_depth += 1;
        if up_depth > 2 {
            break;
        }
        let ancestor_candidates = db.get_activation_candidates(&[parent_str])?;
        all_candidates.extend(ancestor_candidates);
        current = parent.to_path_buf();
    }

    // Inject recently created env
    if let Some((recent_name, recent_path)) = db.get_most_recent_env(10)?
        && std::path::Path::new(&recent_path).exists()
    {
        all_candidates.push((
            recent_name,
            recent_path,
            "(recently created)".to_string(),
            0,
            "recent".to_string(),
        ));
    }

    // Deduplicate by env name
    let mut seen = std::collections::HashSet::new();
    let candidates: Vec<_> = all_candidates
        .into_iter()
        .filter(|(env_name, _, _, _, _)| seen.insert(env_name.clone()))
        .collect();

    // Validate on disk
    let valid: Vec<_> = candidates
        .into_iter()
        .filter(|(env_name, env_path, _, _, _)| {
            if std::path::Path::new(env_path).exists() {
                true
            } else {
                eprintln!("⚠ Stale link: '{}' no longer exists on disk", env_name);
                false
            }
        })
        .collect();

    match valid.len() {
        0 => {
            eprintln!("No environments linked to this directory.");
            eprintln!("Use: {} to link one.", "zen link add <env>".cyan());
            std::process::exit(1);
        }
        1 => {
            let (env_name, env_path, project_path, count, link_type) = &valid[0];
            let rel = project_path.clone();
            let _ = db.record_activation(&cwd, env_name);
            if path_only {
                if link_type == "recent" {
                    eprintln!("✓ Activating recently created: {}", env_name.cyan());
                } else {
                    eprintln!(
                        "✓ Auto-selecting: {} ({}{})",
                        env_name.cyan(),
                        rel.dimmed(),
                        if *count >= 10 {
                            " ·frequent".to_string()
                        } else {
                            String::new()
                        }
                    );
                }
                println!("{}", env_path);
            } else {
                eprintln!("✓ Auto-selecting: {} ({})", env_name.cyan(), rel.dimmed());
            }
        }
        _ => {
            // Interactive menu on stderr
            eprintln!("\n{}", "Previously activated environments:".cyan());
            for (i, (env_name, _, project_path, count, link_type)) in valid.iter().enumerate() {
                let rel = project_path.clone();
                let count_str = if *count >= 10 {
                    " ·frequent".to_string()
                } else {
                    String::new()
                };
                let type_marker = match link_type.as_str() {
                    "user" => " ★",
                    "recent" => " 🕐",
                    _ => "",
                };
                eprintln!(
                    "  {}: {}{} ({}{})",
                    (i + 1).to_string().bold(),
                    env_name.bold(),
                    type_marker,
                    rel.dimmed(),
                    count_str
                );
            }
            eprintln!("  {}: Cancel activation", "0".bold());
            eprint!("\nSelect [{}]: ", "1".bold());

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let choice = input.trim();

            let idx: usize = if choice.is_empty() {
                0
            } else if let Ok(n) = choice.parse::<usize>() {
                if n == 0 {
                    eprintln!("Cancelled.");
                    std::process::exit(0);
                }
                n - 1
            } else {
                eprintln!("Invalid selection.");
                std::process::exit(1);
            };

            if idx >= valid.len() {
                eprintln!("Invalid selection.");
                std::process::exit(1);
            }

            let (env_name, env_path, _, _, _) = &valid[idx];
            let _ = db.record_activation(&cwd, env_name);
            if path_only {
                println!("{}", env_path);
            } else {
                eprintln!("Selected: {}", env_name.cyan());
            }
        }
    }
    Ok(())
}
