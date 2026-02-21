// SPDX-License-Identifier: Apache-2.0

use colored::*;
use std::error::Error;
use std::io::Write;

pub fn run(yes: bool) -> Result<(), Box<dyn Error>> {
    if !yes {
        print!(
            "{} This will delete the zen database and config. Environments on disk will NOT be affected.\nContinue? [y/N] ",
            "⚠".yellow()
        );
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let home = home::home_dir().ok_or("Could not find home directory")?;
    let db_path = home.join(".zen").join("zen.db");
    let config_path = home.join(".zen").join("config.toml");

    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
        println!("{} Removed {}", "✓".green(), db_path.display());
    }
    if config_path.exists() {
        std::fs::remove_file(&config_path)?;
        println!("{} Removed {}", "✓".green(), config_path.display());
    }

    println!(
        "\n{} Database reset. Run {} to rediscover environments.",
        "✓".green(),
        "zen scan".cyan()
    );
    Ok(())
}
