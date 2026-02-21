// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use crate::ops::ZenOps;
use crate::utils;
use colored::*;
use owo_colors::OwoColorize;
use std::error::Error;
use std::path::PathBuf;

/// List format options matching the CLI enum.
#[derive(Debug, PartialEq)]
pub enum ListFormat {
    Minimal,
    Compact,
    Wide,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    ops: &ZenOps,
    db: &Database,
    home: &PathBuf,
    pattern: Option<String>,
    sort: &str,
    label: Option<String>,
    format: ListFormat,
    oneline: bool,
    long_format: bool,
) -> Result<(), Box<dyn Error>> {
    // Auto-discover new environments (silent, fast)
    if home.exists()
        && let Ok(entries) = std::fs::read_dir(home)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            let python_bin = path.join("bin/python");
            let python3_bin = path.join("bin/python3");
            if path.is_dir() && (python_bin.exists() || python3_bin.exists()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if db.get_env_id(&name)?.is_none() {
                    let path_str = path.to_string_lossy().to_string();
                    let py_ver = utils::read_python_version(&path_str)
                        .unwrap_or_else(|| "unknown".to_string());
                    db.register_env(&name, &path_str, &py_ver)?;
                }
            }
        }
    }

    // Get envs, optionally filtered by label
    let envs = if let Some(ref label_filter) = label {
        let label_envs = db.get_envs_by_label(label_filter)?;
        ops.list_envs_with_status(pattern.as_deref(), Some(sort), None)?
            .into_iter()
            .filter(|(name, ..)| label_envs.contains(name))
            .collect()
    } else {
        ops.list_envs_with_status(pattern.as_deref(), Some(sort), None)?
    };

    // Handle -1 (oneline) — names only, then exit
    if oneline {
        for (name, ..) in &envs {
            println!("{}", name);
        }
        return Ok(());
    }

    let stack_info_config = db
        .get_config("stack_info")?
        .unwrap_or_else(|| "torch numpy transformers diffusers".to_string());
    let tracked_keys: Vec<&str> = stack_info_config.split_whitespace().collect();

    // Pre-scan all environments for package versions + health
    let env_data: Vec<_> = envs
        .iter()
        .map(|(name, path, py_ver, exists, _updated, is_fav)| {
            let packages = crate::utils::get_packages(path);
            let versions: std::collections::HashMap<String, Option<String>> =
                packages.into_iter().map(|p| (p.name, p.version)).collect();
            // Real health check (native, no subprocess)
            let health = if *exists {
                crate::ops::check_health_quick(std::path::Path::new(path))
            } else {
                crate::types::HealthLevel::Fail
            };
            (
                name.clone(),
                path.clone(),
                py_ver.clone(),
                *exists,
                *is_fav,
                versions,
                health,
            )
        })
        .collect();

    match format {
        ListFormat::Minimal => {
            render_minimal(&env_data, &tracked_keys, long_format);
        }
        ListFormat::Compact => {
            render_compact(&env_data, &tracked_keys);
        }
        ListFormat::Wide => {
            render_wide(&env_data, &tracked_keys);
        }
    }

    // Legend footer with health counts
    render_footer(&env_data);

    Ok(())
}

fn render_minimal(
    env_data: &[(
        String,
        String,
        String,
        bool,
        bool,
        std::collections::HashMap<String, Option<String>>,
        crate::types::HealthLevel,
    )],
    tracked_keys: &[&str],
    long_format: bool,
) {
    // Pre-calculate all column widths
    let max_name = env_data
        .iter()
        .map(|(name, _, _, _, is_fav, _, _)| {
            let icon_w = if *is_fav { 2 } else { 0 };
            name.len() + icon_w
        })
        .max()
        .unwrap_or(12);

    let max_pyver = env_data
        .iter()
        .map(|(_, _, py_ver, _, _, _, _)| py_ver.len())
        .max()
        .unwrap_or(4);

    // Pre-calculate max width per tracked package column
    let tracked_display: Vec<&str> = tracked_keys.iter().take(2).copied().collect();
    let mut max_col_widths: Vec<usize> = tracked_display.iter().map(|k| k.len()).collect();
    for (_, _, _, _, _, versions, _) in env_data {
        for (i, key) in tracked_display.iter().enumerate() {
            if let Some(Some(v)) = versions.get(*key) {
                // key:version — plain width
                let w = key.len() + 1 + v.len();
                if w > max_col_widths[i] {
                    max_col_widths[i] = w;
                }
            }
        }
    }

    for (name, path, py_ver, _exists, is_fav, versions, health) in env_data {
        let name_display = if *is_fav {
            format!("★ {}", name)
        } else {
            format!("  {}", name)
        };
        // Health status — zen aesthetics
        let status_str = match health {
            crate::types::HealthLevel::Pass => {
                format!(" {}", "✓".truecolor(100, 200, 255))
            }
            crate::types::HealthLevel::Info => {
                format!(" {}", "△".truecolor(255, 182, 193))
            }
            crate::types::HealthLevel::Warn => {
                format!(" {}", "!".truecolor(255, 140, 0))
            }
            crate::types::HealthLevel::Fail => {
                format!(" {}", "✗".red())
            }
        };

        // Build stack columns with pre-calculated widths
        let mut stack_str = String::new();
        for (i, key) in tracked_display.iter().enumerate() {
            if let Some(Some(v)) = versions.get(*key) {
                let colored_v = if *key == "torch" && v.contains("+cu") {
                    v.green().to_string()
                } else if *key == "numpy" {
                    if v.starts_with('2') || v.starts_with('3') {
                        v.truecolor(100, 200, 255).to_string()
                    } else {
                        v.truecolor(255, 140, 0).to_string()
                    }
                } else {
                    v.to_string()
                };
                let plain = format!("{}:{}", key, v);
                let colored = format!("{}:{}", key.dimmed(), colored_v);
                let pad = max_col_widths[i].saturating_sub(plain.len());
                stack_str.push_str(&format!("  {}{}", colored, " ".repeat(pad)));
            } else {
                // Blank column, maintain alignment
                stack_str.push_str(&format!("  {}", " ".repeat(max_col_widths[i])));
            }
        }

        let path_str = if long_format {
            format!("  {}", path.dimmed())
        } else {
            String::new()
        };
        println!(
            "{:<name_w$} {:<py_w$}{}{}{}",
            name_display,
            py_ver.dimmed(),
            status_str,
            stack_str,
            path_str,
            name_w = max_name + 2,
            py_w = max_pyver,
        );
    }
}

fn render_compact(
    env_data: &[(
        String,
        String,
        String,
        bool,
        bool,
        std::collections::HashMap<String, Option<String>>,
        crate::types::HealthLevel,
    )],
    tracked_keys: &[&str],
) {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{Cell, Color, ContentArrangement, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let header_style = comfy_table::Attribute::Bold;
    let mut header_row = vec![
        Cell::new("Name").add_attribute(header_style),
        Cell::new("Py").add_attribute(header_style),
        Cell::new("Health").add_attribute(header_style),
    ];

    // Only show first 2 tracked packages in compact mode
    for key in tracked_keys.iter().take(2) {
        header_row.push(
            Cell::new(*key)
                .add_attribute(header_style)
                .set_alignment(comfy_table::CellAlignment::Center),
        );
    }
    table.set_header(header_row);

    for (name, _path, py_ver, _exists, is_fav, versions, health) in env_data {
        let name_display = if *is_fav {
            format!("★ {}", name)
        } else {
            name.clone()
        };

        let health_cell = match health {
            crate::types::HealthLevel::Pass => Cell::new("✓").fg(Color::Rgb {
                r: 100,
                g: 200,
                b: 255,
            }),
            crate::types::HealthLevel::Info => Cell::new("△").fg(Color::Rgb {
                r: 255,
                g: 182,
                b: 193,
            }),
            crate::types::HealthLevel::Warn => Cell::new("!").fg(Color::Red),
            crate::types::HealthLevel::Fail => Cell::new("✗").fg(Color::Red),
        };

        let mut row = vec![
            if *is_fav {
                Cell::new(&name_display).fg(Color::Yellow)
            } else {
                Cell::new(&name_display)
            },
            Cell::new(py_ver),
            health_cell,
        ];

        for key in tracked_keys.iter().take(2) {
            let version = versions.get(*key).and_then(|v| v.clone());
            let cell = match version {
                Some(v) => {
                    if *key == "torch" && v.contains("+cu") {
                        Cell::new(&v).fg(Color::Green)
                    } else if *key == "numpy" && v.starts_with('2') {
                        Cell::new(&v).fg(Color::Cyan)
                    } else {
                        Cell::new(&v)
                    }
                }
                None => Cell::new("--"),
            };
            row.push(cell.set_alignment(comfy_table::CellAlignment::Left));
        }
        table.add_row(row);
    }
    println!("{}", table);
}

fn render_wide(
    env_data: &[(
        String,
        String,
        String,
        bool,
        bool,
        std::collections::HashMap<String, Option<String>>,
        crate::types::HealthLevel,
    )],
    tracked_keys: &[&str],
) {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{Cell, Color, ContentArrangement, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Disabled);

    let header_style = comfy_table::Attribute::Bold;
    let mut header_row = vec![
        Cell::new("Name").add_attribute(header_style),
        Cell::new("Python").add_attribute(header_style),
        Cell::new("Health").add_attribute(header_style),
    ];
    header_row.push(Cell::new("Path").add_attribute(header_style));

    for key in tracked_keys {
        header_row.push(
            Cell::new(*key)
                .add_attribute(header_style)
                .set_alignment(comfy_table::CellAlignment::Center),
        );
    }
    table.set_header(header_row);

    for (name, path, py_ver, _exists, is_fav, versions, health) in env_data {
        let name_display = if *is_fav {
            format!("★ {}", name)
        } else {
            name.clone()
        };

        let health_cell = match health {
            crate::types::HealthLevel::Pass => Cell::new("✓").fg(Color::Rgb {
                r: 100,
                g: 200,
                b: 255,
            }),
            crate::types::HealthLevel::Info => Cell::new("△").fg(Color::Rgb {
                r: 255,
                g: 182,
                b: 193,
            }),
            crate::types::HealthLevel::Warn => Cell::new("!").fg(Color::Red),
            crate::types::HealthLevel::Fail => Cell::new("✗").fg(Color::Red),
        };

        let mut row = vec![
            if *is_fav {
                Cell::new(&name_display).fg(Color::Yellow)
            } else {
                Cell::new(&name_display)
            },
            Cell::new(py_ver),
            health_cell,
        ];
        row.push(Cell::new(path).fg(Color::DarkGrey));

        for key in tracked_keys {
            let version = versions.get(*key).and_then(|v| v.clone());
            let cell = match version {
                Some(v) => {
                    if *key == "torch" && v.contains("+cu") {
                        Cell::new(&v).fg(Color::Green)
                    } else if *key == "numpy" {
                        if v.starts_with('2') {
                            Cell::new(&v).fg(Color::Cyan)
                        } else {
                            Cell::new(&v).fg(Color::Red)
                        }
                    } else {
                        Cell::new(&v)
                    }
                }
                None => Cell::new("--"),
            };
            row.push(cell.set_alignment(comfy_table::CellAlignment::Left));
        }
        table.add_row(row);
    }
    println!("{}", table);
}

fn render_footer(
    env_data: &[(
        String,
        String,
        String,
        bool,
        bool,
        std::collections::HashMap<String, Option<String>>,
        crate::types::HealthLevel,
    )],
) {
    let total = env_data.len();
    let n_fav = env_data
        .iter()
        .filter(|(_, _, _, _, fav, _, _)| *fav)
        .count();
    let n_pass = env_data
        .iter()
        .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Pass)
        .count();
    let n_info = env_data
        .iter()
        .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Info)
        .count();
    let n_warn = env_data
        .iter()
        .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Warn)
        .count();
    let n_fail = env_data
        .iter()
        .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Fail)
        .count();

    print!("{}", format!("{} environments", total).dimmed());
    if n_pass > 0 {
        print!(
            "  {} {}",
            "✓".truecolor(100, 200, 255),
            format!("{} ok", n_pass).dimmed()
        );
    }
    if n_info > 0 {
        print!(
            "  {} {}",
            "△".truecolor(255, 182, 193),
            format!("{} minor", n_info).dimmed()
        );
    }
    if n_warn > 0 {
        print!(
            "  {} {}",
            "!".truecolor(255, 140, 0),
            format!("{} drift", n_warn).dimmed()
        );
    }
    if n_fail > 0 {
        print!("  {} {}", "✗".red(), format!("{} broken", n_fail).dimmed());
    }
    if n_fav > 0 {
        print!(
            "  {} {}",
            "★".truecolor(255, 215, 0),
            format!("{} fav", n_fav).dimmed()
        );
    }
    println!();
}
