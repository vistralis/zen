// SPDX-License-Identifier: Apache-2.0

//! Database persistence layer for Zen.
//!
//! This module handles all interactions with the SQLite database, including
//! schema initialization, environment registry, project-environment association,
//! template storage, and project history (chat) logging.
use rusqlite::{Connection, OptionalExtension, params};
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Metadata for a single package in an environment.
#[derive(Debug, Clone, Default)]
pub struct PackageMetadata {
    pub name: String,
    pub version: Option<String>,
    pub installer: Option<String>,      // uv, pip
    pub install_source: Option<String>, // pypi, git, local
    pub is_editable: bool,
    pub source_url: Option<String>,  // git URL or file path
    pub commit_id: Option<String>,   // for git installs
    pub import_name: Option<String>, // primary Python import name (only if differs from pip name)
    pub installed_at: Option<i64>,   // epoch seconds from .dist-info mtime
}

/// The central database handle for Zen.
///
/// Wraps a thread-safe SQLite connection and provides high-level methods for
/// all of Zen's persistent data needs.
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

/// Current schema version. Increment when making schema changes.
/// - v1: Initial schema
/// - v2: Added project_environments, comments tables (v0.3.0)
/// - v3: Added labels table, removed dead tables
/// - v4: Added activation history columns to project_environments (v0.6.5)
const SCHEMA_VERSION: i32 = 4;

impl Database {
    /// Opens the Zen database at the specified path, or the default `~/.config/zen/zen.db`.
    ///
    /// Automatically initializes the schema if necessary.
    pub fn open(custom_path: Option<&Path>) -> Result<Self> {
        let db_path = if let Some(path) = custom_path {
            path.to_path_buf()
        } else {
            let config_dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".config/zen");
            std::fs::create_dir_all(&config_dir).ok();
            config_dir.join("zen.db")
        };
        let conn = Connection::open(&db_path)?;

        // Security L1: restrict DB to owner-only (prevents path enumeration on shared machines)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600)).ok();
        }
        // Enable WAL mode for better concurrency
        let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

        let db = Database {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        db.check_schema_version()?;
        Ok(db)
    }

    /// Check and handle schema version mismatch
    fn check_schema_version(&self) -> Result<()> {
        let stored_version = self
            .get_config("schema_version")?
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1); // Assume v1 if not set

        if stored_version < SCHEMA_VERSION {
            eprintln!(
                "⚠️  Database schema outdated (v{} → v{}). Upgrading...",
                stored_version, SCHEMA_VERSION
            );
            // Auto-migrate: currently all migrations are additive (CREATE TABLE IF NOT EXISTS)
            // so init_schema already handles them. Just update version.
            self.set_config("schema_version", &SCHEMA_VERSION.to_string())?;
            eprintln!("✓ Schema upgraded to v{}.", SCHEMA_VERSION);
        } else if stored_version > SCHEMA_VERSION {
            eprintln!(
                "⚠️  Warning: Database schema (v{}) is newer than this Zen version (v{}).",
                stored_version, SCHEMA_VERSION
            );
            eprintln!(
                "   Some features may not work. Consider updating Zen or run 'zen reset --yes'."
            );
        }
        Ok(())
    }

    /// Initializes all database tables and runs additive migrations.
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS environments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL UNIQUE,
                python_version TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_favorite INTEGER DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                env_id INTEGER,
                package_name TEXT NOT NULL,
                version TEXT,
                install_type TEXT, -- pypi, git, edit
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(env_id) REFERENCES environments(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Migration: Ensure new columns exist
        let _ = conn.execute(
            "ALTER TABLE environments ADD COLUMN is_favorite INTEGER DEFAULT 0",
            [],
        );
        // Migration: Add install_args column for pip arguments (--index-url, etc.)
        let _ = conn.execute(
            "ALTER TABLE template_packages ADD COLUMN install_args TEXT",
            [],
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                python_version TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(name, version)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_environments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                env_id INTEGER NOT NULL,
                is_default INTEGER DEFAULT 0,
                tag TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(env_id) REFERENCES environments(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS configuration (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_path ON project_environments(project_path)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS comments (
                uuid TEXT PRIMARY KEY,
                project_path TEXT NOT NULL,
                env_id INTEGER,
                message TEXT NOT NULL,
                tag TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(env_id) REFERENCES environments(id) ON DELETE SET NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS template_packages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id INTEGER,
                package_name TEXT NOT NULL,
                version TEXT NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                install_type TEXT,
                UNIQUE(template_id, package_name),
                FOREIGN KEY(template_id) REFERENCES templates(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS active_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id INTEGER,
                env_path TEXT NOT NULL,
                start_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(template_id) REFERENCES templates(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT
            )",
            [],
        )?;

        // Labels table for environment tagging (v0.5.0)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS labels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                env_id INTEGER NOT NULL,
                label TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(env_id, label),
                FOREIGN KEY(env_id) REFERENCES environments(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_labels_env ON labels(env_id)",
            [],
        )?;

        // v4: Activation history columns (safe to re-run — ALTER ignores existing columns)
        // SQLite doesn't support IF NOT EXISTS for ALTER, so we check pragma first
        let has_link_type: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('project_environments') WHERE name = 'link_type'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0) > 0;

        if !has_link_type {
            conn.execute_batch(
                "ALTER TABLE project_environments ADD COLUMN link_type TEXT DEFAULT 'user';
                 ALTER TABLE project_environments ADD COLUMN last_activated_at DATETIME;
                 ALTER TABLE project_environments ADD COLUMN activation_count INTEGER DEFAULT 0;",
            )?;
        }

        Ok(())
    }

    /// Registers a new environment in the database.
    ///
    /// If an environment with the same name already exists, it is updated.
    pub fn register_env(&self, name: &str, path: &str, python_version: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO environments (name, path, python_version, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
            params![name, path, python_version],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Logs a package installation event to the audit log.
    pub fn log_package(
        &self,
        env_id: i64,
        name: &str,
        version: &str,
        install_type: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audit_log (env_id, package_name, version, install_type)
             VALUES (?1, ?2, ?3, ?4)",
            params![env_id, name, version, install_type],
        )?;
        Ok(())
    }

    /// Gets the database ID for an environment by name.
    pub fn get_env_id(&self, name: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM environments WHERE name = ?1")?;
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Deletes an environment from the database.
    pub fn delete_env(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM environments WHERE name = ?1", params![name])?;
        Ok(())
    }

    // =========================================================================
    // Labels (v0.5.0)
    // =========================================================================

    /// Adds a label to an environment.
    pub fn add_label(&self, env_name: &str, label: &str) -> Result<()> {
        let env_id = self.get_env_id(env_name)?.ok_or("Environment not found")?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO labels (env_id, label) VALUES (?1, ?2)",
            params![env_id, label.to_lowercase()],
        )?;
        Ok(())
    }

    /// Removes a label from an environment.
    pub fn remove_label(&self, env_name: &str, label: &str) -> Result<()> {
        let env_id = self.get_env_id(env_name)?.ok_or("Environment not found")?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM labels WHERE env_id = ?1 AND label = ?2",
            params![env_id, label.to_lowercase()],
        )?;
        Ok(())
    }

    /// Gets all labels for an environment.
    pub fn get_labels(&self, env_name: &str) -> Result<Vec<String>> {
        let env_id = self.get_env_id(env_name)?.ok_or("Environment not found")?;
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT label FROM labels WHERE env_id = ?1 ORDER BY label")?;
        let labels = stmt
            .query_map(params![env_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(labels)
    }

    /// Gets all labels across all environments, grouped by env name.
    pub fn get_all_labels(&self) -> Result<Vec<(String, Vec<String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.name, l.label FROM labels l
             JOIN environments e ON e.id = l.env_id
             ORDER BY e.name, l.label",
        )?;
        let mut map: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (env, label) = row?;
            map.entry(env).or_default().push(label);
        }
        Ok(map.into_iter().collect())
    }

    /// Gets all environment names with a specific label.
    pub fn get_envs_by_label(&self, label: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.name FROM environments e 
             JOIN labels l ON e.id = l.env_id 
             WHERE l.label = ?1 
             ORDER BY e.name",
        )?;
        let names = stmt
            .query_map(params![label.to_lowercase()], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(names)
    }

    /// Checks if an environment has a specific label.
    #[allow(dead_code)]
    pub fn has_label(&self, env_name: &str, label: &str) -> Result<bool> {
        let env_id = self.get_env_id(env_name)?.ok_or("Environment not found")?;
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM labels WHERE env_id = ?1 AND label = ?2",
            params![env_id, label.to_lowercase()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Looks up an environment name by its filesystem path.
    #[allow(dead_code)]
    pub fn get_env_name_by_path(&self, path: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let name: Option<String> = conn
            .query_row(
                "SELECT name FROM environments WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .optional()?;
        Ok(name)
    }

    /// Lists all environments with basic info (name, path, python_version, updated_at, is_favorite).
    pub fn list_envs(
        &self,
    ) -> Result<
        Vec<(
            String, // name
            String, // path
            String, // python_version
            String, // updated_at
            bool,   // is_favorite
        )>,
    > {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, path, python_version, updated_at, is_favorite FROM environments",
        )?;
        let rows = stmt.query_map([], |row| {
            let is_fav: i32 = row.get(4)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                is_fav == 1,
            ))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Creates a new template or updates an existing one.
    pub fn create_template(&self, name: &str, version: &str, python_version: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO templates (name, version, python_version, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
            params![name, version, python_version],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Adds a package to a template definition.
    pub fn add_template_package(
        &self,
        template_id: i64,
        name: &str,
        version: &str,
        is_pinned: bool,
        install_type: &str,
        install_args: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let pinned = if is_pinned { 1 } else { 0 };
        conn.execute(
            "INSERT OR REPLACE INTO template_packages (template_id, package_name, version, is_pinned, install_type, install_args)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![template_id, name, version, pinned, install_type, install_args],
        )?;
        Ok(())
    }

    /// Starts a template recording session.
    pub fn start_session(&self, template_id: i64, env_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO active_sessions (template_id, env_path) VALUES (?1, ?2)",
            params![template_id, env_path],
        )?;
        Ok(())
    }

    /// Gets the currently active recording session, if any.
    pub fn get_active_session(&self) -> Result<Option<(i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT template_id, env_path FROM active_sessions LIMIT 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some((row.get(0)?, row.get(1)?)))
        } else {
            Ok(None)
        }
    }

    /// Clears all active recording sessions.
    pub fn clear_sessions(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM active_sessions", [])?;
        Ok(())
    }

    /// Gets the database ID for a template by name and version.
    pub fn get_template_id(&self, name: &str, version: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM templates WHERE name = ?1 AND version = ?2")?;
        let mut rows = stmt.query(params![name, version])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Returns all packages defined in a template.
    pub fn get_template_packages(
        &self,
        template_id: i64,
    ) -> Result<Vec<(String, String, bool, String, Option<String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT package_name, version, is_pinned, install_type, install_args FROM template_packages WHERE template_id = ?1")?;
        let rows = stmt.query_map(params![template_id], |row| {
            let is_pinned: i32 = row.get(2)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                is_pinned == 1,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Lists all templates with their name, version, and Python version.
    pub fn list_templates(&self) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name, version, python_version FROM templates")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Deletes a template and its associated packages by name.
    pub fn delete_template(&self, name: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        // First get the template ID
        let template_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM templates WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = template_id {
            // Delete associated packages first
            conn.execute(
                "DELETE FROM template_packages WHERE template_id = ?1",
                params![id],
            )?;
            // Then delete the template
            conn.execute("DELETE FROM templates WHERE id = ?1", params![id])?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // Project-Environment Association
    /// Associates a project directory with a specific Zen environment.
    ///
    /// This allows for context-aware activation and tool-use.
    /// If `is_default` is true, previous defaults for this project are cleared.
    pub fn associate_project(
        &self,
        project_path: &str,
        env_name: &str,
        tag: Option<&str>,
        is_default: bool,
    ) -> Result<()> {
        let env_id = self
            .get_env_id(env_name)?
            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;

        let conn = self.conn.lock().unwrap();
        // If setting as default, unset other defaults for this project
        if is_default {
            conn.execute(
                "UPDATE project_environments SET is_default = 0 WHERE project_path = ?1",
                params![project_path],
            )?;
        }

        // Remove existing association for same project+env (prevent duplicates)
        conn.execute(
            "DELETE FROM project_environments WHERE project_path = ?1 AND env_id = ?2",
            params![project_path, env_id],
        )?;

        // Insert new association with link_type='user' (explicit zen link)
        conn.execute(
            "INSERT INTO project_environments (project_path, env_id, tag, is_default, link_type)
             VALUES (?1, ?2, ?3, ?4, 'user')",
            params![project_path, env_id, tag, is_default as i32],
        )?;
        Ok(())
    }

    /// Returns all environments linked to a project directory, with activation metadata.
    pub fn get_project_environments(
        &self,
        project_path: &str,
    ) -> Result<Vec<(String, String, Option<String>, bool)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.name, e.path, pe.tag, pe.is_default
             FROM project_environments pe
             JOIN environments e ON pe.env_id = e.id
             WHERE pe.project_path = ?1
             ORDER BY pe.is_default DESC, pe.activation_count DESC, pe.created_at DESC",
        )?;

        let rows = stmt.query_map(params![project_path], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)? == 1,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Records an activation event for a project-environment pair.
    ///
    /// If the link already exists, increments `activation_count` and updates `last_activated_at`.
    /// If no link exists, creates one with `link_type='activated'`.
    pub fn record_activation(&self, project_path: &str, env_name: &str) -> Result<()> {
        let env_id = self
            .get_env_id(env_name)?
            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;

        let conn = self.conn.lock().unwrap();

        // Try to update existing link
        let updated = conn.execute(
            "UPDATE project_environments
             SET activation_count = activation_count + 1,
                 last_activated_at = CURRENT_TIMESTAMP
             WHERE project_path = ?1 AND env_id = ?2",
            params![project_path, env_id],
        )?;

        // No existing link — create one from activation
        if updated == 0 {
            conn.execute(
                "INSERT INTO project_environments
                 (project_path, env_id, link_type, activation_count, last_activated_at)
                 VALUES (?1, ?2, 'activated', 1, CURRENT_TIMESTAMP)",
                params![project_path, env_id],
            )?;
        }
        Ok(())
    }

    /// Returns activation candidates for multiple paths, sorted by relevance.
    ///
    /// Results are ordered: is_default DESC, then activation_count DESC, then recency.
    /// Each result includes: (env_name, env_path, project_path, activation_count, link_type).
    pub fn get_activation_candidates(
        &self,
        paths: &[String],
    ) -> Result<Vec<(String, String, String, i64, String)>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().unwrap();
        let placeholders: Vec<String> = paths
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT e.name, e.path, pe.project_path,
                    COALESCE(pe.activation_count, 0),
                    COALESCE(pe.link_type, 'user')
             FROM project_environments pe
             JOIN environments e ON pe.env_id = e.id
             WHERE pe.project_path IN ({})
             ORDER BY pe.is_default DESC, pe.activation_count DESC, pe.last_activated_at DESC",
            placeholders.join(", ")
        );

        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = paths
            .iter()
            .map(|p| p as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Returns activation candidates linked to subdirectories of the given path.
    ///
    /// Finds links where `project_path` starts with `base_path/` (not the base itself),
    /// filtered to at most `max_depth` directory levels below the base.
    /// Results are ordered: is_default DESC, activation_count DESC, recency DESC.
    pub fn get_subfolder_candidates(
        &self,
        base_path: &str,
        max_depth: usize,
    ) -> Result<Vec<(String, String, String, i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let prefix = format!("{}/", base_path.trim_end_matches('/'));
        let like_pattern = format!("{}%", prefix);

        let sql = "SELECT e.name, e.path, pe.project_path,
                COALESCE(pe.activation_count, 0),
                COALESCE(pe.link_type, 'user')
         FROM project_environments pe
         JOIN environments e ON pe.env_id = e.id
         WHERE pe.project_path LIKE ?1
         ORDER BY pe.is_default DESC, pe.activation_count DESC, pe.last_activated_at DESC";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([&like_pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let entry = row?;
            // Filter by depth: count path components below the base
            let relative = entry.2.strip_prefix(&prefix).unwrap_or("");
            let depth = relative.split('/').filter(|s| !s.is_empty()).count();
            if depth <= max_depth {
                result.push(entry);
            }
        }
        Ok(result)
    }

    /// Returns the most recently activated environment globally.
    ///
    /// Used by `zen activate --last` to re-activate the last used env.
    pub fn get_last_activated(&self) -> Result<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT e.name, e.path
                 FROM project_environments pe
                 JOIN environments e ON pe.env_id = e.id
                 WHERE pe.last_activated_at IS NOT NULL
                 ORDER BY pe.last_activated_at DESC
                 LIMIT 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(result)
    }

    /// Removes stale project-environment links where the env no longer exists on disk.
    ///
    /// Returns the list of pruned (project_path, env_name) pairs.
    pub fn prune_stale_links(&self) -> Result<Vec<(String, String, String)>> {
        // First, collect all links with their paths
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT pe.id, pe.project_path, e.name, e.path
             FROM project_environments pe
             JOIN environments e ON pe.env_id = e.id",
        )?;

        let links: Vec<(i64, String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut pruned = Vec::new();
        for (id, project_path, env_name, env_path) in &links {
            let env_gone = !std::path::Path::new(env_path).exists();
            let project_gone = !std::path::Path::new(project_path).exists();
            if env_gone || project_gone {
                conn.execute(
                    "DELETE FROM project_environments WHERE id = ?1",
                    params![id],
                )?;
                let reason = if env_gone {
                    "env deleted"
                } else {
                    "project dir missing"
                };
                pruned.push((project_path.clone(), env_name.clone(), reason.to_string()));
            }
        }
        Ok(pruned)
    }

    /// Resets activation history (zeroes counts and clears timestamps).
    /// If `older_than_days` is provided, only affects entries older than N days.
    pub fn reset_activation_history(&self, older_than_days: Option<u32>) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = if let Some(days) = older_than_days {
            conn.execute(
                "UPDATE project_environments SET activation_count = 0, last_activated_at = NULL
                 WHERE (activation_count > 0 OR last_activated_at IS NOT NULL)
                   AND last_activated_at < datetime('now', ?1)",
                params![format!("-{} days", days)],
            )?
        } else {
            conn.execute(
                "UPDATE project_environments SET activation_count = 0, last_activated_at = NULL
                 WHERE activation_count > 0 OR last_activated_at IS NOT NULL",
                [],
            )?
        };
        Ok(count)
    }

    /// Removes auto-created activation links (link_type='activated', not explicit user links).
    /// If `older_than_days` is provided, only affects entries older than N days.
    pub fn remove_activation_links(&self, older_than_days: Option<u32>) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = if let Some(days) = older_than_days {
            conn.execute(
                "DELETE FROM project_environments
                 WHERE link_type = 'activated'
                   AND (last_activated_at IS NULL OR last_activated_at < datetime('now', ?1))",
                params![format!("-{} days", days)],
            )?
        } else {
            conn.execute(
                "DELETE FROM project_environments WHERE link_type = 'activated'",
                [],
            )?
        };
        Ok(count)
    }

    /// Removes ALL project links for a given path, regardless of type.
    ///
    /// Returns the number of deleted links.
    pub fn remove_links_for_path(&self, project_path: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM project_environments WHERE project_path = ?1",
            params![project_path],
        )?;
        Ok(count)
    }

    /// Returns project links with activation metadata for `zen link list`.
    ///
    /// Each result: (env_name, env_path, tag, is_default, link_type, activation_count, last_activated_at).
    pub fn get_project_links_with_stats(
        &self,
        project_path: &str,
    ) -> Result<
        Vec<(
            String,
            String,
            Option<String>,
            bool,
            String,
            i64,
            Option<String>,
        )>,
    > {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.name, e.path, pe.tag, pe.is_default,
                    COALESCE(pe.link_type, 'user'),
                    COALESCE(pe.activation_count, 0),
                    pe.last_activated_at
             FROM project_environments pe
             JOIN environments e ON pe.env_id = e.id
             WHERE pe.project_path = ?1
             ORDER BY pe.is_default DESC, pe.activation_count DESC",
        )?;

        let rows = stmt.query_map(params![project_path], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)? == 1,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Returns the default environment name for a project, if one is set.
    pub fn get_default_environment(&self, project_path: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT e.name FROM project_environments pe
             JOIN environments e ON pe.env_id = e.id
             WHERE pe.project_path = ?1 AND pe.is_default = 1
             LIMIT 1",
                params![project_path],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(result)
    }

    /// Remove a project-environment association
    pub fn remove_project_association(&self, project_path: &str, env_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM project_environments WHERE project_path = ?1 AND env_id = ?2",
            params![project_path, env_id],
        )?;
        Ok(())
    }

    /// Get all unique project paths that have environment associations
    pub fn get_all_project_paths(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT DISTINCT project_path FROM project_environments")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Looks up an environment name by its database row ID.
    pub fn get_env_name_by_id(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let res = conn
            .query_row(
                "SELECT name FROM environments WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(res)
    }

    /// Stores a key-value configuration pair (upserts if key exists).
    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO configuration (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(())
    }

    /// Retrieves a configuration value by key.
    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let res = conn
            .query_row(
                "SELECT value FROM configuration WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(res)
    }

    /// Lists all configuration key-value pairs.
    pub fn list_all_config(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM configuration ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Returns all templates with their full package lists (for export/display).
    pub fn get_all_templates_with_packages(
        &self,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            Vec<(String, String, bool, String, Option<String>)>,
        )>,
    > {
        let (templates, _packages_map) = {
            let conn = self.conn.lock().unwrap();
            let mut stmt =
                conn.prepare("SELECT id, name, version, python_version FROM templates")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            (results, ()) // Just to isolate the lock if needed, but we call self.get_template_packages
        };

        let mut final_results = Vec::new();
        for (id, name, version, py_ver) in templates {
            let packages = self.get_template_packages(id)?;
            final_results.push((name, version, py_ver, packages));
        }
        Ok(final_results)
    }

    /// Inserts a new comment linked to a project and optionally an environment.
    pub fn add_comment(
        &self,
        uuid: &str,
        project_path: &str,
        env_id: Option<i64>,
        message: &str,
        tag: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO comments (uuid, project_path, env_id, message, tag) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![uuid, project_path, env_id, message, tag],
        )?;
        Ok(())
    }

    /// Lists comments filtered by project path and/or environment ID.
    pub fn list_comments(
        &self,
        project_path: Option<&str>,
        env_id: Option<i64>,
    ) -> Result<Vec<(String, String, Option<i64>, String, Option<String>, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut query =
            "SELECT uuid, project_path, env_id, message, tag, created_at FROM comments".to_string();
        let mut filters = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(pp) = project_path {
            filters.push("project_path = ?");
            params_vec.push(Box::new(pp.to_string()));
        }
        if let Some(eid) = env_id {
            filters.push("env_id = ?");
            params_vec.push(Box::new(eid));
        }

        if !filters.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&filters.join(" AND "));
        }
        query.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare(&query)?;

        // Convert Params to something stmt.query can use
        // Note: rusqlite params! works with slices, but dynamic params are trickier.
        // For simplicity, let's just handle the common cases manually since we only have 2 filters.

        let mapper = |row: &rusqlite::Row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        };

        let rows = if filters.len() == 2 {
            stmt.query_map(params![params_vec[0], params_vec[1]], mapper)?
        } else if filters.len() == 1 {
            stmt.query_map(params![params_vec[0]], mapper)?
        } else {
            stmt.query_map([], mapper)?
        };

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Removes a comment by UUID or UUID prefix.
    pub fn remove_comment(&self, uuid_prefix: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("{}%", uuid_prefix);
        let deleted = conn.execute("DELETE FROM comments WHERE uuid LIKE ?1", params![pattern])?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Creates a test database in a temporary directory.
    fn create_test_db() -> (Database, tempfile::TempDir) {
        let tmp_dir = tempdir().unwrap();
        let db_path = tmp_dir.path().join("test.db");
        let db = Database::open(Some(&db_path)).unwrap();
        (db, tmp_dir)
    }

    #[test]
    fn test_labels_crud() {
        let (db, _tmp) = create_test_db();

        // Register a test environment
        db.register_env("test_env", "/tmp/test_env", "3.12")
            .unwrap();

        // Add labels
        db.add_label("test_env", "ml").unwrap();
        db.add_label("test_env", "dev").unwrap();

        // Get labels
        let labels = db.get_labels("test_env").unwrap();
        assert!(labels.contains(&"ml".to_string()));
        assert!(labels.contains(&"dev".to_string()));
        assert_eq!(labels.len(), 2);

        // Check has_label
        assert!(db.has_label("test_env", "ml").unwrap());
        assert!(!db.has_label("test_env", "production").unwrap());

        // Remove label
        db.remove_label("test_env", "ml").unwrap();
        let labels = db.get_labels("test_env").unwrap();
        assert!(!labels.contains(&"ml".to_string()));
        assert!(labels.contains(&"dev".to_string()));

        // Duplicate label should be ignored
        db.add_label("test_env", "dev").unwrap();
        let labels = db.get_labels("test_env").unwrap();
        assert_eq!(labels.len(), 1);
    }

    #[test]
    fn test_get_envs_by_label() {
        let (db, _tmp) = create_test_db();

        // Register test environments
        db.register_env("env_ml", "/tmp/env_ml", "3.12").unwrap();
        db.register_env("env_dev", "/tmp/env_dev", "3.11").unwrap();
        db.register_env("env_both", "/tmp/env_both", "3.10")
            .unwrap();

        // Add labels
        db.add_label("env_ml", "ml").unwrap();
        db.add_label("env_dev", "dev").unwrap();
        db.add_label("env_both", "ml").unwrap();
        db.add_label("env_both", "dev").unwrap();

        // Get envs by label
        let ml_envs = db.get_envs_by_label("ml").unwrap();
        assert!(ml_envs.contains(&"env_ml".to_string()));
        assert!(ml_envs.contains(&"env_both".to_string()));
        assert!(!ml_envs.contains(&"env_dev".to_string()));
        assert_eq!(ml_envs.len(), 2);

        let dev_envs = db.get_envs_by_label("dev").unwrap();
        assert!(dev_envs.contains(&"env_dev".to_string()));
        assert!(dev_envs.contains(&"env_both".to_string()));
        assert_eq!(dev_envs.len(), 2);
    }

    #[test]
    fn test_labels_nonexistent_env() {
        let (db, _tmp) = create_test_db();

        // Try to add label to nonexistent env - should return error
        let result = db.add_label("nonexistent", "ml");
        assert!(result.is_err());
    }
}
