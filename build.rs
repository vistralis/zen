// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

fn main() {
    // Version stamping: tagged releases get clean semver, dev builds get hash suffix
    // Example: v0.6.5 tag → "0.6.5", dev commit → "0.6.5-890abd9"
    let pkg_version = env!("CARGO_PKG_VERSION");

    // Check if HEAD is exactly on a version tag
    let describe = Command::new("git")
        .args(["describe", "--tags", "--exact-match", "HEAD"])
        .output();

    let is_tagged = describe
        .as_ref()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let full_version = if is_tagged {
        // On a tag — clean release version
        pkg_version.to_string()
    } else {
        // Dev build — append short commit hash
        let output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output();
        if let Ok(output) = output {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !hash.is_empty() {
                format!("{}-{}", pkg_version, hash)
            } else {
                pkg_version.to_string()
            }
        } else {
            pkg_version.to_string()
        }
    };

    println!("cargo:rustc-env=ZEN_VERSION={}", full_version);

    // Re-run if git state changes (new commit or tag)
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rerun-if-changed=.git/refs/tags/");
}
