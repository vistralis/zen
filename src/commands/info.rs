// SPDX-License-Identifier: Apache-2.0

use crate::ops::ZenOps;
use crate::output::EnvDetails;
use crate::types::HealthLevel;
use crate::utils;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;

/// Runs the `zen info <name>` command.
pub fn run(ops: &ZenOps, name: &str, json: bool) -> Result<(), Box<dyn Error>> {
    let envs = ops.list_envs_with_status(None, None, None)?;
    let env = envs.iter().find(|(n, ..)| n == name);

    if let Some((_, path, _, exists, ..)) = env {
        if !exists {
            if json {
                let details = EnvDetails {
                    name: name.to_string(),
                    python: "missing".to_string(),
                    path: path.clone(),
                    packages: 0,
                    created: None,
                    torch: None,
                    cuda: None,
                    numpy: None,
                };
                println!("{}", serde_json::to_string_pretty(&details)?);
            } else {
                println!(
                    "Environment: {} (MISSING on filesystem)",
                    name.magenta().bold()
                );
            }
        } else {
            let py_ver = utils::read_python_version(path).unwrap_or_else(|| "unknown".to_string());
            let (torch_ver, cuda_ver) = utils::read_torch_version(path)
                .map(|(t, c)| (Some(t), c))
                .unwrap_or((None, None));
            let packages = utils::get_packages(path);
            let get_ver = |pkg_name: &str| {
                packages
                    .iter()
                    .find(|p| p.name == pkg_name)
                    .and_then(|p| p.version.clone())
            };

            if json {
                let details = EnvDetails {
                    name: name.to_string(),
                    python: py_ver,
                    path: path.clone(),
                    packages: packages.len(),
                    created: None,
                    torch: torch_ver,
                    cuda: cuda_ver,
                    numpy: get_ver("numpy"),
                };
                println!("{}", serde_json::to_string_pretty(&details)?);
                return Ok(());
            }

            println!(
                "{}  {}",
                "Environment:".bold(),
                name.truecolor(100, 200, 255)
            );
            println!("{}       {}", "Path:".bold(), path.dimmed());
            println!("{}     {}", "Python:".bold(), py_ver);

            // NumPy with version coloring
            if let Some(np_ver) = get_ver("numpy") {
                let colored = if np_ver.starts_with('2') || np_ver.starts_with('3') {
                    np_ver.truecolor(100, 200, 255).to_string()
                } else {
                    np_ver.truecolor(255, 140, 0).to_string()
                };
                println!("{}      {}", "NumPy:".bold(), colored);
            }

            // Torch with +cu coloring
            if let Some(ref tv) = torch_ver {
                let colored = if tv.contains("+cu") {
                    tv.green().to_string()
                } else {
                    tv.to_string()
                };
                println!("{}      {}", "Torch:".bold(), colored);
            }
            if let Some(ref cv) = cuda_ver {
                println!("{}       {}", "CUDA:".bold(), cv);
            }

            // Package count
            println!(
                "{}   {}",
                "Packages:".bold(),
                packages.len().to_string().dimmed()
            );

            // Quick health
            let health = crate::ops::check_health_quick(std::path::Path::new(path));
            let health_str = match health {
                HealthLevel::Pass => {
                    format!("{} {}", "✓".truecolor(100, 200, 255), "ok".dimmed())
                }
                HealthLevel::Info => {
                    format!("{} {}", "△".truecolor(255, 182, 193), "minor".dimmed())
                }
                HealthLevel::Warn => {
                    format!("{} {}", "!".truecolor(255, 140, 0), "drift".dimmed())
                }
                HealthLevel::Fail => {
                    format!("{} {}", "✗".red(), "broken".dimmed())
                }
            };
            println!("{}     {}", "Health:".bold(), health_str);

            // Editable source packages
            let source: Vec<_> = packages
                .iter()
                .filter(|p| p.is_editable)
                .map(|p| p.name.clone())
                .collect();
            if !source.is_empty() {
                println!(
                    "{}     {}",
                    "Project:".bold(),
                    source.join(", ").truecolor(100, 200, 255)
                );
            }
        }
    } else {
        eprintln!("Environment '{}' not found.", name);
    }
    Ok(())
}
