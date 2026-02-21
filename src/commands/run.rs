// SPDX-License-Identifier: Apache-2.0

use crate::ops::ZenOps;
use crate::types::EnvName;

use std::error::Error;

pub fn run(ops: &ZenOps, name: &EnvName, command: Vec<String>) -> Result<(), Box<dyn Error>> {
    match ops.run_in_env(name, command) {
        Ok((code, output)) => {
            print!("{}", output);
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
