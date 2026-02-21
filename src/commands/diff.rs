// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use crate::types::EnvName;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;

/// Runs the `zen diff <env1> <env2>` command.
pub fn run(
    db: &Database,
    env1: &EnvName,
    env2: &EnvName,
    only_diff: bool,
) -> Result<(), Box<dyn Error>> {
    let envs = db.list_envs()?;
    let path1 = envs
        .iter()
        .find(|(n, ..)| n.as_str() == env1.as_str())
        .map(|(_, p, ..)| p.clone());
    let path2 = envs
        .iter()
        .find(|(n, ..)| n.as_str() == env2.as_str())
        .map(|(_, p, ..)| p.clone());

    let (path1, path2) = match (path1, path2) {
        (Some(p1), Some(p2)) => (p1, p2),
        (None, _) => {
            eprintln!("{} Environment '{}' not found", "Error:".red(), env1);
            return Ok(());
        }
        (_, None) => {
            eprintln!("{} Environment '{}' not found", "Error:".red(), env2);
            return Ok(());
        }
    };

    let pkgs1: std::collections::HashMap<_, _> = crate::utils::get_packages(&path1)
        .into_iter()
        .map(|p| (p.name, p.version))
        .collect();
    let pkgs2: std::collections::HashMap<_, _> = crate::utils::get_packages(&path2)
        .into_iter()
        .map(|p| (p.name, p.version))
        .collect();

    let mut all_pkgs: Vec<_> = pkgs1.keys().chain(pkgs2.keys()).collect();
    all_pkgs.sort();
    all_pkgs.dedup();

    println!(
        "{:^30} {:^15} {:^15}",
        "Package".bold(),
        env1.as_str().cyan(),
        env2.as_str().cyan()
    );
    println!("{}", "─".repeat(60));

    for pkg in all_pkgs {
        let v1 = pkgs1.get(pkg).and_then(|v| v.clone());
        let v2 = pkgs2.get(pkg).and_then(|v| v.clone());
        let is_diff = v1 != v2;

        if only_diff && !is_diff {
            continue;
        }

        let v1_str = v1.unwrap_or_else(|| "--".to_string());
        let v2_str = v2.unwrap_or_else(|| "--".to_string());

        if is_diff {
            println!(
                "{:30} {:^15} {:^15}",
                pkg.yellow(),
                v1_str.red(),
                v2_str.green()
            );
        } else {
            println!("{:30} {:^15} {:^15}", pkg, v1_str, v2_str);
        }
    }
    Ok(())
}
