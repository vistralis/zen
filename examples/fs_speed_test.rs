// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Reads package versions directly from .dist-info/METADATA files.
/// This is MUCH faster than spawning pip/python processes.
fn get_packages_from_fs(env_path: &Path) -> HashMap<String, String> {
    let mut result = HashMap::new();

    // Find site-packages directory
    let lib_path = env_path.join("lib");
    if !lib_path.exists() {
        return result;
    }

    // Find python version directory (e.g., python3.12)
    let python_dir = fs::read_dir(&lib_path)
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .find(|e| e.file_name().to_string_lossy().starts_with("python"))
        })
        .map(|e| e.path());

    let site_packages = match python_dir {
        Some(p) => p.join("site-packages"),
        None => return result,
    };

    if !site_packages.exists() {
        return result;
    }

    // Iterate over *.dist-info directories
    if let Ok(entries) = fs::read_dir(&site_packages) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".dist-info") {
                // Parse package name and version from directory name
                // Format: <name>-<version>.dist-info
                let _without_suffix = name.trim_end_matches(".dist-info");

                // Read METADATA file for accurate version
                let metadata_path = entry.path().join("METADATA");
                if let Ok(content) = fs::read_to_string(&metadata_path) {
                    let mut pkg_name = None;
                    let mut pkg_version = None;

                    for line in content.lines() {
                        if let Some(name) = line.strip_prefix("Name: ") {
                            pkg_name = Some(name.to_string());
                        } else if let Some(ver) = line.strip_prefix("Version: ") {
                            pkg_version = Some(ver.to_string());
                        }
                        // Stop after we have both (they're always at the top)
                        if pkg_name.is_some() && pkg_version.is_some() {
                            break;
                        }
                    }

                    if let (Some(name), Some(version)) = (pkg_name, pkg_version) {
                        result.insert(name, version);
                    }
                }
            }
        }
    }

    // Special handling for torch: read version.py for CUDA suffix
    let torch_version_file = site_packages.join("torch").join("version.py");
    if torch_version_file.exists()
        && let Ok(content) = fs::read_to_string(&torch_version_file)
    {
        // Look for: __version__ = '2.9.1+cu128'
        for line in content.lines() {
            if line.starts_with("__version__") {
                // Extract version from quotes
                if let Some(start) = line.find('\'').or_else(|| line.find('"')) {
                    let rest = &line[start + 1..];
                    if let Some(end) = rest.find('\'').or_else(|| rest.find('"')) {
                        let version = &rest[..end];
                        result.insert("torch".to_string(), version.to_string());
                    }
                }
                break;
            }
        }
    }

    result
}

fn main() {
    // Use ZEN_HOME env var, or default to XDG standard
    let envs_dir_str = std::env::var("ZEN_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.local/share/zen/envs", home)
    });
    let envs_dir = Path::new(&envs_dir_str);

    // Test on a few environments (adjust names to your setup)
    let test_envs = ["myenv", "dev", "ml"];

    for env_name in &test_envs {
        let env_path = envs_dir.join(env_name);
        if !env_path.exists() {
            println!("Skipping {} (not found)", env_name);
            continue;
        }

        println!("\n=== {} ===", env_name);

        // Time the filesystem approach
        let start = std::time::Instant::now();
        let fs_packages = get_packages_from_fs(&env_path);
        let fs_duration = start.elapsed();

        println!(
            "Filesystem: {} packages in {:?}",
            fs_packages.len(),
            fs_duration
        );

        // Show key packages
        let key_packages = [
            "torch",
            "numpy",
            "transformers",
            "diffusers",
            "pip",
            "setuptools",
        ];
        for pkg in &key_packages {
            if let Some(version) = fs_packages.get(*pkg) {
                println!("  {}: {}", pkg, version);
            }
        }

        // Compare with pip list (time it)
        let start = std::time::Instant::now();
        let pip_output = std::process::Command::new(env_path.join("bin/pip"))
            .args(["list", "--format=json"])
            .output();
        let pip_duration = start.elapsed();

        if let Ok(output) = pip_output
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let pip_packages: Vec<serde_json::Value> =
                serde_json::from_str(&stdout).unwrap_or_default();
            println!("Pip: {} packages in {:?}", pip_packages.len(), pip_duration);
            println!(
                "Speedup: {:.1}x faster",
                pip_duration.as_secs_f64() / fs_duration.as_secs_f64()
            );
        }
    }
}
