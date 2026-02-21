// SPDX-License-Identifier: Apache-2.0

//! Structured error types for Zen — retriable vs system errors.
//!
//! `ZenError` classifies failures so that MCP can tell an LLM whether
//! to retry (user/input errors) or stop (system errors).

use crate::types::EnvNameError;

/// The unified error type for Zen operations.
///
/// Variants are split into two categories:
/// - **Retriable** (user/AI errors) — the caller can self-correct and retry.
/// - **System** — infrastructure failures that retrying won't fix.
#[derive(thiserror::Error, Debug)]
#[allow(dead_code)]
pub enum ZenError {
    // === Retriable — LLM can self-correct ===
    /// A requested resource (environment, package, label, etc.) was not found.
    #[error("not found: {kind} '{name}'")]
    NotFound { kind: &'static str, name: String },

    /// A resource with the given name already exists.
    #[error("already exists: {kind} '{name}'")]
    AlreadyExists { kind: &'static str, name: String },

    /// The input failed validation (e.g. bad env name, invalid version).
    #[error("invalid input for {field}: {reason}")]
    InvalidInput { field: &'static str, reason: String },

    // === System — LLM should stop trying ===
    /// A database operation failed.
    #[error("database error: {0}")]
    Database(String),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A subprocess exited with a non-zero status.
    #[error("command failed: {cmd} (exit {code}): {stderr}")]
    CommandFailed {
        cmd: String,
        code: i32,
        stderr: String,
    },
}

#[allow(dead_code)]
impl ZenError {
    /// Whether an LLM/caller should retry with corrected input.
    ///
    /// Returns `true` for user-correctable errors (`NotFound`, `AlreadyExists`,
    /// `InvalidInput`), `false` for infrastructure errors.
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            Self::NotFound { .. } | Self::AlreadyExists { .. } | Self::InvalidInput { .. }
        )
    }
}

impl From<EnvNameError> for ZenError {
    fn from(e: EnvNameError) -> Self {
        Self::InvalidInput {
            field: "env_name",
            reason: e.to_string(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zen_error_retriable_variants() {
        let not_found = ZenError::NotFound {
            kind: "environment",
            name: "ml".to_string(),
        };
        assert!(not_found.is_retriable());
        assert!(not_found.to_string().contains("not found"));
        assert!(not_found.to_string().contains("ml"));

        let already = ZenError::AlreadyExists {
            kind: "environment",
            name: "ml".to_string(),
        };
        assert!(already.is_retriable());
        assert!(already.to_string().contains("already exists"));

        let invalid = ZenError::InvalidInput {
            field: "env_name",
            reason: "cannot be empty".to_string(),
        };
        assert!(invalid.is_retriable());
        assert!(invalid.to_string().contains("invalid input"));
    }

    #[test]
    fn test_zen_error_system_variants() {
        let db_err = ZenError::Database("connection lost".to_string());
        assert!(!db_err.is_retriable());
        assert!(db_err.to_string().contains("database error"));

        let io_err = ZenError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file missing",
        ));
        assert!(!io_err.is_retriable());

        let cmd_err = ZenError::CommandFailed {
            cmd: "pip install".to_string(),
            code: 1,
            stderr: "No such package".to_string(),
        };
        assert!(!cmd_err.is_retriable());
        assert!(cmd_err.to_string().contains("pip install"));
        assert!(cmd_err.to_string().contains("exit 1"));
    }

    #[test]
    fn test_zen_error_from_env_name_error() {
        use std::str::FromStr;
        let env_err = crate::types::EnvName::from_str("").unwrap_err();
        let zen_err: ZenError = env_err.into();
        assert!(zen_err.is_retriable());
        match &zen_err {
            ZenError::InvalidInput { field, reason } => {
                assert_eq!(*field, "env_name");
                assert!(reason.contains("empty") || reason.contains("Invalid"));
            }
            other => panic!("expected InvalidInput, got: {:?}", other),
        }
    }
}
