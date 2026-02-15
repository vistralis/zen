// SPDX-License-Identifier: Apache-2.0

//! Core types for Zen — "Parse, Don't Validate" philosophy.
//!
//! These types enforce invariants at construction time so that downstream code
//! never needs to re-validate. Inspired by uv's `PackageName` newtype pattern.

use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

// =============================================================================
// EnvName — validated environment name
// =============================================================================

/// A validated environment name.
///
/// Guarantees:
/// - Non-empty, trimmed
/// - No path separators (`/`, `\`), no `..`
/// - No shell metacharacters (`;|&$` etc.)
/// - Doesn't start with `.`
/// - Max 128 characters
///
/// Once constructed, you can use it anywhere a `&str` is expected via `Deref`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EnvName(String);

/// Errors that can occur when parsing an environment name.
#[derive(Debug, Clone)]
pub struct EnvNameError {
    input: String,
    reason: &'static str,
}

impl fmt::Display for EnvNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid environment name '{}': {}",
            self.input, self.reason
        )
    }
}

impl std::error::Error for EnvNameError {}

impl EnvName {
    /// Create a new `EnvName` from a string, validating all invariants.
    pub fn new(name: impl Into<String>) -> Result<Self, EnvNameError> {
        let raw = name.into();
        let trimmed = raw.trim().to_string();

        if trimmed.is_empty() {
            return Err(EnvNameError {
                input: raw,
                reason: "cannot be empty",
            });
        }

        if trimmed.len() > 128 {
            return Err(EnvNameError {
                input: trimmed,
                reason: "too long (max 128 characters)",
            });
        }

        if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
            return Err(EnvNameError {
                input: trimmed,
                reason: "cannot contain path characters",
            });
        }

        const FORBIDDEN: &[char] = &[
            ';', '|', '&', '$', '`', '(', ')', '<', '>', '"', '\'', '\n', '\r', '\0',
        ];
        if trimmed.chars().any(|c| FORBIDDEN.contains(&c)) {
            return Err(EnvNameError {
                input: trimmed,
                reason: "contains shell metacharacters",
            });
        }

        if trimmed.starts_with('.') {
            return Err(EnvNameError {
                input: trimmed,
                reason: "cannot start with a dot",
            });
        }

        Ok(Self(trimmed))
    }

    /// Returns the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `EnvName` and returns the inner `String`.
    #[allow(dead_code)]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for EnvName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for EnvName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EnvName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for EnvName {
    type Err = EnvNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

// serde: deserialize with validation
impl<'de> serde::Deserialize<'de> for EnvName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        EnvName::new(s).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for EnvName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

// schemars v0.8: used by standalone `schemars = "0.8"` dep
impl schemars::JsonSchema for EnvName {
    fn schema_name() -> String {
        "EnvName".to_string()
    }
    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        // Reuse String's schema — the validation happens at parse time
        String::json_schema(generator)
    }
}

// schemars v1.x: used by rmcp's re-exported schemars
impl rmcp::schemars::JsonSchema for EnvName {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("EnvName")
    }
    fn json_schema(generator: &mut rmcp::schemars::SchemaGenerator) -> rmcp::schemars::Schema {
        String::json_schema(generator)
    }
}

// =============================================================================
// Diagnostic Trait — decoupled health reporting
// =============================================================================

/// Minimal diagnostic interface (inspired by uv's Diagnostic trait).
///
/// Each diagnostic carries structured data. The `message()` and `level()` methods
/// provide the rendering and severity, completely decoupled from the check logic.
pub trait Diagnostic {
    /// Convert the diagnostic into a user-facing message.
    fn message(&self) -> String;

    /// The severity level of this diagnostic.
    fn level(&self) -> HealthLevel;
}

/// Severity level for health check results.
///
/// Ordered by severity: Pass < Info < Warn < Fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum HealthLevel {
    /// Environment is fully healthy.
    #[default]
    Pass,
    /// Minor issues (e.g. missing optional dependencies).
    Info,
    /// Actionable issues (e.g. version conflicts, CUDA mismatch).
    Warn,
    /// Critical failure (e.g. missing python binary, broken symlink).
    Fail,
}

impl fmt::Display for HealthLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => write!(f, "OK"),
            Self::Info => write!(f, "MINOR"),
            Self::Warn => write!(f, "DRIFT"),
            Self::Fail => write!(f, "BROKEN"),
        }
    }
}

impl HealthLevel {
    /// The icon for this severity level.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pass => "✓",
            Self::Info => "~",
            Self::Warn => "⚠",
            Self::Fail => "✗",
        }
    }
}

// =============================================================================
// HealthDiagnostic — typed health check results
// =============================================================================

/// Typed health diagnostic — each variant carries exactly the data it needs.
///
/// Replaces the old string-based `HealthItem` with structured data, enabling
/// programmatic inspection, filtering, and display without string parsing.
pub enum HealthDiagnostic {
    /// Python binary exists and is functional.
    PythonOk { version: String },
    /// Python binary is missing or broken.
    PythonMissing,
    /// Python binary is a broken symlink.
    BrokenSymlink { target: PathBuf },
    /// site-packages directory exists.
    SitePackagesOk,
    /// site-packages directory is missing.
    SitePackagesMissing,
    /// All CUDA packages use the same backend.
    CudaConsistent { suffix: String },
    /// Mixed CUDA versions detected.
    CudaMismatch { details: String },
    /// CPU and CUDA packages mixed.
    CpuCudaConflict { details: String },
    /// All dependency constraints satisfied.
    DependenciesOk,
    /// Missing dependencies (info-level).
    MissingDependencies { count: usize, details: String },
    /// Version conflicts (warn-level).
    VersionConflicts { count: usize, details: String },
}

impl Diagnostic for HealthDiagnostic {
    fn message(&self) -> String {
        match self {
            Self::PythonOk { version } => format!("Python {} OK", version),
            Self::PythonMissing => "Python binary missing or not found".to_string(),
            Self::BrokenSymlink { target } => {
                format!("Python symlink broken → {}", target.display())
            }
            Self::SitePackagesOk => "site-packages OK".to_string(),
            Self::SitePackagesMissing => "site-packages directory missing".to_string(),
            Self::CudaConsistent { suffix } => {
                format!("CUDA consistency OK (all packages use +{})", suffix)
            }
            Self::CudaMismatch { details } => details.clone(),
            Self::CpuCudaConflict { details } => details.clone(),
            Self::DependenciesOk => "Dependencies OK (all Requires-Dist satisfied)".to_string(),
            Self::MissingDependencies { count, details } => {
                format!(
                    "{} missing dep{}:\n{}",
                    count,
                    if *count == 1 { "" } else { "s" },
                    details
                )
            }
            Self::VersionConflicts { count, details } => {
                format!(
                    "{} version conflict{}:\n{}",
                    count,
                    if *count == 1 { "" } else { "s" },
                    details
                )
            }
        }
    }

    fn level(&self) -> HealthLevel {
        match self {
            Self::PythonOk { .. }
            | Self::SitePackagesOk
            | Self::CudaConsistent { .. }
            | Self::DependenciesOk => HealthLevel::Pass,
            Self::MissingDependencies { .. } => HealthLevel::Info,
            Self::CudaMismatch { .. }
            | Self::CpuCudaConflict { .. }
            | Self::VersionConflicts { .. } => HealthLevel::Warn,
            Self::PythonMissing | Self::BrokenSymlink { .. } | Self::SitePackagesMissing => {
                HealthLevel::Fail
            }
        }
    }
}

// =============================================================================
// HealthReport — collection of diagnostics
// =============================================================================

/// Result of an environment health check.
///
/// Collects typed `HealthDiagnostic` items and computes the overall severity.
#[derive(Default)]
pub struct HealthReport {
    pub items: Vec<HealthDiagnostic>,
}

impl HealthReport {
    /// Add a diagnostic to the report.
    pub fn push(&mut self, diagnostic: HealthDiagnostic) {
        self.items.push(diagnostic);
    }

    /// Overall status: Fail > Warn > Info > Pass.
    pub fn overall(&self) -> HealthLevel {
        self.items
            .iter()
            .map(|d| d.level())
            .max()
            .unwrap_or(HealthLevel::Pass)
    }

    /// Format as plain text for MCP/programmatic use.
    pub fn to_text(&self, env_name: &str) -> String {
        let mut out = format!("Health: {}\n", env_name);
        for item in &self.items {
            out.push_str(&format!("{} {}\n", item.level().icon(), item.message()));
        }
        out.push_str(&format!("\nOverall: {}", self.overall()));
        out
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_env_names() {
        assert!(EnvName::new("myenv").is_ok());
        assert!(EnvName::new("my-env-123").is_ok());
        assert!(EnvName::new("test_env").is_ok());
        assert!(EnvName::new("  trimmed  ").is_ok());
        assert_eq!(EnvName::new("  trimmed  ").unwrap().as_str(), "trimmed");
    }

    #[test]
    fn test_invalid_env_names() {
        assert!(EnvName::new("").is_err());
        assert!(EnvName::new("   ").is_err());
        assert!(EnvName::new("../escape").is_err());
        assert!(EnvName::new("path/to/env").is_err());
        assert!(EnvName::new("env;rm -rf").is_err());
        assert!(EnvName::new(".hidden").is_err());
        assert!(EnvName::new("$(whoami)").is_err());
    }

    #[test]
    fn test_env_name_deref() {
        let name = EnvName::new("test").unwrap();
        // Can use as &str transparently
        let s: &str = &name;
        assert_eq!(s, "test");
    }

    #[test]
    fn test_env_name_from_str() {
        let name: EnvName = "myenv".parse().unwrap();
        assert_eq!(name.as_str(), "myenv");
        assert!("".parse::<EnvName>().is_err());
    }

    #[test]
    fn test_env_name_display() {
        let name = EnvName::new("myenv").unwrap();
        assert_eq!(format!("{}", name), "myenv");
    }

    #[test]
    fn test_health_level_ordering() {
        assert!(HealthLevel::Pass < HealthLevel::Info);
        assert!(HealthLevel::Info < HealthLevel::Warn);
        assert!(HealthLevel::Warn < HealthLevel::Fail);
    }

    #[test]
    fn test_health_report_overall() {
        let mut report = HealthReport::default();
        report.push(HealthDiagnostic::PythonOk {
            version: "3.12".to_string(),
        });
        assert_eq!(report.overall(), HealthLevel::Pass);

        report.push(HealthDiagnostic::MissingDependencies {
            count: 1,
            details: "    foo".to_string(),
        });
        assert_eq!(report.overall(), HealthLevel::Info);

        report.push(HealthDiagnostic::PythonMissing);
        assert_eq!(report.overall(), HealthLevel::Fail);
    }

    #[test]
    fn test_diagnostic_messages() {
        let d = HealthDiagnostic::PythonOk {
            version: "3.12.1".to_string(),
        };
        assert_eq!(d.message(), "Python 3.12.1 OK");
        assert_eq!(d.level(), HealthLevel::Pass);

        let d = HealthDiagnostic::PythonMissing;
        assert!(d.message().contains("missing"));
        assert_eq!(d.level(), HealthLevel::Fail);
    }
}
