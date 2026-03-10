// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::ops::ZenOps;
use crate::types::EnvName;

use colored::*;
use std::error::Error;

/// Marks an environment as protected — `zen rm` will refuse to remove it.
pub fn protect(ops: &ZenOps, name: &EnvName) -> Result<(), Box<dyn Error>> {
    if ops.protect_env(name, true)? {
        println!(
            "🔒 Environment '{}' is now protected.",
            name.to_string().bold()
        );
        activity_log::log_activity("cli", "protect", name);
    } else {
        eprintln!("{} Environment '{}' not found.", "Error:".red(), name);
    }
    Ok(())
}

/// Removes protection from an environment.
pub fn unprotect(ops: &ZenOps, name: &EnvName) -> Result<(), Box<dyn Error>> {
    if ops.protect_env(name, false)? {
        println!(
            "🔓 Environment '{}' is no longer protected.",
            name.to_string().bold()
        );
        activity_log::log_activity("cli", "unprotect", name);
    } else {
        eprintln!("{} Environment '{}' not found.", "Error:".red(), name);
    }
    Ok(())
}
