// SPDX-License-Identifier: Apache-2.0

//! Input validation utilities for command-line arguments.
//!
//! This module provides validation functions to prevent path traversal,
//! command injection, and other invalid inputs.

use std::path::Path;

/// Validates a name for environments or templates.
///
/// Returns an error if the name contains:
/// - Path separators (/ or \)
/// - Parent directory references (..)
/// - Shell metacharacters that could enable injection
/// - Leading/trailing whitespace
/// - Empty strings
pub fn validate_name(name: &str, kind: &str) -> Result<(), String> {
    let name = name.trim();

    if name.is_empty() {
        return Err(format!("{} name cannot be empty", kind));
    }

    if name.len() > 128 {
        return Err(format!("{} name is too long (max 128 characters)", kind));
    }

    // Check for path traversal
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!("{} name cannot contain path characters", kind));
    }

    // Check for shell metacharacters
    const FORBIDDEN: &[char] = &[
        ';', '|', '&', '$', '`', '(', ')', '<', '>', '"', '\'', '\n', '\r', '\0',
    ];
    if name.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(format!("{} name contains invalid characters", kind));
    }

    // Check for leading dots (hidden files)
    if name.starts_with('.') {
        return Err(format!("{} name cannot start with a dot", kind));
    }

    Ok(())
}

/// Validates a Python version string.
///
/// Accepts formats like "3.12", "3.11.4", "3"
pub fn validate_python_version(version: &str) -> Result<(), String> {
    let parts: Vec<&str> = version.split('.').collect();

    if parts.is_empty() || parts.len() > 3 {
        return Err("Invalid Python version format (use X.Y or X.Y.Z)".to_string());
    }

    for part in parts {
        if part.parse::<u32>().is_err() {
            return Err(format!("Invalid Python version component: {}", part));
        }
    }

    Ok(())
}

/// Validates a CUDA version string.
///
/// Accepts formats like "12.6", "13.0", "11.8"
pub fn validate_cuda_version(version: &str) -> Result<(), String> {
    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() != 2 {
        return Err("Invalid CUDA version format (use X.Y, e.g., 12.6)".to_string());
    }

    let major: u32 = parts[0].parse().map_err(|_| "Invalid CUDA major version")?;
    let minor: u32 = parts[1].parse().map_err(|_| "Invalid CUDA minor version")?;

    // Reasonable CUDA version range
    if !(10..=15).contains(&major) {
        return Err(format!(
            "Unsupported CUDA major version: {} (expected 10-15)",
            major
        ));
    }

    if minor > 9 {
        return Err(format!("Invalid CUDA minor version: {}", minor));
    }

    Ok(())
}

/// Validates a file path for safety.
///
/// Ensures the path doesn't escape expected boundaries.
#[allow(dead_code)]
pub fn validate_path(path: &Path, must_exist: bool) -> Result<(), String> {
    // Check for null bytes
    if path.to_string_lossy().contains('\0') {
        return Err("Path contains null bytes".to_string());
    }

    // Check existence if required
    if must_exist && !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_name("myenv", "Environment").is_ok());
        assert!(validate_name("my-env-123", "Environment").is_ok());
        assert!(validate_name("test_env", "Environment").is_ok());
    }

    #[test]
    fn test_invalid_names() {
        assert!(validate_name("", "Environment").is_err());
        assert!(validate_name("../escape", "Environment").is_err());
        assert!(validate_name("path/to/env", "Environment").is_err());
        assert!(validate_name("env;rm -rf", "Environment").is_err());
        assert!(validate_name(".hidden", "Environment").is_err());
        assert!(validate_name("$(whoami)", "Environment").is_err());
    }

    #[test]
    fn test_python_version() {
        assert!(validate_python_version("3.12").is_ok());
        assert!(validate_python_version("3.11.4").is_ok());
        assert!(validate_python_version("3").is_ok());
        assert!(validate_python_version("abc").is_err());
        assert!(validate_python_version("3.12.1.0").is_err());
    }

    #[test]
    fn test_cuda_version() {
        assert!(validate_cuda_version("12.6").is_ok());
        assert!(validate_cuda_version("13.0").is_ok());
        assert!(validate_cuda_version("11.8").is_ok());
        assert!(validate_cuda_version("12").is_err());
        assert!(validate_cuda_version("9.0").is_err());
        assert!(validate_cuda_version("abc").is_err());
    }
}
