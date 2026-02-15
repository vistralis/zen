// SPDX-License-Identifier: Apache-2.0

//! Table formatting utilities for consistent CLI output.
//!
//! This module provides a helper function for creating styled tables
//! that maintain consistent formatting even with colored text.

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{ContentArrangement, Table};

/// Creates a new styled table with consistent formatting.
///
/// The table uses UTF-8 borders with rounded corners and handles
/// colored text width correctly.
pub fn new_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.apply_modifier(UTF8_ROUND_CORNERS);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Creates a new styled table with custom headers.
#[allow(dead_code)]
pub fn new_table_with_headers(headers: Vec<&str>) -> Table {
    let mut table = new_table();
    table.set_header(headers);
    table
}
