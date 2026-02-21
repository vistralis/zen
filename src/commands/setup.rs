// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use crate::ops::ZenOps;

use colored::*;
use std::error::Error;
use std::path::PathBuf;

pub fn run_init(ops: &ZenOps, path: PathBuf, yes: bool) -> Result<(), Box<dyn Error>> {
    println!(
        "Zen Setup Wizard: Scanning {} for environments...",
        path.display()
    );
    let found = crate::utils::discover_venvs(&path);

    if found.is_empty() {
        println!("No virtual environments found in this directory.");
    } else {
        let confirm = if yes {
            true
        } else {
            println!("\nFound {} environments in this directory.", found.len());
            match dialoguer::Confirm::new()
                .with_prompt("Do you want to import them into Zen now?")
                .default(true)
                .interact()
            {
                Ok(v) => v,
                Err(_) => {
                    println!();
                    return Ok(());
                }
            }
        };

        if confirm {
            println!("Importing... (this will scan packages for each env)");
            match ops.bulk_import(found) {
                Ok(msg) => println!("\n✓ {}", msg),
                Err(e) => eprintln!("\nError: {}", e),
            }
        } else {
            println!("Import cancelled.");
        }
    }
    Ok(())
}

pub fn run_stack_info(db: &Database) -> Result<(), Box<dyn Error>> {
    use dialoguer::{Input, theme::ColorfulTheme};
    let config = db
        .get_config("stack_info")?
        .unwrap_or_else(|| "torch numpy transformers diffusers".to_string());
    let new_config: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter packages to track (space separated)")
        .default(config)
        .interact_text()?;
    db.set_config("stack_info", &new_config)?;
    println!("{} Stack info packages updated.", "✓".green());
    Ok(())
}
