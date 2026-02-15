// SPDX-License-Identifier: Apache-2.0

//! CLI integration tests — run the zen binary as a subprocess with real venvs.
//!
//! Each test gets an isolated HOME + ZEN_HOME via tempdir, so tests are
//! fully independent and don't touch the developer's setup.

use std::process::Command;

/// Helper: run zen with an isolated config + env home.
fn zen_cmd(tmp: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_zen"))
        .args(args)
        .env("HOME", tmp)
        .env("ZEN_HOME", tmp.join("envs"))
        .output()
        .expect("failed to execute zen binary")
}

/// Combined stdout + stderr (zen prints to stderr for display).
fn all_output(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ── Version & Help ──────────────────────────────────────────────

#[test]
fn test_cli_version() {
    let tmp = tempfile::tempdir().unwrap();
    let out = zen_cmd(tmp.path(), &["--version"]);
    assert!(
        all_output(&out).contains("zen 0."),
        "unexpected: {}",
        all_output(&out)
    );
}

#[test]
fn test_cli_help() {
    let tmp = tempfile::tempdir().unwrap();
    let out = zen_cmd(tmp.path(), &["--help"]);
    assert!(
        all_output(&out).contains("Peace of mind"),
        "unexpected: {}",
        all_output(&out)
    );
}

// ── List (empty) ────────────────────────────────────────────────

#[test]
fn test_cli_list_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let out = zen_cmd(tmp.path(), &["list"]);
    assert!(out.status.success(), "failed: {}", all_output(&out));
}

// ── Create + List (real venv) ───────────────────────────────────

#[test]
fn test_cli_create_and_list() {
    let tmp = tempfile::tempdir().unwrap();

    let create = zen_cmd(tmp.path(), &["create", "cli-test"]);
    assert!(
        create.status.success(),
        "create failed: {}",
        all_output(&create)
    );

    let list = zen_cmd(tmp.path(), &["list"]);
    let out = all_output(&list);
    assert!(out.contains("cli-test"), "list missing env: {}", out);
}

// ── Create + Info (real venv) ───────────────────────────────────

#[test]
fn test_cli_create_and_info() {
    let tmp = tempfile::tempdir().unwrap();

    let create = zen_cmd(tmp.path(), &["create", "info-test"]);
    assert!(
        create.status.success(),
        "create failed: {}",
        all_output(&create)
    );

    let info = zen_cmd(tmp.path(), &["info", "info-test"]);
    assert!(info.status.success(), "info failed: {}", all_output(&info));
    let out = all_output(&info);
    assert!(out.contains("info-test"), "info missing env: {}", out);
}

// ── Create duplicate guard ──────────────────────────────────────

#[test]
fn test_cli_create_duplicate_blocked() {
    let tmp = tempfile::tempdir().unwrap();

    zen_cmd(tmp.path(), &["create", "dup-test"]);
    let dup = zen_cmd(tmp.path(), &["create", "dup-test"]);
    let combined = all_output(&dup);
    assert!(
        !dup.status.success() || combined.to_lowercase().contains("already"),
        "duplicate should be rejected: {}",
        combined
    );
}

// ── Label lifecycle (real venv) ─────────────────────────────────
// CLI usage: zen label add <LABEL> [ENV]

#[test]
fn test_cli_label_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();

    zen_cmd(tmp.path(), &["create", "label-env"]);

    // Add: zen label add <label> <env>
    let add = zen_cmd(tmp.path(), &["label", "add", "ml", "label-env"]);
    assert!(add.status.success(), "label add: {}", all_output(&add));

    // List
    let list = zen_cmd(tmp.path(), &["label", "list", "label-env"]);
    assert!(
        all_output(&list).contains("ml"),
        "label list: {}",
        all_output(&list)
    );

    // Remove: zen label rm <label> <env>
    let rm = zen_cmd(tmp.path(), &["label", "rm", "ml", "label-env"]);
    assert!(rm.status.success(), "label rm: {}", all_output(&rm));

    // Verify removed
    let list2 = zen_cmd(tmp.path(), &["label", "list", "label-env"]);
    let out = all_output(&list2);
    assert!(
        !out.contains("ml") || out.contains("No labels"),
        "label should be gone: {}",
        out
    );
}

// ── Note lifecycle (real venv) ──────────────────────────────────
// CLI usage: zen note add <MESSAGE> [ENV]

#[test]
fn test_cli_note_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();

    zen_cmd(tmp.path(), &["create", "note-env"]);

    // Add: zen note add <message> <env>
    let add = zen_cmd(
        tmp.path(),
        &["note", "add", "Test note content", "note-env"],
    );
    assert!(add.status.success(), "note add: {}", all_output(&add));

    // List
    let list = zen_cmd(tmp.path(), &["note", "list", "note-env"]);
    assert!(
        all_output(&list).contains("Test note"),
        "note list: {}",
        all_output(&list)
    );
}

// ── Invalid name rejected ───────────────────────────────────────

#[test]
fn test_cli_name_validation() {
    let tmp = tempfile::tempdir().unwrap();

    let traversal = zen_cmd(tmp.path(), &["create", "../escape"]);
    assert!(!traversal.status.success(), "path traversal should fail");

    let injection = zen_cmd(tmp.path(), &["create", "env;rm -rf"]);
    assert!(!injection.status.success(), "shell injection should fail");

    let hidden = zen_cmd(tmp.path(), &["create", ".hidden"]);
    assert!(!hidden.status.success(), "hidden name should fail");
}

// ── Health check (real venv) ────────────────────────────────────

#[test]
fn test_cli_health() {
    let tmp = tempfile::tempdir().unwrap();

    zen_cmd(tmp.path(), &["create", "health-env"]);

    let health = zen_cmd(tmp.path(), &["health", "health-env"]);
    assert!(health.status.success(), "health: {}", all_output(&health));
}

// ── Remove (real venv) ──────────────────────────────────────────

#[test]
fn test_cli_create_and_remove() {
    let tmp = tempfile::tempdir().unwrap();

    zen_cmd(tmp.path(), &["create", "rm-env"]);

    let rm = zen_cmd(tmp.path(), &["rm", "rm-env", "--yes"]);
    assert!(rm.status.success(), "remove: {}", all_output(&rm));

    // Should be gone
    let list = zen_cmd(tmp.path(), &["list"]);
    assert!(
        !all_output(&list).contains("rm-env"),
        "env still in list after remove"
    );
}

// ── DB file permissions (Security L1) ───────────────────────────

#[test]
#[cfg(unix)]
fn test_db_permissions_0600() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();

    // Any zen command triggers DB creation
    zen_cmd(tmp.path(), &["list"]);

    let db_path = tmp.path().join(".config/zen/zen.db");
    assert!(db_path.exists(), "DB not created at {:?}", db_path);

    let perms = std::fs::metadata(&db_path).unwrap().permissions();
    assert_eq!(
        perms.mode() & 0o777,
        0o600,
        "DB permissions should be 0600, got {:o}",
        perms.mode() & 0o777
    );
}
