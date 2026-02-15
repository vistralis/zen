//! Activity log — bash-history-style append-only audit trail.
//!
//! Log file: `~/.config/zen/zen.log`
//! Format:   `YYYY-MM-DD HH:MM:SS [source] action details`

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Max lines to keep before rotating.
const MAX_LINES: usize = 1000;

/// Returns the path to the log file (`~/.config/zen/zen.log`).
fn log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let dir = PathBuf::from(home).join(".config/zen");
    std::fs::create_dir_all(&dir).ok();
    dir.join("zen.log")
}

/// Append a single line to the activity log.
///
/// `source` is `"cli"` or `"mcp"`.
/// `action` is a verb like `create`, `rm`, `install`, etc.
/// `details` is free-form context (env name, packages, etc.).
///
/// Automatically trims the log to [`MAX_LINES`] when it grows past threshold.
pub fn log_activity(source: &str, action: &str, details: &str) {
    let path = log_path();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("{} [{}] {} {}\n", now, source, action, details);

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }

    // Rotate when file exceeds ~100KB (avg ~80 bytes/line × 1000 ≈ 80KB)
    if let Ok(meta) = std::fs::metadata(&path)
        && meta.len() > 100_000
    {
        rotate(&path);
    }
}

/// Keep only the last MAX_LINES lines in the log file.
fn rotate(path: &PathBuf) {
    if let Ok(content) = std::fs::read_to_string(path) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > MAX_LINES {
            let keep = &lines[lines.len() - MAX_LINES..];
            let _ = std::fs::write(path, keep.join("\n") + "\n");
        }
    }
}

/// Delete the log file contents.
pub fn clear_log() {
    let _ = std::fs::write(log_path(), "");
}

/// Read the last `n` lines from the log, optionally filtering by a keyword.
pub fn read_log(n: usize, filter: Option<&str>) -> Vec<String> {
    let path = log_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();

    let filtered: Vec<String> = if let Some(keyword) = filter {
        let kw = keyword.to_lowercase();
        lines
            .iter()
            .filter(|l| l.to_lowercase().contains(&kw))
            .map(|l| l.to_string())
            .collect()
    } else {
        lines.iter().map(|l| l.to_string()).collect()
    };

    // Return last N
    let skip = filtered.len().saturating_sub(n);
    filtered.into_iter().skip(skip).collect()
}
