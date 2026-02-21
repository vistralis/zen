// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;

/// Runs the `zen find <package> [--exact]` command.
pub fn run(db: &Database, package: &str, exact: bool) -> Result<(), Box<dyn Error>> {
    // Split query into name and optional version at "=="
    let (pkg_query, version_query) = if package.contains("==") {
        let parts: Vec<&str> = package.split("==").collect();
        (
            parts[0].to_string(),
            Some(parts.get(1).unwrap_or(&"").to_string()),
        )
    } else {
        (package.to_string(), None)
    };

    let pattern = pkg_query.replace('*', "");
    // pip treats hyphens and underscores as equivalent
    let normalize = |s: &str| s.to_lowercase().replace('-', "_");

    let envs = db.list_envs()?;
    let mut found = Vec::new();

    for (name, path, ..) in &envs {
        let packages = crate::utils::get_packages(path);
        for pkg in packages {
            let pkg_norm = normalize(&pkg.name);
            let pattern_norm = normalize(&pattern);

            let name_match = if exact {
                pkg_norm == pattern_norm
            } else {
                pkg_norm.contains(&pattern_norm)
            };

            let version_match = match (&version_query, &pkg.version) {
                (Some(q), Some(v)) => {
                    if q.contains('+') {
                        v == q
                    } else {
                        let base_ver = v.split('+').next().unwrap_or(v);
                        base_ver.starts_with(q.as_str())
                    }
                }
                (Some(_), None) => false,
                (None, _) => true,
            };

            if name_match && version_match {
                found.push((name.clone(), pkg.name.clone(), pkg.version.clone()));
            }
        }
    }

    if found.is_empty() {
        println!("No environments contain package matching '{}'", package);
    } else {
        println!("{}", "Package matches:".bold());
        for (env, pkg_name, version) in found {
            let ver = version.unwrap_or_else(|| "?".to_string());
            println!(
                "  {} {} {} {}",
                env.cyan(),
                pkg_name,
                "→".dimmed(),
                ver.green()
            );
        }
    }
    Ok(())
}
