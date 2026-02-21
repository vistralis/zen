// SPDX-License-Identifier: Apache-2.0

//! Application context — shared state across CLI and MCP surfaces.
//!
//! `OutputMode` tells ops-layer methods whether they're running in a
//! colored CLI terminal or a plain MCP context, so they can format
//! response strings appropriately.

use owo_colors::OwoColorize;

/// Output mode for formatting ops-layer responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// ANSI-colored output for interactive CLI.
    Cli,
    /// Plain-text output for MCP / structured consumers.
    Plain,
}

impl OutputMode {
    /// Returns a success check mark, colored for CLI, plain for MCP.
    pub fn ok_mark(self) -> String {
        match self {
            Self::Cli => "✓".green().to_string(),
            Self::Plain => "✓".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_mode_ok_mark_plain() {
        let plain = OutputMode::Plain.ok_mark();
        assert_eq!(plain, "✓");
        // Plain output must not contain ANSI escape sequences
        assert!(!plain.contains('\x1b'));
    }

    #[test]
    fn test_output_mode_ok_mark_cli() {
        let cli = OutputMode::Cli.ok_mark();
        // CLI output should contain ANSI escape codes for green
        assert!(cli.contains('\x1b'));
        assert!(cli.contains('✓'));
    }

    #[test]
    fn test_output_mode_equality() {
        assert_eq!(OutputMode::Cli, OutputMode::Cli);
        assert_eq!(OutputMode::Plain, OutputMode::Plain);
        assert_ne!(OutputMode::Cli, OutputMode::Plain);
    }
}
