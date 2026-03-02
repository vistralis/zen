// SPDX-License-Identifier: Apache-2.0

//! Shared output types for MCP and CLI JSON responses.
//!
//! These structs are the canonical serialization format for all Zen output.
//! Both the MCP server and CLI `--json` use these types directly.

use serde::Serialize;

/// Summary of a single environment (used by list_environments).
#[derive(Serialize)]
pub struct EnvSummary {
    pub name: String,
    pub python: String,
    pub path: String,
}

/// Detailed view of a single environment (used by inspect_environment details).
#[derive(Serialize)]
pub struct EnvDetails {
    pub name: String,
    pub python: String,
    pub path: String,
    pub packages: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cuda: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numpy: Option<String>,
}

/// Single health check result.
#[derive(Serialize)]
pub struct HealthCheck {
    pub check: String,
    pub status: String,
    pub message: String,
}

/// Full health report for an environment.
#[derive(Serialize)]
pub struct HealthResponse {
    pub env_name: String,
    pub overall: String,
    pub checks: Vec<HealthCheck>,
}

/// A package found across environments (used by find_package cross-env search).
#[derive(Serialize)]
pub struct PackageMatch {
    pub env: String,
    pub package: String,
    pub version: String,
}

/// Detailed info for a single package in a specific environment.
#[derive(Serialize)]
pub struct PackageDetails {
    pub name: String,
    pub env: String,
    pub version: String,
    pub installer: String,
    pub source: String,
    pub editable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<String>,
}

/// Side-by-side environment comparison result.
#[derive(Serialize)]
pub struct ComparisonResult {
    pub environments: Vec<EnvCompare>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub version_diffs: Vec<VersionDiff>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub only_in: Vec<OnlyIn>,
}

/// Environment summary within a comparison.
#[derive(Serialize)]
pub struct EnvCompare {
    pub name: String,
    pub python: String,
    pub packages: usize,
}

/// Version difference for a package across compared environments.
#[derive(Serialize)]
pub struct VersionDiff {
    pub package: String,
    pub versions: Vec<String>,
}

/// Packages that exist only in one environment.
#[derive(Serialize)]
pub struct OnlyIn {
    pub env: String,
    pub packages: Vec<String>,
}

/// Result of running a command in an environment.
#[derive(Serialize)]
pub struct RunResult {
    pub exit_code: i32,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timed_out: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<String>,
}

/// A project-environment link entry.
#[derive(Serialize)]
pub struct ProjectLink {
    pub env: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub is_default: bool,
}

/// A note entry for an environment.
#[derive(Serialize)]
pub struct NoteEntry {
    pub timestamp: String,
    pub text: String,
}

/// Generic action result (create, remove, rename, install, etc.).
#[derive(Serialize)]
pub struct ActionResult {
    pub message: String,
}

/// Version response for get_version tool.
#[derive(Serialize)]
pub struct VersionResponse {
    pub version: String,
}
