// SPDX-License-Identifier: Apache-2.0

//! Output control for Zen — "Verbosity as a Type" pattern.
//!
//! The `Printer` enum prevents stray `println!` from corrupting MCP output.
//! All CLI output goes through the printer, which silently discards output
//! when running in MCP mode.
//!
//! Inspired by uv's `Printer` enum (Silent/Quiet/Default/Verbose/NoProgress).

use owo_colors::OwoColorize;

/// Controls all zen terminal output.
///
/// In `Default` mode, output goes to stdout/stderr with colors.
/// In `Silent` mode (MCP), all output is suppressed — the MCP layer
/// returns structured data instead.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Printer {
    /// Normal CLI output with colors.
    Default,
    /// Suppress all terminal output (MCP mode).
    Silent,
}

#[allow(dead_code)]
impl Printer {
    /// Print a plain message to stdout.
    pub fn println(&self, msg: &str) {
        if *self == Self::Default {
            println!("{msg}");
        }
    }

    /// Print a success message (Zen Blue ✓ prefix).
    pub fn success(&self, msg: &str) {
        if *self == Self::Default {
            println!("  {} {}", "✓".truecolor(100, 200, 255), msg);
        }
    }

    /// Print an info message (Peace Pink △ prefix).
    pub fn info(&self, msg: &str) {
        if *self == Self::Default {
            println!("  {} {}", "△".truecolor(255, 182, 193), msg);
        }
    }

    /// Print a warning message (Stressed Orange ⚠ prefix).
    pub fn warning(&self, msg: &str) {
        if *self == Self::Default {
            eprintln!("  {} {}", "⚠".truecolor(255, 140, 0), msg);
        }
    }

    /// Print an error message (Lava Red ✗ prefix).
    pub fn error(&self, msg: &str) {
        if *self == Self::Default {
            eprintln!("  {} {}", "✗".red(), msg);
        }
    }

    /// Print a comfy_table::Table to stdout.
    pub fn table(&self, table: &comfy_table::Table) {
        if *self == Self::Default {
            println!("{table}");
        }
    }

    /// Print a formatted string (like println! but routed through the printer).
    pub fn status(&self, msg: &str) {
        if *self == Self::Default {
            println!("{msg}");
        }
    }
}
