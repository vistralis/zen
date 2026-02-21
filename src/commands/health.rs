// SPDX-License-Identifier: Apache-2.0

use crate::ops::ZenOps;
use crate::types::{Diagnostic, EnvName, HealthLevel};
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;

/// Runs the `zen health <name>` command.
pub fn run(ops: &ZenOps, env_name: &EnvName) -> Result<(), Box<dyn Error>> {
    match ops.check_health(env_name) {
        Ok(report) => {
            println!(
                "{}  {}",
                "Environment:".bold(),
                env_name.as_str().truecolor(100, 200, 255)
            );
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
        }
        Err(e) => eprintln!("{} {}", "Error:".red(), e),
    }
    Ok(())
}
