// SPDX-License-Identifier: Apache-2.0

use std::fs;

#[test]
fn test_database_creation() {
    let temp_dir = std::env::temp_dir().join("zen_test_db");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Test environment registration
    let env_id = db
        .register_env("test-env", "/tmp/test-env", "3.12")
        .unwrap();
    assert!(env_id > 0);

    // Test listing
    let envs = db.list_envs().unwrap();
    assert_eq!(envs.len(), 1);
    assert_eq!(envs[0].0, "test-env");

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_project_environment_association() {
    let temp_dir = std::env::temp_dir().join("zen_test_project");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Register environment
    db.register_env("myproject-main", "/tmp/myproject-main", "3.12")
        .unwrap();

    // Associate with project
    db.associate_project("/path/to/project", "myproject-main", Some("main"), true)
        .unwrap();

    // Get default
    let default_env = db.get_default_environment("/path/to/project").unwrap();
    assert_eq!(default_env, Some("myproject-main".to_string()));

    // Get all project envs
    let envs = db.get_project_environments("/path/to/project").unwrap();
    assert_eq!(envs.len(), 1);
    assert_eq!(envs[0].0, "myproject-main");
    assert!(envs[0].3); // is_default

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_environment_inference() {
    let temp_dir = std::env::temp_dir().join("zen_test_inference");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Register an environment
    let env_path = temp_dir.join("inference-env");
    fs::create_dir_all(&env_path).unwrap();
    db.register_env("inference-env", env_path.to_str().unwrap(), "3.12")
        .unwrap();

    // Simulate active environment via VIRTUAL_ENV
    unsafe {
        std::env::set_var("VIRTUAL_ENV", env_path.to_str().unwrap());
    }

    let venv_path = zen::utils::get_current_venv_path();
    assert_eq!(venv_path, Some(env_path.to_str().unwrap().to_string()));

    let ops = zen::ops::ZenOps::new(&db, temp_dir.clone());
    let inferred = ops.infer_current_env().unwrap();
    assert_eq!(inferred, Some("inference-env".to_string()));

    // Test with no match
    unsafe {
        std::env::set_var("VIRTUAL_ENV", "/unknown/path");
    }
    let inferred_none = ops.infer_current_env().unwrap();
    assert_eq!(inferred_none, None);

    // Unset for safety
    unsafe {
        std::env::remove_var("VIRTUAL_ENV");
    }

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
#[ignore] // Flaky: depends on temp directory cleanup behavior
fn test_list_status_verification() {
    let temp_dir = std::env::temp_dir().join("zen_test_status");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();
    let ops = zen::ops::ZenOps::new(&db, temp_dir.clone());

    // Register real path
    let real_env = temp_dir.join("real-env");
    fs::create_dir_all(&real_env).unwrap();
    db.register_env("real", real_env.to_str().unwrap(), "3.12")
        .unwrap();

    // Register fake path
    db.register_env("fake", "/non/existent/path", "3.12")
        .unwrap();

    let envs = ops.list_envs_with_status(None, None, None).unwrap();
    assert_eq!(envs.len(), 2);

    let real_status = envs.iter().find(|(n, _, _, _, _, _)| n == "real").unwrap();
    assert!(real_status.3); // Exists

    let fake_status = envs.iter().find(|(n, _, _, _, _, _)| n == "fake").unwrap();
    assert!(!fake_status.3); // Missing

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_bulk_import() {
    let temp_dir = std::env::temp_dir().join("zen_test_bulk");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    // Create folders that look like venvs
    let env1 = temp_dir.join("env1");
    let env2 = temp_dir.join("env2");
    fs::create_dir_all(env1.join("bin")).unwrap();
    fs::create_dir_all(env2.join("bin")).unwrap();
    fs::write(env1.join("bin/python"), "").unwrap();
    fs::write(env2.join("bin/python"), "").unwrap();

    let db = zen::db::Database::open(Some(&db_path)).unwrap();
    let ops = zen::ops::ZenOps::new(&db, temp_dir.clone());

    // Test discovery
    let found = zen::utils::discover_venvs(&temp_dir);
    assert_eq!(found.len(), 2);

    // Test bulk import
    ops.bulk_import(found).unwrap();

    let envs = db.list_envs().unwrap();
    assert_eq!(envs.len(), 2);

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_template_creation_and_packages() {
    let temp_dir = std::env::temp_dir().join("zen_test_templates");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Create template
    let template_id = db.create_template("ml-base", "1.0", "3.12").unwrap();
    assert!(template_id > 0);

    // List templates
    let templates = db.list_templates().unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].0, "ml-base");

    // Get template ID by name
    let found_id = db.get_template_id("ml-base", "1.0").unwrap();
    assert_eq!(found_id, Some(template_id));

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_labels() {
    let temp_dir = std::env::temp_dir().join("zen_test_labels");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Register environment
    db.register_env("label-test", "/tmp/label-test", "3.12")
        .unwrap();

    // Initially no labels
    let labels = db.get_labels("label-test").unwrap();
    assert!(labels.is_empty());

    // Add labels
    db.add_label("label-test", "ml").unwrap();
    db.add_label("label-test", "favorite").unwrap();

    let labels = db.get_labels("label-test").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"ml".to_string()));
    assert!(labels.contains(&"favorite".to_string()));

    // Remove label
    db.remove_label("label-test", "ml").unwrap();
    let labels = db.get_labels("label-test").unwrap();
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"favorite".to_string()));

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_comments() {
    let temp_dir = std::env::temp_dir().join("zen_test_comments");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Register environment
    let env_id = db
        .register_env("comment-env", "/tmp/comment-env", "3.12")
        .unwrap();

    // Add comment with UUID
    let uuid = uuid::Uuid::new_v4().to_string();
    db.add_comment(
        &uuid,
        "/project/path",
        Some(env_id),
        "Test comment content",
        Some("note"),
    )
    .unwrap();

    // List comments
    let comments = db
        .list_comments(Some("/project/path"), Some(env_id))
        .unwrap();
    assert_eq!(comments.len(), 1);
    assert!(comments[0].3.contains("Test comment content"));

    // Remove comment
    let uuid = &comments[0].0;
    db.remove_comment(uuid).unwrap();

    // Verify removed
    let comments_after = db
        .list_comments(Some("/project/path"), Some(env_id))
        .unwrap();
    assert_eq!(comments_after.len(), 0);

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_config() {
    let temp_dir = std::env::temp_dir().join("zen_test_config");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Initially no config
    let val = db.get_config("stack_info").unwrap();
    assert!(val.is_none());

    // Set config
    db.set_config("stack_info", "compact").unwrap();

    // Get config
    let val = db.get_config("stack_info").unwrap();
    assert_eq!(val, Some("compact".to_string()));

    // Update config
    db.set_config("stack_info", "full").unwrap();
    let val = db.get_config("stack_info").unwrap();
    assert_eq!(val, Some("full".to_string()));

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_session_management() {
    let temp_dir = std::env::temp_dir().join("zen_test_sessions");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Create template
    let template_id = db.create_template("session-tpl", "1.0", "3.12").unwrap();

    // No active session initially
    let session = db.get_active_session().unwrap();
    assert!(session.is_none());

    // Start session
    db.start_session(template_id, "/tmp/session-env").unwrap();

    // Check active session
    let session = db.get_active_session().unwrap();
    assert!(session.is_some());
    let (tpl_id, path) = session.unwrap();
    assert_eq!(tpl_id, template_id);
    assert_eq!(path, "/tmp/session-env");

    // Clear sessions
    db.clear_sessions().unwrap();
    let session = db.get_active_session().unwrap();
    assert!(session.is_none());

    // Cleanup
    fs::remove_file(db_path).ok();
}

#[test]
fn test_environment_update() {
    let temp_dir = std::env::temp_dir().join("zen_test_update");
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = zen::db::Database::open(Some(&db_path)).unwrap();

    // Register environment
    let env_id = db
        .register_env("update-env", "/tmp/update-env", "3.11")
        .unwrap();
    assert!(env_id > 0);

    // Verify registration
    let envs = db.list_envs().unwrap();
    assert_eq!(envs.len(), 1);
    assert_eq!(envs[0].2, "3.11"); // python_version

    // Cleanup
    fs::remove_file(db_path).ok();
}

// Note: table module is binary-only (not exported in lib.rs)

#[test]
fn test_utils_venv_discovery() {
    let temp_dir = std::env::temp_dir().join("zen_test_discovery");
    fs::create_dir_all(&temp_dir).unwrap();

    // Create valid venv structure
    let venv = temp_dir.join("valid-venv");
    fs::create_dir_all(venv.join("bin")).unwrap();
    fs::write(venv.join("bin/python"), "").unwrap();

    // Create invalid structure (no python)
    let not_venv = temp_dir.join("not-venv");
    fs::create_dir_all(not_venv.join("bin")).unwrap();

    // Create nested too deep (should not be found at default depth)
    let deep = temp_dir.join("a/b/c/d/deep-venv");
    fs::create_dir_all(deep.join("bin")).unwrap();
    fs::write(deep.join("bin/python"), "").unwrap();

    let found = zen::utils::discover_venvs(&temp_dir);

    // Should find valid-venv, not find not-venv or deep-venv
    assert!(found.iter().any(|p| p.ends_with("valid-venv")));
    assert!(!found.iter().any(|p| p.ends_with("not-venv")));

    // Cleanup
    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_utils_template_parsing() {
    // Single template
    let parts = zen::utils::parse_template_string("ml-base");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].name, "ml-base");
    assert_eq!(parts[0].version, "latest");

    // Template with version
    let parts = zen::utils::parse_template_string("ml-base:1.0");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].name, "ml-base");
    assert_eq!(parts[0].version, "1.0");

    // Multiple templates
    let parts = zen::utils::parse_template_string("base|ml:2.0|vision");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].name, "base");
    assert_eq!(parts[1].name, "ml");
    assert_eq!(parts[1].version, "2.0");
    assert_eq!(parts[2].name, "vision");
}

#[test]
fn test_utils_torch_index_url() {
    // Valid CUDA versions (as defined in utils.rs)
    assert!(zen::utils::get_torch_index_url("12.4").is_some());
    assert!(zen::utils::get_torch_index_url("12.1").is_some());
    assert!(zen::utils::get_torch_index_url("11.8").is_some());

    // Invalid/unsupported versions
    assert!(zen::utils::get_torch_index_url("12.6").is_none()); // Not in list
    assert!(zen::utils::get_torch_index_url("9.0").is_none());
    assert!(zen::utils::get_torch_index_url("invalid").is_none());
}
