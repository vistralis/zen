// SPDX-License-Identifier: Apache-2.0

use crate::ops::{InstallOptions, ZenOps};
use crate::types::{Diagnostic, EnvName, HealthLevel};
use crate::utils;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;

/// Runs the `zen health <name>` command.
pub fn run(ops: &ZenOps, env_name: &EnvName, fix: bool) -> Result<(), Box<dyn Error>> {
    // Look up path for display
    let env_path_display = ops
        .list_envs_with_status(None, None, None)
        .ok()
        .and_then(|envs| {
            envs.iter()
                .find(|(n, ..)| n == env_name.as_str())
                .map(|(_, p, ..)| p.clone())
        });

    match ops.check_health(env_name) {
        Ok(report) => {
            println!(
                "{}  {}",
                "Environment:".bold(),
                env_name.as_str().truecolor(100, 200, 255)
            );
            if let Some(ref p) = env_path_display {
                println!("{}       {}", "Path:".bold(), p.dimmed());
            }
            let label = " Health ";
            let total_w: usize = 50;
            let pad = total_w.saturating_sub(label.len()) / 2;
            println!(
                "{}{}{}",
                "─".repeat(pad),
                label.dimmed(),
                "─".repeat(total_w - pad - label.len())
            );
            for item in &report.items {
                let (icon, color_msg) = match item.level() {
                    HealthLevel::Pass => (
                        "✓".truecolor(100, 200, 255).to_string(),
                        item.message().normal().to_string(),
                    ),
                    HealthLevel::Info => (
                        "△".truecolor(255, 182, 193).to_string(),
                        item.message().truecolor(255, 182, 193).to_string(),
                    ),
                    HealthLevel::Warn => (
                        "⚠".truecolor(255, 140, 0).to_string(),
                        item.message().truecolor(255, 140, 0).to_string(),
                    ),
                    HealthLevel::Fail => ("✗".red().to_string(), item.message().red().to_string()),
                };
                println!("{} {}", icon, color_msg);
            }
            println!();
            let status = match report.overall() {
                HealthLevel::Pass => "OK".truecolor(100, 200, 255).bold().to_string(),
                HealthLevel::Info => "MINOR".truecolor(255, 182, 193).bold().to_string(),
                HealthLevel::Warn => "DRIFT".truecolor(255, 140, 0).bold().to_string(),
                HealthLevel::Fail => "BROKEN".red().bold().to_string(),
            };
            println!("Overall: {}", status);

            // --fix: auto-install missing dependencies
            if fix {
                println!();
                let envs = ops.list_envs_with_status(None, None, None)?;
                let env_path = envs
                    .iter()
                    .find(|(n, ..)| n == env_name.as_str())
                    .map(|(_, p, ..)| p.clone())
                    .ok_or_else(|| format!("Environment '{}' not found", env_name))?;
                let dep_issues = utils::check_dependencies(&env_path);
                let mut missing: Vec<String> = dep_issues
                    .iter()
                    .filter_map(|issue| match issue {
                        utils::DepIssue::Missing { requires, .. } => {
                            // Extract bare package name from requirement specifier
                            // e.g., "numpy>=1.0" → "numpy", "torch[cuda]>=2.0" → "torch"
                            let name = requires
                                .split(&['>', '<', '=', '!', '~', '[', ';'][..])
                                .next()
                                .unwrap_or(requires)
                                .trim();
                            if !name.is_empty() {
                                Some(name.to_string())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    })
                    .collect();
                missing.sort();
                missing.dedup();

                if missing.is_empty() {
                    println!("{} No fixable issues found.", "✓".truecolor(100, 200, 255));
                } else {
                    println!(
                        "{} Installing {} missing dep{}...",
                        "⟳".truecolor(100, 200, 255),
                        missing.len(),
                        if missing.len() == 1 { "" } else { "s" }
                    );
                    for pkg in &missing {
                        print!("  → {} ... ", pkg);
                        match ops.install_packages(
                            env_name,
                            vec![pkg.clone()],
                            InstallOptions::default(),
                        ) {
                            Ok(_) => println!("{}", "ok".truecolor(100, 200, 255)),
                            Err(e) => println!("{} {}", "failed".red(), e),
                        }
                    }
                    println!();
                    println!(
                        "{} Fix complete. Run {} again to verify.",
                        "✓".truecolor(100, 200, 255),
                        "zen health".bold()
                    );
                }
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".red(), e),
    }
    Ok(())
}
