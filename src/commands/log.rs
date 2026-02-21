// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use colored::*;
use std::error::Error;

pub fn run(filter: Option<String>, lines: usize, clear: bool) -> Result<(), Box<dyn Error>> {
    if clear {
        activity_log::clear_log();
        println!("Log cleared.");
        return Ok(());
    }
    let entries = activity_log::read_log(lines, filter.as_deref());
    if entries.is_empty() {
        println!("No log entries found.");
    } else {
        for entry in &entries {
            println!("{}", entry);
        }
        println!("{}", format!("({} entries)", entries.len()).dimmed());
    }
    Ok(())
}
