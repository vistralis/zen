// SPDX-License-Identifier: Apache-2.0

use crate::activity_log;
use crate::db::Database;
use crate::ops::ZenOps;
use crate::types::EnvName;
use colored::*;
use std::error::Error;

pub fn run(
    ops: &ZenOps,
    _db: &Database,
    env_name: &EnvName,
    packages: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    match ops.uninstall_packages(env_name, packages.clone()) {
        Ok(msg) => {
            println!("{}", msg);
            activity_log::log_activity(
                "cli",
                "uninstall",
                &format!("{} {}", env_name.as_str(), packages.join(" ")),
            );
        }
        Err(e) => {
            activity_log::log_activity(
                "cli",
                "uninstall:error",
                &format!("{} {} - {}", env_name.as_str(), packages.join(" "), e),
            );
            eprintln!("{} {}", "Error:".red(), e);
        }
    }
    Ok(())
}
