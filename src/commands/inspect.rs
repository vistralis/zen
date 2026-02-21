// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use crate::utils;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;

pub fn run(
    db: &Database,
    env: &str,
    package: Option<String>,
    names_only: bool,
    long: bool,
) -> Result<(), Box<dyn Error>> {
    let envs = db.list_envs()?;
    let env_entry = envs.iter().find(|(n, ..)| n == env);
    if let Some((name, path, ..)) = env_entry {
        let packages = utils::get_packages(path);

        if let Some(package) = package {
            // Single package detail view
            let pkg_lower = package.to_lowercase();
            let found = packages
                .into_iter()
                .find(|p| p.name.to_lowercase() == pkg_lower);

            if let Some(pkg) = found {
                let ver_str = pkg.version.as_deref().unwrap_or("unknown");
                let colored_ver = if ver_str.contains("+cu") {
                    ver_str.green().to_string()
                } else {
                    ver_str.to_string()
                };
                let source_str = pkg.install_source.as_deref().unwrap_or("unknown");
                let colored_source = if source_str == "pypi" {
                    source_str.dimmed().to_string()
                } else {
                    source_str.cyan().to_string()
                };
                println!(
                    "{:12}{}",
                    "Package:".bold(),
                    pkg.name.truecolor(100, 200, 255)
                );
                println!("{:12}{}", "Version:".bold(), colored_ver);
                println!(
                    "{:12}{}",
                    "Installer:".bold(),
                    pkg.installer.as_deref().unwrap_or("unknown").dimmed()
                );
                println!("{:12}{}", "Project:".bold(), colored_source);
                println!(
                    "{:12}{}",
                    "Editable:".bold(),
                    if pkg.is_editable {
                        "yes".truecolor(100, 200, 255).to_string()
                    } else {
                        "no".dimmed().to_string()
                    }
                );
                if let Some(url) = &pkg.source_url {
                    println!("{:12}{}", "URL:".bold(), url.cyan());
                }
                if let Some(commit) = &pkg.commit_id {
                    println!("{:12}{}", "Commit:".bold(), commit.dimmed());
                }
                if let Some(ref import) = pkg.import_name {
                    println!("{:12}{}", "Import:".bold(), import.truecolor(100, 200, 255));
                }
                if let Some(epoch) = pkg.installed_at {
                    use chrono::{Local, TimeZone};
                    if let Some(dt) = Local.timestamp_opt(epoch, 0).single() {
                        println!(
                            "{:12}{}",
                            "Installed:".bold(),
                            dt.format("%Y-%m-%d %H:%M").to_string().dimmed()
                        );
                    }
                }
            } else {
                eprintln!("Package '{}' not found in environment '{}'", package, name);
            }
        } else {
            // List all packages
            let mut sorted = packages;
            sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            if names_only {
                // -1: one name per line
                for pkg in &sorted {
                    println!("{}", pkg.name);
                }
            } else if long {
                // -l: long format, aligned name + version + installer + date
                println!(
                    "{} {} — {} package(s)",
                    "●".truecolor(100, 200, 255),
                    name.truecolor(100, 200, 255).bold(),
                    sorted.len()
                );
                println!();
                let max_name = sorted.iter().map(|p| p.name.len()).max().unwrap_or(20);
                let max_ver = sorted
                    .iter()
                    .map(|p| p.version.as_deref().unwrap_or("?").len())
                    .max()
                    .unwrap_or(10);
                for pkg in &sorted {
                    let ver = pkg.version.as_deref().unwrap_or("?");
                    let colored_ver = if ver.contains("+cu") {
                        ver.green().to_string()
                    } else {
                        ver.dimmed().to_string()
                    };
                    let installer = pkg.installer.as_deref().unwrap_or("?");
                    let editable_mark = if pkg.is_editable { " ✎" } else { "" };
                    let date_str = if let Some(epoch) = pkg.installed_at {
                        use chrono::{Local, TimeZone};
                        if let Some(dt) = Local.timestamp_opt(epoch, 0).single() {
                            dt.format("%Y-%m-%d").to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    println!(
                        "  {:<nw$}  {:<vw$}  {:<3}  {}{}",
                        pkg.name.truecolor(100, 200, 255),
                        colored_ver,
                        installer.dimmed(),
                        date_str.dimmed(),
                        editable_mark,
                        nw = max_name,
                        vw = max_ver
                    );
                }
            } else {
                // Default: ls-style column layout
                println!(
                    "{} {} — {} package(s)",
                    "●".truecolor(100, 200, 255),
                    name.truecolor(100, 200, 255).bold(),
                    sorted.len()
                );
                println!();
                let term_width = terminal_size::terminal_size()
                    .map(|(terminal_size::Width(w), _)| w as usize)
                    .unwrap_or(80);

                // Build display entries: name(version)
                let entries: Vec<(String, String)> = sorted
                    .iter()
                    .map(|pkg| {
                        let ver = pkg.version.as_deref().unwrap_or("?");
                        let plain = format!("{} ({})", pkg.name, ver);
                        let colored = format!(
                            "{} {}{}{}",
                            pkg.name.truecolor(100, 200, 255),
                            "(".dimmed(),
                            if ver.contains("+cu") {
                                ver.green().to_string()
                            } else {
                                ver.dimmed().to_string()
                            },
                            ")".dimmed()
                        );
                        (plain, colored)
                    })
                    .collect();

                let max_width = entries.iter().map(|(p, _)| p.len()).max().unwrap_or(20);
                let col_width = max_width + 2; // 2 char gap
                let num_cols = (term_width / col_width).max(1);
                let num_rows = entries.len().div_ceil(num_cols);

                for row in 0..num_rows {
                    for col in 0..num_cols {
                        let idx = col * num_rows + row; // column-major
                        if idx >= entries.len() {
                            continue;
                        }
                        let (ref plain, ref colored) = entries[idx];
                        if col + 1 < num_cols {
                            let padding = col_width.saturating_sub(plain.len());
                            print!("{}{}", colored, " ".repeat(padding));
                        } else {
                            print!("{}", colored);
                        }
                    }
                    println!();
                }
            }
        }
    } else {
        eprintln!("Environment '{}' not found.", env);
    }
    Ok(())
}
