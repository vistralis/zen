// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use crate::ops::ZenOps;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;
use std::path::Path;

pub fn run(ops: &ZenOps, db: &Database, home: &Path, db_path: &Path) -> Result<(), Box<dyn Error>> {
    let envs = db.list_envs()?;
    let active = ops.infer_current_env()?;

    println!(
        "\n{}",
        " Zen System Dashboard "
            .bold()
            .on_truecolor(100, 160, 160)
            .white()
    );
    println!("{}", "----------------------".truecolor(100, 160, 160));

    if let Some(ref name) = active {
        println!("  {: <20} {}", "Active Env:".bold(), name.green().bold());
        // Show path
        if let Some((_, path, ..)) = envs.iter().find(|(n, ..)| n == name) {
            println!("  {: <20} {}", "Active Path:".bold(), path.dimmed());
        }
    } else {
        println!("  {: <20} {}", "Active Env:".bold(), "none".dimmed());
    }

    println!(
        "  {: <20} {}",
        "Managed Envs:".bold(),
        envs.len().to_string().truecolor(100, 160, 160)
    );
    let zen_home_default = std::env::var("ZEN_HOME").is_err();
    println!(
        "  {: <20} {}{}",
        "Zen Home:".bold(),
        home.display().to_string().dimmed(),
        if zen_home_default {
            " (default)".dimmed().to_string()
        } else {
            String::new()
        }
    );

    let zen_dojo_default = std::env::var("ZEN_DOJO").is_err();
    println!(
        "  {: <20} {}{}",
        "Zen Dojo:".bold(),
        db_path.display().to_string().dimmed(),
        if zen_dojo_default {
            " (default)".dimmed().to_string()
        } else {
            String::new()
        }
    );
    println!();
    Ok(())
}
