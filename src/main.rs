// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::type_complexity)]

mod activity_log;
mod db;
mod hooks;
mod mcp;
mod ops;
mod printer;
mod table;
mod types;
mod utils;
mod validation;

use crate::db::Database;
use clap::{Parser, Subcommand};
use colored::*;
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "zen")]
#[command(version = env!("ZEN_VERSION"))]
#[command(about = "Peace of mind for Python environments", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Custom database path (for development/testing)
    #[arg(long, env = "ZEN_DOJO", hide = true)]
    db_path: Option<PathBuf>,

    /// Custom environment home (for development/testing)
    #[arg(
        long,
        env = "ZEN_HOME",
        default_value = "~/.local/share/zen/envs",
        hide = true
    )]
    home: PathBuf,
}

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    /// Create a new virtual environment
    Create {
        /// Name of the environment
        name: String,

        /// Python version to use (if not specified by template)
        #[arg(long)]
        python: Option<String>,

        /// Template(s) to apply (e.g., spatial-torch:2.10)
        #[arg(short, long, alias = "from")]
        template: Option<String>,

        /// Use exact versions from template snapshots
        #[arg(long)]
        strict: bool,

        /// Install ML stack (PyTorch, torchvision, torchaudio)
        #[arg(long)]
        ml: bool,

        /// CUDA version for ML stack (requires --ml, e.g., "12.6", "12.8", "13.0")
        #[arg(long, requires = "ml")]
        cuda: Option<String>,

        /// Remove existing environment with the same name before creating
        #[arg(long)]
        rm: bool,
    },
    /// List all managed environments
    List {
        /// Wildcard pattern to filter environments (e.g., *ai*)
        pattern: Option<String>,
        /// Sort by field (name, date)
        #[arg(short, long, default_value = "name")]
        sort: String,
        /// Number of environments to show
        #[arg(short = 'n', long)]
        num: Option<usize>,
        /// Filter by label (e.g., --label ml)
        #[arg(short = 'l', long)]
        label: Option<String>,
        /// Output format: auto, minimal, compact, wide (default: auto)
        #[arg(short = 'f', long, default_value = "auto")]
        format: String,
    },
    /// Remove an environment from the database and disk
    Rm {
        /// Name of the environment to remove
        name: String,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Add packages to an environment (or active session)
    ///
    /// Examples:
    ///   zen install numpy scipy          # install in active environment
    ///   zen install -n myenv requests    # install in a specific environment
    ///   zen install torch-cu130          # install PyTorch with CUDA 13.0
    Install {
        /// Packages to install (or paths for -e)
        packages: Vec<String>,
        /// Environment name (uses active environment if omitted)
        #[arg(short = 'n', long = "name")]
        env: Option<String>,
        /// Pin these packages in the template (if in a session)
        #[arg(long)]
        pinned: bool,
        /// Custom PyPI index URL (e.g., https://download.pytorch.org/whl/cu130)
        #[arg(long)]
        index_url: Option<String>,
        /// Additional PyPI index URL (used alongside default)
        #[arg(long)]
        extra_index_url: Option<String>,
        /// Install in editable/development mode (like pip install -e)
        #[arg(short = 'e', long)]
        editable: bool,
        /// Include pre-release/development versions
        #[arg(long)]
        pre: bool,
        /// Upgrade packages to latest version
        #[arg(short = 'U', long)]
        upgrade: bool,
        /// Show what would be installed without actually installing
        #[arg(long)]
        dry_run: bool,
    },
    /// Run a command inside an environment without activating it
    Run {
        /// Environment name
        name: String,
        /// Command and arguments to run
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Uninstall packages from an environment
    ///
    /// Examples:
    ///   zen uninstall numpy              # uninstall from active environment
    ///   zen uninstall -n myenv requests  # uninstall from a specific environment
    Uninstall {
        /// Packages to uninstall
        packages: Vec<String>,
        /// Environment name (uses active environment if omitted)
        #[arg(short = 'n', long = "name")]
        env: Option<String>,
    },
    /// Managed templates
    Template {
        #[command(subcommand)]
        subcommand: TemplateCommands,
    },
    /// Show details of an environment
    #[command(visible_alias = "show")]
    Info {
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        name: Option<String>,
    },
    /// Show system status and active environment
    Status,
    /// Manage project-environment links
    #[command(visible_alias = "init")]
    Link {
        #[command(subcommand)]
        subcommand: LinkCommands,
    },
    /// Export the environment registry and templates to a portable JSON file
    #[command(hide = true)]
    Export {
        /// File to export to
        #[arg(short, long, default_value = "zen_registry.json")]
        file: PathBuf,
    },
    /// Generate shell completion scripts
    #[command(hide = true)]
    Completions {
        /// The shell to generate the script for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Activate an environment (internal use for za hook)
    #[command(hide = true)]
    Activate {
        /// Name of the environment (optional — if omitted, shows selection menu)
        name: Option<String>,
        /// Only output the path (for shell hooks)
        #[arg(long)]
        path_only: bool,
        /// Re-activate the most recently used environment
        #[arg(long)]
        last: bool,
    },
    /// Generate shell hooks for stateless activation
    #[command(hide = true)]
    Hook {
        /// Shell type (bash, zsh, fish)
        #[arg(default_value = "zsh")]
        shell: String,
    },
    /// Clone an existing environment (fast copy) - temporarily disabled
    #[command(hide = true)]
    Clone {
        /// Source environment to clone from
        source: String,
        /// Name for the new environment
        name: String,
    },
    /// Import an environment registry and templates from a JSON file
    #[command(hide = true)]
    Import {
        /// The JSON file to import from
        file: PathBuf,
    },
    /// Interactive setup wizards for Zen
    Setup {
        #[command(subcommand)]
        subcommand: SetupCommands,
    },
    /// Get or set configuration values (stack_info, env_home, etc.)
    Config {
        /// Configuration key to read or write (omit to list all)
        key: Option<String>,
        /// New value to set (requires key)
        value: Option<String>,
    },
    /// Reset database and config to fresh state (preserves environments on disk)
    Reset {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Add, list, or remove notes on an environment
    Note {
        #[command(subcommand)]
        subcommand: NoteCommands,
    },
    /// Legacy alias for 'note'
    #[command(hide = true)]
    Comment {
        #[command(subcommand)]
        subcommand: NoteCommands,
    },
    /// Manage environment labels (add, rm, list)
    Label {
        #[command(subcommand)]
        subcommand: LabelCommands,
    },
    /// Find a package across all environments (substring match by default)
    Find {
        /// Package name or pattern to search for
        package: String,
        /// Exact name match only (default is substring/contains)
        #[arg(long, short)]
        exact: bool,
    },
    /// Inspect a specific package in an environment (like pip show)
    Inspect {
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        env: Option<String>,
        /// Package name to inspect (omit to list all packages)
        package: Option<String>,
        /// One name per line (no versions)
        #[arg(short = '1')]
        names_only: bool,
        /// Long format: one package per line with version
        #[arg(short = 'l')]
        long: bool,
    },
    /// Compare packages between two environments
    Diff {
        /// First environment
        env1: String,
        /// Second environment
        env2: String,
        /// Only show differences (default shows all)
        #[arg(short = 'd', long)]
        only_diff: bool,
    },
    /// Check environment health: Python binary, CUDA consistency, dependency conflicts
    Health {
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        name: Option<String>,
    },
    /// View the activity log (recent operations)
    #[command(alias = "logs")]
    Log {
        /// Filter log entries by keyword (env name, action, etc.)
        filter: Option<String>,
        /// Number of lines to show (default: 25)
        #[arg(short = 'n', long, default_value = "25")]
        lines: usize,
        /// Clear the entire log
        #[arg(long)]
        clear: bool,
    },
    /// Start the Model Context Protocol (MCP) server
    #[command(hide = true)]
    Mcp,
}

#[derive(Subcommand, Clone, Debug)]
enum SetupCommands {
    /// Import existing environments from a directory
    Init {
        /// Path to scan
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Automatic yes to prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Configure tracked packages for stack view
    StackInfo,
}

#[derive(Subcommand, Clone, Debug)]
enum LinkCommands {
    /// Link an environment to a project directory
    ///
    /// Examples:
    ///   zen link add ml_env                    # link ml_env to current directory
    ///   zen link add ml_env /path/to/project   # link ml_env to a specific directory
    ///   zen link add                           # link active env to current directory
    Add {
        /// Name of the environment to link (inferred from $VIRTUAL_ENV if omitted)
        name: Option<String>,
        /// Project directory to link (default: current directory)
        path: Option<String>,
    },
    /// Unlink an environment from a project directory
    ///
    /// Examples:
    ///   zen link rm ml_env                     # unlink from current directory
    ///   zen link rm ml_env /path/to/project    # unlink from a specific directory
    Rm {
        /// Name of the environment to unlink (inferred from $VIRTUAL_ENV if omitted)
        name: Option<String>,
        /// Project directory to unlink from (default: current directory)
        path: Option<String>,
    },
    /// Show environments linked to a project directory (default: current dir)
    List {
        /// Project directory to list links for (default: current directory)
        path: Option<String>,
    },
    /// Remove stale links (deleted envs or missing project dirs)
    Prune,
    /// Clear activation history, remove auto-created links, or wipe all links for a path
    ///
    /// Examples:
    ///   zen link reset --path                # remove ALL links for current directory
    ///   zen link reset --path /some/dir      # remove ALL links for a specific directory
    ///   zen link reset --activations         # remove only auto-created links
    ///   zen link reset --history             # clear counts/timestamps, keep links
    Reset {
        /// Remove ALL links for a project path (default: current directory)
        #[arg(long, num_args = 0..=1, default_missing_value = ".")]
        path: Option<String>,
        /// Only remove auto-created links (from activation, not explicit zen link)
        #[arg(long)]
        activations: bool,
        /// Only clear counts and timestamps (keep all links)
        #[arg(long)]
        history: bool,
        /// Only affect entries older than N days
        #[arg(long, value_name = "DAYS")]
        older_than: Option<u32>,
    },
}

#[derive(Subcommand, Clone, Debug)]
enum LabelCommands {
    /// Add a label to an environment
    Add {
        /// Label to add (e.g., dev, testing, ml, debug)
        label: String,
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        env: Option<String>,
    },
    /// Remove a label from an environment
    Rm {
        /// Label to remove
        label: String,
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        env: Option<String>,
    },
    /// List labels for an environment (or all with --all)
    List {
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        env: Option<String>,
        /// Show labels for all environments
        #[arg(short, long)]
        all: bool,
    },
}

#[derive(Subcommand, Clone, Debug)]
enum NoteCommands {
    /// Add a note to an environment
    Add {
        /// The note text
        message: String,
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        env: Option<String>,
    },
    /// List notes for an environment (or all with --all)
    List {
        /// Name of the environment (inferred from $VIRTUAL_ENV if omitted)
        env: Option<String>,
        /// Show all notes across all environments
        #[arg(short, long)]
        all: bool,
    },
    /// Remove a note by its UUID (or prefix)
    Rm {
        /// The UUID (or prefix) of the note to remove
        uuid: String,
    },
}

#[derive(Subcommand, Clone, Debug)]
enum TemplateCommands {
    /// Start recording a new template session
    Create {
        /// Name of the template (e.g., torch:2.10)
        name: String,
        /// Python version
        #[arg(long)]
        python: Option<String>,
    },
    /// Save the current recording session
    Save,
    /// Abort the current recording session
    Exit,
    /// List all templates
    List,
    /// Remove a template
    Rm { name: String },
    /// Update unpinned dependencies for a template
    Update { name: String },
}

/// Displays the branded landing screen when `zen` is invoked without a subcommand.
///
/// Shows the 禅 icon, version, live status (environment count, active environment,
/// terminal width), and commands organized into five groups by usage frequency.
fn print_landing_screen(db: &Database, _home: &Path) {
    use terminal_size::{Width, terminal_size};

    let full_version = env!("ZEN_VERSION");

    // Header with kanji icon and version
    eprintln!();
    eprintln!(
        "  {}  {}",
        "禅".bold(),
        format!("zen v{}", full_version).dimmed()
    );
    eprintln!("  {}", "Peace of mind for Python environments".dimmed());
    eprintln!();

    // Live status: environment count, active virtualenv, and detected list format
    let env_count = db.list_envs().map(|e| e.len()).unwrap_or(0);
    let active_env = std::env::var("VIRTUAL_ENV").ok().map(|p| {
        std::path::Path::new(&p)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or(p)
    });
    let _term_cols = terminal_size().map(|(Width(w), _)| w).unwrap_or(80);

    eprintln!("  {} {} environments managed", "●".green(), env_count);
    if let Some(ref env_name) = active_env {
        eprintln!("  {} Active: {}", "●".cyan(), env_name.bold());
    } else {
        eprintln!("  {} No active environment", "○".dimmed());
    }
    eprintln!();

    // Command groups ordered by usage frequency
    eprintln!("  {}", "Getting Started".bold().underline());
    eprintln!(
        "    {}       {}",
        "setup init".cyan(),
        "Import existing environments".dimmed()
    );
    eprintln!(
        "    {}  {}",
        "setup stack-info".cyan(),
        "Configure tracked packages".dimmed()
    );
    eprintln!();

    eprintln!("  {}", "Core Commands".bold().underline());
    eprintln!(
        "    {}        {}",
        "activate".cyan(),
        "Activate an environment".dimmed()
    );
    eprintln!(
        "    {}        {}",
        "deactivate".cyan(),
        "Deactivate current environment".dimmed()
    );
    eprintln!(
        "    {}            {}",
        "list".cyan(),
        "List all environments".dimmed()
    );
    eprintln!(
        "    {}          {}",
        "create".cyan(),
        "Create a new environment".dimmed()
    );
    eprintln!(
        "    {}          {}",
        "status".cyan(),
        "Show system status".dimmed()
    );
    eprintln!();

    eprintln!("  {}", "Environment Tools".bold().underline());
    eprintln!(
        "    {}            {}",
        "info".cyan(),
        "Show environment details".dimmed()
    );
    eprintln!(
        "    {}         {}",
        "install".cyan(),
        "Add packages to an environment".dimmed()
    );
    eprintln!(
        "    {}            {}",
        "find".cyan(),
        "Find a package across environments".dimmed()
    );
    eprintln!(
        "    {}         {}",
        "inspect".cyan(),
        "Inspect a package in an environment".dimmed()
    );
    eprintln!(
        "    {}            {}",
        "diff".cyan(),
        "Compare two environments".dimmed()
    );
    eprintln!(
        "    {}          {}",
        "health".cyan(),
        "Check environment health".dimmed()
    );
    eprintln!();

    eprintln!("  {}", "Project & Organization".bold().underline());
    eprintln!(
        "    {}        {}",
        "link add".cyan(),
        "Link environment to project".dimmed()
    );
    eprintln!(
        "    {}       {}",
        "label add".cyan(),
        "Tag environments with labels".dimmed()
    );
    eprintln!(
        "    {}         {}",
        "note".cyan(),
        "Add notes to environments".dimmed()
    );
    eprintln!();

    eprintln!("  {}", "Configuration".bold().underline());
    eprintln!(
        "    {}          {}",
        "config".cyan(),
        "Get or set configuration".dimmed()
    );
    eprintln!(
        "    {}        {}",
        "template".cyan(),
        "Manage environment templates".dimmed()
    );
    eprintln!();

    eprintln!(
        "  {} {} for detailed usage",
        "Run".dimmed(),
        "zen <command> --help".cyan()
    );
    eprintln!();
}

/// Entry point for the Zen CLI.
/// Formats and prints a single link entry with activation metadata.
fn print_link_entry(
    env_name: &str,
    env_path: &str,
    tag: &Option<String>,
    is_default: bool,
    link_type: &str,
    count: i64,
    last_at: &Option<String>,
) {
    let default_marker = if is_default {
        " [default]".green().to_string()
    } else {
        String::new()
    };
    let tag_str = tag
        .as_ref()
        .map(|t| format!(" ({})", t))
        .unwrap_or_default();
    let type_icon = if link_type == "user" { " ★" } else { "" };
    let stats = if count > 0 {
        let last_str = last_at
            .as_ref()
            .map(|t| format!(", last: {}", &t[..10]))
            .unwrap_or_default();
        format!(" [{}x{}]", count, last_str)
    } else {
        String::new()
    };
    println!(
        "  • {}{}{}{} → {}{}",
        env_name.bold(),
        type_icon,
        tag_str,
        default_marker,
        env_path.dimmed(),
        stats.dimmed()
    );
}

/// Resolves an environment name from an optional argument or `$VIRTUAL_ENV`.
///
/// Used by commands that support auto-detection: info, inspect, health,
/// label add/rm/list, link add/rm.
fn resolve_env_name(
    name: Option<String>,
    db: &Database,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(n) = name {
        return Ok(n);
    }
    // Try $VIRTUAL_ENV
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        let venv_path = std::path::Path::new(&venv);
        let envs = db.list_envs()?;
        // Match by path
        if let Some((name, ..)) = envs
            .iter()
            .find(|(_, p, ..)| std::path::Path::new(p) == venv_path)
        {
            return Ok(name.clone());
        }
        // Fall back to directory basename
        if let Some(basename) = venv_path.file_name() {
            return Ok(basename.to_string_lossy().to_string());
        }
    }
    Err(
        "No environment specified. Activate one with 'za <env>' or pass its name as an argument."
            .into(),
    )
}

///
/// Parses arguments via clap, opens the SQLite registry, and dispatches to the
/// appropriate command handler. Displays the branded landing screen when no
/// subcommand is provided.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cli = Cli::parse();

    // Restore terminal cursor on Ctrl+C.
    // dialoguer hides the cursor during prompts; SIGINT without cleanup
    // leaves the terminal with an invisible cursor.
    ctrlc::set_handler(move || {
        // Show cursor: ESC [ ? 25 h
        eprint!("\x1B[?25h");
        std::process::exit(130);
    })
    .ok();

    // Expand ~ to $HOME since PathBuf doesn't handle tilde
    if cli.home.starts_with("~")
        && let Ok(home) = std::env::var("HOME")
    {
        cli.home = PathBuf::from(cli.home.to_string_lossy().replacen('~', &home, 1));
    }

    let db = Database::open(cli.db_path.as_deref())?;

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            print_landing_screen(&db, &cli.home);
            return Ok(());
        }
    };

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let ops = crate::ops::ZenOps::new(&db, cli.home.clone());
        match command {
            Commands::Create {
                name,
                python: user_python,
                template,
                strict,
                ml,
                cuda,
                rm,
            } => {
                // Validate inputs
                crate::validation::validate_name(&name, "Environment")?;
                if let Some(ref py) = user_python {
                    crate::validation::validate_python_version(py)?;
                }
                if let Some(ref cuda_ver) = cuda {
                    crate::validation::validate_cuda_version(cuda_ver)?;
                }

                let mut python = user_python.clone().unwrap_or_else(|| "3.12".to_string());
                let env_path = cli.home.join(&name);

                // Guard: check if environment already exists
                let existing = db.list_envs()?;
                if existing.iter().any(|(n, ..)| n == &name) {
                    if rm {
                        // Auto-remove before re-creating
                        println!("Removing existing environment '{}'...", name.dimmed());
                        let env_name = types::EnvName::new(&name).map_err(|e| e.to_string())?;
                        if let Err(e) = ops.remove_env(&env_name) {
                            eprintln!("{} {}", "Error:".red(), e);
                            return Ok(());
                        }
                    } else {
                        eprintln!(
                            "{} Environment '{}' already exists. Use {} or {} to replace it.",
                            "Error:".red(),
                            name,
                            "zen rm".bold(),
                            "--rm".bold()
                        );
                        return Ok(());
                    }
                }
                if env_path.exists() && !rm {
                    eprintln!(
                        "{} Directory '{}' already exists. Remove it or choose a different name.",
                        "Error:".red(),
                        env_path.display()
                    );
                    return Ok(());
                } else if env_path.exists() && rm {
                    std::fs::remove_dir_all(&env_path)?;
                }

                // Validate templates before starting creation
                let mut templates_to_apply = Vec::new();
                let mut first_tpl_python: Option<String> = None;
                if let Some(t_str) = template {
                    let parts = utils::parse_template_string(&t_str);
                    for part in parts {
                        if let Some(t_id) = db.get_template_id(&part.name, &part.version)? {
                            // Inherit python version from the FIRST template only
                            if user_python.is_none()
                                && let Ok(all_tpls) = db.list_templates()
                                && let Some(t_info) = all_tpls
                                    .iter()
                                    .find(|t| t.0 == part.name && t.1 == part.version)
                            {
                                if first_tpl_python.is_none() {
                                    python = t_info.2.clone();
                                    first_tpl_python = Some(t_info.2.clone());
                                } else if first_tpl_python.as_deref() != Some(&t_info.2) {
                                    eprintln!(
                                        "  {} Template '{}:{}' uses Python {} but first template uses Python {} — using {}",
                                        "⚠".yellow(),
                                        part.name,
                                        part.version,
                                        t_info.2,
                                        first_tpl_python.as_deref().unwrap_or("?"),
                                        first_tpl_python.as_deref().unwrap_or("?")
                                    );
                                }
                            }
                            templates_to_apply.push((t_id, part.name, part.version));
                        } else {
                            eprintln!(
                                "{} Template '{}:{}' not found. Use {} to see available templates.",
                                "Error:".red(),
                                part.name,
                                part.version,
                                "zen template list".bold()
                            );
                            std::process::exit(1);
                        }
                    }
                }

                println!("Creating environment '{}'...", name.cyan());

                std::fs::create_dir_all(&cli.home)?;

                // Ordering: Python -> NumPy -> Torch -> others
                templates_to_apply.sort_by_key(|(_, name, _)| match name.to_lowercase().as_str() {
                    "python" | "py" => 0,
                    "numpy" => 1,
                    "torch" | "pytorch" => 2,
                    _ => 3,
                });

                // If a python template is present, use its version
                for (_, name, _) in &templates_to_apply {
                    if name.to_lowercase() == "python" || name.to_lowercase() == "py" {
                        // Assuming version field in template might be the python version if it's a python template
                        // or storing it in python_version field. Let's check DB.
                        // For now, let's assume 'python:3.10' means version is 3.10
                        // python = part.version;
                    }
                }

                // Try to use uv if available, otherwise fallback to venv
                let status = if let Ok(uv_path) = which::which("uv") {
                    std::process::Command::new(uv_path)
                        .arg("venv")
                        .arg(&env_path)
                        .arg("--python")
                        .arg(&python)
                        .arg("--clear")
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()?
                } else {
                    std::process::Command::new("python3")
                        .arg("-m")
                        .arg("venv")
                        .arg(&env_path)
                        .arg("--clear")
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()?
                };

                if status.success() {
                    let env_str = env_path.to_str().unwrap();

                    // Silent bootstrap — no need to show pip/uv/setuptools install
                    if let Ok(_uv_path) = which::which("uv") {
                        utils::run_in_env_silent(
                            env_str,
                            "uv",
                            &["pip", "install", "uv", "setuptools"],
                        );
                    } else {
                        utils::run_in_env_silent(
                            env_str,
                            "pip",
                            &["install", "--upgrade", "pip", "setuptools"],
                        );
                    }

                    // Save template info for logging before ownership is consumed
                    let tpl_log_info: String = if !templates_to_apply.is_empty() {
                        format!(
                            " --template {}",
                            templates_to_apply
                                .iter()
                                .map(|(_, n, v)| format!("{}:{}", n, v))
                                .collect::<Vec<_>>()
                                .join(",")
                        )
                    } else {
                        String::new()
                    };

                    // Apply templates — track installed packages for conflict detection
                    let mut installed_pkgs: std::collections::HashMap<
                        String,
                        (String, String, Option<String>),
                    > = std::collections::HashMap::new(); // pkg_name -> (version, template_name, install_args)

                    for (t_id, t_name, t_ver) in templates_to_apply {
                        println!("Applying template '{}:{}'...", t_name, t_ver);
                        let packages = db.get_template_packages(t_id)?;

                        // Detect conflicts with previously applied templates
                        for (p_name, p_ver, _, _, pkg_install_args) in &packages {
                            let pkg_lower = p_name.to_lowercase();
                            if let Some((prev_ver, prev_tpl, prev_args)) =
                                installed_pkgs.get(&pkg_lower)
                            {
                                // Check for index URL conflict (e.g. cu128 vs cu130)
                                if prev_args != pkg_install_args {
                                    eprintln!(
                                        "  {} '{}' will be reinstalled from a different index (was in '{}', now in '{}:{}').",
                                        "⚠ Index conflict:".yellow().bold(),
                                        p_name,
                                        prev_tpl,
                                        t_name,
                                        t_ver
                                    );
                                } else if prev_ver != p_ver {
                                    eprintln!(
                                        "  {} '{}' {}→{} (was in '{}', overridden by '{}:{}')",
                                        "⚠ Override:".yellow(),
                                        p_name,
                                        prev_ver.dimmed(),
                                        p_ver.yellow(),
                                        prev_tpl,
                                        t_name,
                                        t_ver
                                    );
                                }
                            }
                        }

                        // Group packages by install_args to handle different index URLs
                        let mut pkg_groups: std::collections::HashMap<Option<String>, Vec<String>> =
                            std::collections::HashMap::new();

                        for (p_name, p_ver, is_pinned, _, pkg_install_args) in packages {
                            let pkg_spec = if strict || is_pinned {
                                format!("{}=={}", p_name, p_ver)
                            } else {
                                p_name.clone()
                            };
                            // Track for conflict detection in subsequent templates
                            installed_pkgs.insert(
                                p_name.to_lowercase(),
                                (
                                    p_ver,
                                    format!("{}:{}", t_name, t_ver),
                                    pkg_install_args.clone(),
                                ),
                            );
                            pkg_groups
                                .entry(pkg_install_args)
                                .or_default()
                                .push(pkg_spec);
                        }

                        // Install each group with its specific args
                        for (group_args, group_pkgs) in pkg_groups {
                            if group_pkgs.is_empty() {
                                continue;
                            }
                            let mut cmd_args = vec!["pip", "install"];

                            // Add any stored pip args (e.g., --index-url)
                            if let Some(ref args_str) = group_args {
                                for arg in args_str.split_whitespace() {
                                    cmd_args.push(arg);
                                }
                            }

                            for pkg in &group_pkgs {
                                cmd_args.push(pkg);
                            }

                            if which::which("uv").is_ok() {
                                utils::run_in_env(env_str, "uv", &cmd_args);
                            } else {
                                utils::run_in_env(env_str, "pip", &cmd_args[1..]);
                            }
                        }
                    }

                    let py_ver =
                        utils::read_python_version(env_path.to_str().unwrap()).unwrap_or(python);

                    let _env_id = db.register_env(&name, env_path.to_str().unwrap(), &py_ver)?;

                    // Package versions are now tracked dynamically via `zen list --refresh`

                    println!(
                        "{} Environment '{}' created. (Python {})",
                        "✓".green(),
                        name.cyan(),
                        py_ver.dimmed()
                    );
                    println!(
                        "  Activate: {} ({})",
                        format!("zen activate {}", name).bold(),
                        format!("za {}", name).dimmed()
                    );
                    activity_log::log_activity(
                        "cli",
                        "create",
                        &format!("{} (Python {}){}", name, py_ver, tpl_log_info),
                    );

                    // Install ML stack if requested
                    if ml {
                        let cuda_ver = cuda.unwrap_or_else(|| "12.6".to_string());
                        println!(
                            "\n{}",
                            "Installing ML stack (PyTorch + CUDA)...".bold().cyan()
                        );
                        let index_url = format!(
                            "https://download.pytorch.org/whl/cu{}",
                            cuda_ver.replace('.', "")
                        );
                        println!("  Using CUDA {} index: {}", cuda_ver, index_url);

                        let pip_path = env_path.join("bin").join("pip");
                        let result = std::process::Command::new(&pip_path)
                            .args([
                                "install",
                                "torch",
                                "torchvision",
                                "torchaudio",
                                "--index-url",
                                &index_url,
                            ])
                            .status();

                        match result {
                            Ok(status) if status.success() => {
                                println!("{} ML stack installed successfully.", "✓".green());
                            }
                            _ => {
                                eprintln!("{} ML stack installation failed.", "✗".red());
                            }
                        }
                    }
                } else {
                    eprintln!("Failed to create environment.");
                }
            }
            Commands::List {
                pattern,
                sort,
                num,
                label,
                format,
            } => {
                // Auto-discover new environments (silent, fast)
                let home_path = &cli.home;
                if home_path.exists()
                    && let Ok(entries) = std::fs::read_dir(home_path)
                {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let python_bin = path.join("bin/python");
                        let python3_bin = path.join("bin/python3");
                        if path.is_dir() && (python_bin.exists() || python3_bin.exists()) {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if db.get_env_id(&name)?.is_none() {
                                let path_str = path.to_string_lossy().to_string();
                                let py_ver = utils::read_python_version(&path_str)
                                    .unwrap_or_else(|| "unknown".to_string());
                                db.register_env(&name, &path_str, &py_ver)?;
                            }
                        }
                    }
                }

                // Get envs, optionally filtered by label
                let envs = if let Some(ref label_filter) = label {
                    let label_envs = db.get_envs_by_label(label_filter)?;
                    ops.list_envs_with_status(pattern.as_deref(), Some(&sort), num)?
                        .into_iter()
                        .filter(|(name, ..)| label_envs.contains(name))
                        .collect()
                } else {
                    ops.list_envs_with_status(pattern.as_deref(), Some(&sort), num)?
                };

                let stack_info_config = db
                    .get_config("stack_info")?
                    .unwrap_or_else(|| "torch numpy transformers diffusers".to_string());
                let tracked_keys: Vec<&str> = stack_info_config.split_whitespace().collect();

                // Determine format based on terminal width or explicit flag
                #[derive(Debug, PartialEq)]
                enum ListFormat {
                    Minimal,
                    Compact,
                    Wide,
                }

                let list_format = match format.as_str() {
                    "minimal" | "min" | "m" => ListFormat::Minimal,
                    "compact" | "c" => ListFormat::Compact,
                    "wide" | "w" => ListFormat::Wide,
                    _ => {
                        // Auto-detect based on terminal width
                        use terminal_size::{Width, terminal_size};
                        match terminal_size() {
                            Some((Width(w), _)) if w < 60 => ListFormat::Minimal,
                            Some((Width(w), _)) if w < 200 => ListFormat::Minimal,
                            Some(_) => ListFormat::Compact,
                            None => ListFormat::Minimal, // Safe default for pipes/non-TTY
                        }
                    }
                };

                // Pre-scan all environments for package versions + health
                let env_data: Vec<_> = envs
                    .iter()
                    .map(|(name, path, py_ver, exists, _updated, is_fav)| {
                        let packages = crate::utils::get_packages(path);
                        let versions: std::collections::HashMap<String, Option<String>> =
                            packages.into_iter().map(|p| (p.name, p.version)).collect();
                        // Real health check (native, no subprocess)
                        let health = if *exists {
                            crate::ops::check_health_quick(std::path::Path::new(path))
                        } else {
                            crate::types::HealthLevel::Fail
                        };
                        (
                            name.clone(),
                            path.clone(),
                            py_ver.clone(),
                            *exists,
                            *is_fav,
                            versions,
                            health,
                        )
                    })
                    .collect();

                match list_format {
                    ListFormat::Minimal => {
                        // Pre-calculate all column widths
                        let max_name = env_data
                            .iter()
                            .map(|(name, _, _, _, is_fav, _, _)| {
                                let icon_w = if *is_fav { 2 } else { 0 };
                                name.len() + icon_w
                            })
                            .max()
                            .unwrap_or(12);

                        let max_pyver = env_data
                            .iter()
                            .map(|(_, _, py_ver, _, _, _, _)| py_ver.len())
                            .max()
                            .unwrap_or(4);

                        // Pre-calculate max width per tracked package column
                        let tracked_display: Vec<&str> =
                            tracked_keys.iter().take(2).copied().collect();
                        let mut max_col_widths: Vec<usize> =
                            tracked_display.iter().map(|k| k.len()).collect();
                        for (_, _, _, _, _, versions, _) in &env_data {
                            for (i, key) in tracked_display.iter().enumerate() {
                                if let Some(Some(v)) = versions.get(*key) {
                                    // key:version — plain width
                                    let w = key.len() + 1 + v.len();
                                    if w > max_col_widths[i] {
                                        max_col_widths[i] = w;
                                    }
                                }
                            }
                        }

                        for (name, _path, py_ver, _exists, is_fav, versions, health) in &env_data {
                            let name_display = if *is_fav {
                                format!("★ {}", name)
                            } else {
                                format!("  {}", name)
                            };
                            // Health status — zen aesthetics
                            let status_str = match health {
                                crate::types::HealthLevel::Pass => {
                                    format!(" {}", "✓".truecolor(100, 200, 255))
                                }
                                crate::types::HealthLevel::Info => {
                                    format!(" {}", "△".truecolor(255, 182, 193))
                                }
                                crate::types::HealthLevel::Warn => {
                                    format!(" {}", "!".truecolor(255, 140, 0))
                                }
                                crate::types::HealthLevel::Fail => {
                                    format!(" {}", "✗".red())
                                }
                            };

                            // Build stack columns with pre-calculated widths
                            let mut stack_str = String::new();
                            for (i, key) in tracked_display.iter().enumerate() {
                                if let Some(Some(v)) = versions.get(*key) {
                                    let colored_v = if *key == "torch" && v.contains("+cu") {
                                        v.green().to_string()
                                    } else if *key == "numpy" {
                                        if v.starts_with('2') || v.starts_with('3') {
                                            v.truecolor(100, 200, 255).to_string()
                                        } else {
                                            v.truecolor(255, 140, 0).to_string()
                                        }
                                    } else {
                                        v.to_string()
                                    };
                                    let plain = format!("{}:{}", key, v);
                                    let colored = format!("{}:{}", key.dimmed(), colored_v);
                                    let pad = max_col_widths[i].saturating_sub(plain.len());
                                    stack_str.push_str(&format!(
                                        "  {}{}",
                                        colored,
                                        " ".repeat(pad)
                                    ));
                                } else {
                                    // Blank column, maintain alignment
                                    stack_str
                                        .push_str(&format!("  {}", " ".repeat(max_col_widths[i])));
                                }
                            }

                            println!(
                                "{:<name_w$} {:<py_w$}{}{}",
                                name_display,
                                py_ver.dimmed(),
                                status_str,
                                stack_str,
                                name_w = max_name + 2,
                                py_w = max_pyver,
                            );
                        }
                    }
                    ListFormat::Compact => {
                        // Medium format: no path, key packages inline
                        use comfy_table::modifiers::UTF8_ROUND_CORNERS;
                        use comfy_table::presets::UTF8_FULL;
                        use comfy_table::{Cell, Color, ContentArrangement, Table};

                        let mut table = Table::new();
                        table
                            .load_preset(UTF8_FULL)
                            .apply_modifier(UTF8_ROUND_CORNERS)
                            .set_content_arrangement(ContentArrangement::Dynamic);

                        let header_style = comfy_table::Attribute::Bold;
                        let mut header_row = vec![
                            Cell::new("Name").add_attribute(header_style),
                            Cell::new("Py").add_attribute(header_style),
                            Cell::new("Health").add_attribute(header_style),
                        ];

                        // Only show first 2 tracked packages in compact mode
                        for key in tracked_keys.iter().take(2) {
                            header_row.push(
                                Cell::new(*key)
                                    .add_attribute(header_style)
                                    .set_alignment(comfy_table::CellAlignment::Center),
                            );
                        }
                        table.set_header(header_row);

                        for (name, _path, py_ver, _exists, is_fav, versions, health) in &env_data {
                            let name_display = if *is_fav {
                                format!("★ {}", name)
                            } else {
                                name.clone()
                            };

                            let health_cell = match health {
                                crate::types::HealthLevel::Pass => Cell::new("✓").fg(Color::Rgb {
                                    r: 100,
                                    g: 200,
                                    b: 255,
                                }),
                                crate::types::HealthLevel::Info => Cell::new("△").fg(Color::Rgb {
                                    r: 255,
                                    g: 182,
                                    b: 193,
                                }),
                                crate::types::HealthLevel::Warn => Cell::new("!").fg(Color::Red),
                                crate::types::HealthLevel::Fail => Cell::new("✗").fg(Color::Red),
                            };

                            let mut row = vec![
                                if *is_fav {
                                    Cell::new(&name_display).fg(Color::Yellow)
                                } else {
                                    Cell::new(&name_display)
                                },
                                Cell::new(py_ver),
                                health_cell,
                            ];

                            for key in tracked_keys.iter().take(2) {
                                let version = versions.get(*key).and_then(|v| v.clone());
                                let cell = match version {
                                    Some(v) => {
                                        if *key == "torch" && v.contains("+cu") {
                                            Cell::new(&v).fg(Color::Green)
                                        } else if *key == "numpy" && v.starts_with('2') {
                                            Cell::new(&v).fg(Color::Cyan)
                                        } else {
                                            Cell::new(&v)
                                        }
                                    }
                                    None => Cell::new("--"),
                                };
                                row.push(cell.set_alignment(comfy_table::CellAlignment::Left));
                            }
                            table.add_row(row);
                        }
                        println!("{}", table);
                    }
                    ListFormat::Wide => {
                        // Full table with paths and all tracked packages
                        use comfy_table::modifiers::UTF8_ROUND_CORNERS;
                        use comfy_table::presets::UTF8_FULL;
                        use comfy_table::{Cell, Color, ContentArrangement, Table};

                        let mut table = Table::new();
                        table
                            .load_preset(UTF8_FULL)
                            .apply_modifier(UTF8_ROUND_CORNERS)
                            .set_content_arrangement(ContentArrangement::Disabled);

                        let header_style = comfy_table::Attribute::Bold;
                        let mut header_row = vec![
                            Cell::new("Name").add_attribute(header_style),
                            Cell::new("Python").add_attribute(header_style),
                            Cell::new("Health").add_attribute(header_style),
                        ];
                        header_row.push(Cell::new("Path").add_attribute(header_style));

                        for key in &tracked_keys {
                            header_row.push(
                                Cell::new(*key)
                                    .add_attribute(header_style)
                                    .set_alignment(comfy_table::CellAlignment::Center),
                            );
                        }
                        table.set_header(header_row);

                        for (name, path, py_ver, _exists, is_fav, versions, health) in &env_data {
                            let name_display = if *is_fav {
                                format!("★ {}", name)
                            } else {
                                name.clone()
                            };

                            let health_cell = match health {
                                crate::types::HealthLevel::Pass => Cell::new("✓").fg(Color::Rgb {
                                    r: 100,
                                    g: 200,
                                    b: 255,
                                }),
                                crate::types::HealthLevel::Info => Cell::new("△").fg(Color::Rgb {
                                    r: 255,
                                    g: 182,
                                    b: 193,
                                }),
                                crate::types::HealthLevel::Warn => Cell::new("!").fg(Color::Red),
                                crate::types::HealthLevel::Fail => Cell::new("✗").fg(Color::Red),
                            };

                            let mut row = vec![
                                if *is_fav {
                                    Cell::new(&name_display).fg(Color::Yellow)
                                } else {
                                    Cell::new(&name_display)
                                },
                                Cell::new(py_ver),
                                health_cell,
                            ];
                            row.push(Cell::new(path).fg(Color::DarkGrey));

                            for key in &tracked_keys {
                                let version = versions.get(*key).and_then(|v| v.clone());
                                let cell = match version {
                                    Some(v) => {
                                        if *key == "torch" && v.contains("+cu") {
                                            Cell::new(&v).fg(Color::Green)
                                        } else if *key == "numpy" {
                                            if v.starts_with('2') {
                                                Cell::new(&v).fg(Color::Cyan)
                                            } else {
                                                Cell::new(&v).fg(Color::Red)
                                            }
                                        } else {
                                            Cell::new(&v)
                                        }
                                    }
                                    None => Cell::new("--"),
                                };
                                row.push(cell.set_alignment(comfy_table::CellAlignment::Left));
                            }
                            table.add_row(row);
                        }
                        println!("{}", table);
                    }
                }

                // Legend footer with health counts
                let total = env_data.len();
                let n_fav = env_data
                    .iter()
                    .filter(|(_, _, _, _, fav, _, _)| *fav)
                    .count();
                let n_pass = env_data
                    .iter()
                    .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Pass)
                    .count();
                let n_info = env_data
                    .iter()
                    .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Info)
                    .count();
                let n_warn = env_data
                    .iter()
                    .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Warn)
                    .count();
                let n_fail = env_data
                    .iter()
                    .filter(|(_, _, _, _, _, _, h)| *h == crate::types::HealthLevel::Fail)
                    .count();

                print!("{}", format!("{} environments", total).dimmed());
                if n_pass > 0 {
                    print!(
                        "  {} {}",
                        "✓".truecolor(100, 200, 255),
                        format!("{} ok", n_pass).dimmed()
                    );
                }
                if n_info > 0 {
                    print!(
                        "  {} {}",
                        "△".truecolor(255, 182, 193),
                        format!("{} minor", n_info).dimmed()
                    );
                }
                if n_warn > 0 {
                    print!(
                        "  {} {}",
                        "!".truecolor(255, 140, 0),
                        format!("{} drift", n_warn).dimmed()
                    );
                }
                if n_fail > 0 {
                    print!("  {} {}", "✗".red(), format!("{} broken", n_fail).dimmed());
                }
                if n_fav > 0 {
                    print!(
                        "  {} {}",
                        "★".truecolor(255, 215, 0),
                        format!("{} fav", n_fav).dimmed()
                    );
                }
                println!();
            }
            Commands::Rm { name, yes } => {
                let env_name = types::EnvName::new(&name).map_err(|e| e.to_string())?;
                // Check existence before prompting
                let envs = db.list_envs()?;
                let in_db = envs.iter().any(|(n, ..)| n == &name);
                let on_disk = cli.home.join(&name).exists();
                if !in_db && !on_disk {
                    activity_log::log_activity("cli", "rm:error", &format!("{} - not found", name));
                    eprintln!("{} Environment '{}' not found.", "Error:".red(), name);
                    return Ok(());
                }
                if !yes {
                    use dialoguer::{Confirm, theme::ColorfulTheme};
                    let confirmed = match Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt(format!(
                            "Are you sure you want to remove environment '{}'?",
                            name
                        ))
                        .default(false)
                        .interact()
                    {
                        Ok(v) => v,
                        Err(_) => {
                            // Ctrl+C — exit silently
                            println!();
                            return Ok(());
                        }
                    };
                    if !confirmed {
                        println!("Abort.");
                        return Ok(());
                    }
                }
                println!("{} {}...", "Removing".magenta().bold(), name);
                activity_log::log_activity("cli", "rm", &name);
                match ops.remove_env(&env_name) {
                    Ok(resp) => println!("{}", resp),
                    Err(e) => {
                        activity_log::log_activity("cli", "rm:error", &format!("{} - {}", name, e));
                        eprintln!("{} {}", "Error:".red(), e);
                        return Ok(());
                    }
                }
            }
            Commands::Config { key, value } => match (key, value) {
                (Some(k), Some(v)) => {
                    db.set_config(&k, &v)?;
                    activity_log::log_activity("cli", "config", &format!("{} = {}", k, v));
                    println!("{} Config updated: {} = {}", "✓".green(), k, v);
                }
                (Some(k), None) => {
                    let v = db.get_config(&k)?.unwrap_or_else(|| "not set".to_string());
                    println!("{} = {}", k, v);
                }
                (None, _) => {
                    let configs = db.list_all_config()?;
                    if configs.is_empty() {
                        println!("No configuration values set.");
                    } else {
                        println!("{}:", "Configuration".cyan());
                        for (k, v) in configs {
                            println!("  {} = {}", k.bold(), v);
                        }
                    }
                }
            },
            Commands::Reset { yes } => {
                use std::io::Write;

                if !yes {
                    print!(
                        "{} This will delete the zen database and config. Environments on disk will NOT be affected.\nContinue? [y/N] ",
                        "⚠".yellow()
                    );
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Aborted.");
                        return Ok(());
                    }
                }

                let home = home::home_dir().ok_or("Could not find home directory")?;
                let db_path = home.join(".zen").join("zen.db");
                let config_path = home.join(".zen").join("config.toml");

                if db_path.exists() {
                    std::fs::remove_file(&db_path)?;
                    println!("{} Removed {}", "✓".green(), db_path.display());
                }
                if config_path.exists() {
                    std::fs::remove_file(&config_path)?;
                    println!("{} Removed {}", "✓".green(), config_path.display());
                }

                println!(
                    "\n{} Database reset. Run {} to rediscover environments.",
                    "✓".green(),
                    "zen scan".cyan()
                );
            }
            Commands::Template { subcommand } => {
                match subcommand {
                    TemplateCommands::Create {
                        name,
                        python: user_python,
                    } => {
                        // Validate inputs
                        crate::validation::validate_name(
                            name.split(':').next().unwrap_or(&name),
                            "Template",
                        )?;
                        if let Some(ref py) = user_python {
                            crate::validation::validate_python_version(py)?;
                        }

                        let python = user_python.unwrap_or_else(|| "3.12".to_string());
                        if db.get_active_session()?.is_some() {
                            eprintln!(
                                "A recording session is already active. Please save or exit first."
                            );
                            return Ok(());
                        }

                        let mut parts = name.splitn(2, ':');
                        let t_name = parts.next().unwrap();
                        let t_ver = parts.next().unwrap_or("latest");

                        let temp_id = db.create_template(t_name, t_ver, &python)?;
                        let tmp_env =
                            std::env::temp_dir().join(format!("zen_tpl_{}_{}", t_name, t_ver));
                        println!(
                            "Creating temporary recording environment at {}...",
                            tmp_env.display()
                        );

                        let status = if let Ok(uv_path) = which::which("uv") {
                            std::process::Command::new(uv_path)
                                .arg("venv")
                                .arg(&tmp_env)
                                .arg("--python")
                                .arg(&python)
                                .arg("--clear")
                                .status()?
                        } else {
                            std::process::Command::new("python3")
                                .arg("-m")
                                .arg("venv")
                                .arg(&tmp_env)
                                .arg("--clear")
                                .status()?
                        };

                        if status.success() {
                            let env_str = tmp_env.to_str().unwrap();
                            if let Ok(_uv_path) = which::which("uv") {
                                utils::run_in_env(
                                    env_str,
                                    "uv",
                                    &["pip", "install", "uv", "setuptools"],
                                );
                            } else {
                                utils::run_in_env(
                                    env_str,
                                    "pip",
                                    &["install", "--upgrade", "pip", "setuptools"],
                                );
                            }
                            db.start_session(temp_id, env_str)?;
                            println!(
                                "{} Recording session started for template '{}'.",
                                "✓".green(),
                                name
                            );
                            println!("  Use {} to add packages.", "zen install <pkg>".cyan());
                            println!("  Use {} to save and exit.", "zen template save".cyan());
                        }
                    }
                    TemplateCommands::Save => {
                        if let Some((t_id, path)) = db.get_active_session()? {
                            // Only session packages (recorded during `zen install`) are stored.
                            // Transitive dependencies are resolved by the solver at apply time,
                            // preventing version churn from index mismatches.
                            let session_pkgs = db.get_template_packages(t_id)?;
                            let count = session_pkgs.len();

                            if count == 0 {
                                eprintln!(
                                    "No packages were installed during this session. Nothing to save."
                                );
                                eprintln!(
                                    "Use {} to add packages first.",
                                    "zen install <pkg>".cyan()
                                );
                                return Ok(());
                            }

                            // Clean up the temp env
                            std::fs::remove_dir_all(&path).ok();
                            db.clear_sessions()?;

                            activity_log::log_activity(
                                "cli",
                                "template:save",
                                &format!("{} pkgs", count),
                            );
                            println!("Template saved successfully ({} packages).", count);
                        } else {
                            eprintln!("No active recording session found.");
                        }
                    }
                    TemplateCommands::Exit => {
                        if let Some((_, path)) = db.get_active_session()? {
                            println!("Aborting session. Cleaning up {}...", path);
                            std::fs::remove_dir_all(path).ok();
                            db.clear_sessions()?;
                            println!("Session exited.");
                        } else {
                            eprintln!("No active recording session found.");
                        }
                    }
                    TemplateCommands::List => {
                        let templates = db.get_all_templates_with_packages()?;
                        use comfy_table::{
                            Attribute, Cell, ContentArrangement, Table,
                            modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED,
                        };
                        let mut table = Table::new();
                        table
                            .load_preset(UTF8_FULL_CONDENSED)
                            .apply_modifier(UTF8_ROUND_CORNERS)
                            .set_content_arrangement(ContentArrangement::Dynamic);

                        table.set_header(vec![
                            Cell::new("Name").add_attribute(Attribute::Bold),
                            Cell::new("Version").add_attribute(Attribute::Bold),
                            Cell::new("Python").add_attribute(Attribute::Bold),
                            Cell::new("Packages").add_attribute(Attribute::Bold),
                        ]);

                        for (n, v, p, pkgs) in templates {
                            table.add_row(vec![n, v, p, pkgs.len().to_string()]);
                        }
                        println!("{}", table);
                    }
                    TemplateCommands::Rm { name } => {
                        if db.delete_template(&name)? {
                            activity_log::log_activity("cli", "template:rm", &name);
                            println!("{} Template '{}' deleted.", "✓".green(), name);
                        } else {
                            println!("{} Template '{}' not found.", "✗".red(), name);
                        }
                    }
                    TemplateCommands::Update { name: _ } => {
                        println!("Template update is not yet implemented.");
                    }
                }
            }
            Commands::Install {
                packages,
                env,
                pinned: _,
                index_url: cli_index_url,
                extra_index_url,
                editable,
                pre,
                upgrade,
                dry_run,
            } => {
                let (target_id, target_path, is_session) =
                    if let Some(session) = db.get_active_session()? {
                        (Some(session.0), session.1, true)
                    } else if let Some(env_name) = env {
                        let envs = db.list_envs()?;
                        let e = envs
                            .iter()
                            .find(|(n, ..)| n == &env_name)
                            .ok_or_else(|| format!("Environment '{}' not found", env_name))?;
                        let id = db.get_env_id(&env_name)?.ok_or_else(|| {
                            format!("Environment '{}' not found in database", env_name)
                        })?;
                        (Some(id), e.1.clone(), false)
                    } else {
                        // Fall back: try to resolve from $VIRTUAL_ENV
                        let resolved = resolve_env_name(None, &db).map_err(
                            |_| "No active environment. Use: zen install -n <env> <packages>",
                        )?;
                        let envs = db.list_envs()?;
                        let e = envs
                            .iter()
                            .find(|(n, ..)| n == &resolved)
                            .ok_or_else(|| format!("Environment '{}' not found", resolved))?;
                        let id = db.get_env_id(&resolved)?.ok_or_else(|| {
                            format!("Environment '{}' not found in database", resolved)
                        })?;
                        (Some(id), e.1.clone(), false)
                    };

                println!("Installing packages in {}...", target_path);

                let mut final_args = Vec::new();
                let mut index_url = cli_index_url.clone();

                for pkg in &packages {
                    if pkg.starts_with("torch-cu") {
                        let cuda_ver = pkg.trim_start_matches("torch-cu");
                        // Map common aliases (e.g. 130 -> 13.0)
                        let normalized_cuda = if cuda_ver.len() == 3 {
                            format!("{}.{}", &cuda_ver[0..2], &cuda_ver[2..])
                        } else {
                            cuda_ver.to_string()
                        };

                        if let Some(url) = utils::get_torch_index_url(&normalized_cuda) {
                            index_url = Some(url.to_string());
                            final_args.push("torch".to_string());
                            final_args.push("torchvision".to_string());
                            final_args.push("torchaudio".to_string());
                        } else {
                            final_args.push(pkg.clone());
                        }
                    } else {
                        final_args.push(pkg.clone());
                    }
                }

                let mut cmd_args = vec!["pip", "install"];

                // Add pip-compatible flags
                if editable {
                    cmd_args.push("-e");
                }
                if pre {
                    cmd_args.push("--pre");
                }
                if upgrade {
                    cmd_args.push("--upgrade");
                }
                if dry_run {
                    cmd_args.push("--dry-run");
                }
                if let Some(ref url) = index_url {
                    cmd_args.push("--index-url");
                    cmd_args.push(url);
                }
                if let Some(ref url) = extra_index_url {
                    cmd_args.push("--extra-index-url");
                    cmd_args.push(url);
                }

                for pkg in &final_args {
                    cmd_args.push(pkg);
                }

                let success = if which::which("uv").is_ok() {
                    utils::run_in_env(&target_path, "uv", &cmd_args)
                } else {
                    utils::run_in_env(&target_path, "pip", &cmd_args[1..])
                };

                if success {
                    if is_session {
                        let t_id = target_id.ok_or("Missing template ID for session")?;
                        let installed = utils::get_packages(&target_path);

                        // Capture install_args (e.g., --index-url, --extra-index-url)
                        // to preserve CUDA version and custom indices
                        let install_args_str: Option<String> = {
                            let mut parts = Vec::new();
                            if let Some(ref url) = index_url {
                                parts.push(format!("--index-url {}", url));
                            }
                            if let Some(ref url) = extra_index_url {
                                parts.push(format!("--extra-index-url {}", url));
                            }
                            if parts.is_empty() {
                                None
                            } else {
                                Some(parts.join(" "))
                            }
                        };

                        for pkg_name in &packages {
                            let base_name = if pkg_name.starts_with("torch-cu") {
                                "torch"
                            } else {
                                pkg_name
                            };
                            if let Some(pkg) = installed.iter().find(|p| p.name == base_name) {
                                let ver = pkg.version.as_deref().unwrap_or("unknown");
                                db.add_template_package(
                                    t_id,
                                    &pkg.name,
                                    ver,
                                    true,
                                    if pkg.is_editable { "edit" } else { "pypi" },
                                    install_args_str.as_deref(),
                                )?;
                            }
                        }
                    } else {
                        let e_id = target_id.ok_or("Missing environment ID")?;
                        // Log to audit log
                        let installed = utils::get_packages(&target_path);
                        for pkg_name in &packages {
                            let base_name = if pkg_name.starts_with("torch-cu") {
                                "torch"
                            } else {
                                pkg_name
                            };
                            if let Some(pkg) = installed.iter().find(|p| p.name == base_name) {
                                let ver = pkg.version.as_deref().unwrap_or("unknown");
                                db.log_package(e_id, &pkg.name, ver, "pypi")?;
                            }
                        }
                    }
                    println!("Installation complete.");
                    // Extract env name for logging
                    let log_env = std::path::Path::new(&target_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| target_path.clone());
                    activity_log::log_activity(
                        "cli",
                        "install",
                        &format!("{} {}", log_env, packages.join(" ")),
                    );
                } else {
                    let log_env = std::path::Path::new(&target_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| target_path.clone());
                    activity_log::log_activity(
                        "cli",
                        "install:error",
                        &format!("{} {}", log_env, packages.join(" ")),
                    );
                    eprintln!(
                        "{} Package installation failed. Check the error message above.",
                        "Error:".red()
                    );
                    std::process::exit(1);
                }
            }
            Commands::Run { name, command } => {
                let env_name = types::EnvName::new(&name)?;
                match ops.run_in_env(&env_name, command) {
                    Ok((code, output)) => {
                        print!("{}", output);
                        if code != 0 {
                            std::process::exit(code);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Commands::Uninstall { packages, env } => {
                let env_name = if let Some(name) = env {
                    types::EnvName::new(&name)?
                } else if let Some(session) = db.get_active_session()? {
                    let envs = db.list_envs()?;
                    let e = envs.iter().find(|(_, p, ..)| p == &session.1);
                    if let Some((n, ..)) = e {
                        types::EnvName::new(n)?
                    } else {
                        return Err("Could not resolve session to an environment".into());
                    }
                } else {
                    // Fall back: try to resolve from $VIRTUAL_ENV
                    let resolved = resolve_env_name(None, &db).map_err(
                        |_| "No active environment. Use: zen uninstall -n <env> <packages>",
                    )?;
                    types::EnvName::new(&resolved)?
                };

                match ops.uninstall_packages(&env_name, packages.clone()) {
                    Ok(msg) => {
                        println!("{}", msg);
                        activity_log::log_activity(
                            "cli",
                            "uninstall",
                            &format!("{} {}", env_name.as_str(), packages.join(" ")),
                        );
                    }
                    Err(e) => {
                        activity_log::log_activity(
                            "cli",
                            "uninstall:error",
                            &format!("{} {} - {}", env_name.as_str(), packages.join(" "), e),
                        );
                        eprintln!("{} {}", "Error:".red(), e);
                        return Ok(());
                    }
                }
            }
            Commands::Info { name } => {
                let name = resolve_env_name(name, &db)?;
                let envs = ops.list_envs_with_status(None, None, None)?;
                let env = envs.iter().find(|(n, ..)| n == &name);
                if let Some((_, path, _, exists, ..)) = env {
                    if !exists {
                        println!(
                            "Environment: {} (MISSING on filesystem)",
                            name.magenta().bold()
                        );
                    } else {
                        let py_ver = utils::read_python_version(path)
                            .unwrap_or_else(|| "unknown".to_string());
                        println!(
                            "{}  {}",
                            "Environment:".bold(),
                            name.truecolor(100, 200, 255)
                        );
                        println!("{}       {}", "Path:".bold(), path.dimmed());
                        println!("{}     {}", "Python:".bold(), py_ver);

                        // Torch version from version.py (accurate CUDA suffix)
                        let (torch_ver, cuda_ver) = utils::read_torch_version(path)
                            .map(|(t, c)| (Some(t), c))
                            .unwrap_or((None, None));

                        // All packages from scan
                        let packages = utils::get_packages(path);
                        let get_ver = |name: &str| {
                            packages
                                .iter()
                                .find(|p| p.name == name)
                                .and_then(|p| p.version.clone())
                        };

                        // NumPy with version coloring
                        if let Some(np_ver) = get_ver("numpy") {
                            let colored = if np_ver.starts_with('2') || np_ver.starts_with('3') {
                                np_ver.truecolor(100, 200, 255).to_string()
                            } else {
                                np_ver.truecolor(255, 140, 0).to_string()
                            };
                            println!("{}      {}", "NumPy:".bold(), colored);
                        }

                        // Torch with +cu coloring
                        if let Some(ref tv) = torch_ver {
                            let colored = if tv.contains("+cu") {
                                tv.green().to_string()
                            } else {
                                tv.to_string()
                            };
                            println!("{}      {}", "Torch:".bold(), colored);
                        }
                        if let Some(ref cv) = cuda_ver {
                            println!("{}       {}", "CUDA:".bold(), cv);
                        }

                        // Package count
                        println!(
                            "{}   {}",
                            "Packages:".bold(),
                            packages.len().to_string().dimmed()
                        );

                        // Quick health
                        let health = crate::ops::check_health_quick(std::path::Path::new(path));
                        let health_str = match health {
                            crate::types::HealthLevel::Pass => {
                                format!("{} {}", "✓".truecolor(100, 200, 255), "ok".dimmed())
                            }
                            crate::types::HealthLevel::Info => {
                                format!("{} {}", "△".truecolor(255, 182, 193), "minor".dimmed())
                            }
                            crate::types::HealthLevel::Warn => {
                                format!("{} {}", "!".truecolor(255, 140, 0), "drift".dimmed())
                            }
                            crate::types::HealthLevel::Fail => {
                                format!("{} {}", "✗".red(), "broken".dimmed())
                            }
                        };
                        println!("{}     {}", "Health:".bold(), health_str);

                        // Editable source packages
                        let source: Vec<_> = packages
                            .iter()
                            .filter(|p| p.is_editable)
                            .map(|p| p.name.clone())
                            .collect();
                        if !source.is_empty() {
                            println!(
                                "{}     {}",
                                "Project:".bold(),
                                source.join(", ").truecolor(100, 200, 255)
                            );
                        }
                    }
                } else {
                    eprintln!("Environment '{}' not found.", name);
                }
            }
            Commands::Status => {
                let envs = db.list_envs()?;
                let active = ops.infer_current_env()?;

                println!(
                    "\n{}",
                    " Zen System Dashboard "
                        .bold()
                        .on_truecolor(100, 160, 160)
                        .white()
                );
                println!("{}", "----------------------".truecolor(100, 160, 160));

                if let Some(name) = active {
                    println!("  {: <20} {}", "Active Env:".bold(), name.green().bold());
                } else {
                    println!("  {: <20} {}", "Active Env:".bold(), "none".dimmed());
                }

                println!(
                    "  {: <20} {}",
                    "Managed Envs:".bold(),
                    envs.len().to_string().truecolor(100, 160, 160)
                );

                let db_path = cli.db_path.clone().unwrap_or_else(|| {
                    let home = home::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                    home.join(".config").join("zen").join("zen.db")
                });
                println!(
                    "  {: <20} {}",
                    "Database:".bold(),
                    db_path.display().to_string().dimmed()
                );

                let mode = db.get_config("mode")?.unwrap_or_else(|| "cli".to_string());
                println!(
                    "  {: <20} {}",
                    "Mode:".bold(),
                    mode.truecolor(100, 160, 160)
                );

                println!("  {: <20} {}\n", "Companion:".bold(), "Active".green());
            }

            Commands::Export { file } => {
                #[derive(serde::Serialize)]
                struct TemplateExport {
                    name: String,
                    version: String,
                    python_version: String,
                    packages: Vec<(String, String, bool, String, Option<String>)>, // includes install_args
                }

                #[derive(serde::Serialize)]
                struct FullRegistry {
                    environments: Vec<(
                        String, // name
                        String, // path
                        String, // python_version
                        String, // updated_at
                        bool,   // is_favorite
                    )>,
                    templates: Vec<TemplateExport>,
                }

                let envs = db.list_envs()?;
                let tpls_data = db.get_all_templates_with_packages()?;
                let templates_export = tpls_data
                    .into_iter()
                    .map(|(name, version, python_version, packages)| TemplateExport {
                        name,
                        version,
                        python_version,
                        packages,
                    })
                    .collect();

                let registry = FullRegistry {
                    environments: envs,
                    templates: templates_export,
                };

                let json = serde_json::to_string_pretty(&registry)?;
                std::fs::write(file, json)?;
                println!("Full registry (environments + templates) exported.");
            }
            Commands::Import { file } => {
                #[derive(serde::Deserialize)]
                struct FullRegistry {
                    environments: Vec<(
                        String, // name
                        String, // path
                        String, // python_version
                        String, // updated_at
                        bool,   // is_favorite
                    )>,
                    templates: Vec<TemplateExport>,
                }
                #[derive(serde::Deserialize)]
                struct TemplateExport {
                    name: String,
                    version: String,
                    python_version: String,
                    packages: Vec<(String, String, bool, String, Option<String>)>, // includes install_args
                }

                let content = std::fs::read_to_string(file)?;
                let registry: FullRegistry = serde_json::from_str(&content)?;

                for (name, path, python, ..) in registry.environments {
                    db.register_env(&name, &path, &python)?;
                }

                for t in registry.templates {
                    let t_id = db.create_template(&t.name, &t.version, &t.python_version)?;
                    for (p_name, p_ver, is_pinned, install_type, install_args) in t.packages {
                        db.add_template_package(
                            t_id,
                            &p_name,
                            &p_ver,
                            is_pinned,
                            &install_type,
                            install_args.as_deref(),
                        )?;
                    }
                }
                println!("Full registry (environments + templates) imported.");
            }
            Commands::Setup { subcommand } => match subcommand {
                SetupCommands::Init { path, yes } => {
                    println!(
                        "Zen Setup Wizard: Scanning {} for environments...",
                        path.display()
                    );
                    let found = crate::utils::discover_venvs(&path);

                    if found.is_empty() {
                        println!("No virtual environments found in this directory.");
                    } else {
                        let confirm = if yes {
                            true
                        } else {
                            println!("\nFound {} environments in this directory.", found.len());
                            match dialoguer::Confirm::new()
                                .with_prompt("Do you want to import them into Zen now?")
                                .default(true)
                                .interact()
                            {
                                Ok(v) => v,
                                Err(_) => {
                                    println!();
                                    return Ok(());
                                }
                            }
                        };

                        if confirm {
                            println!("Importing... (this will scan packages for each env)");
                            match ops.bulk_import(found) {
                                Ok(msg) => println!("\n✓ {}", msg),
                                Err(e) => eprintln!("\nError: {}", e),
                            }
                        } else {
                            println!("Import cancelled.");
                        }
                    }
                }
                SetupCommands::StackInfo => {
                    use dialoguer::{Input, theme::ColorfulTheme};
                    let config = db
                        .get_config("stack_info")?
                        .unwrap_or_else(|| "torch numpy transformers diffusers".to_string());
                    let new_config: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enter packages to track (space separated)")
                        .default(config)
                        .interact_text()?;
                    db.set_config("stack_info", &new_config)?;
                    println!("{} Stack info packages updated.", "✓".green());
                }
            },
            Commands::Note { subcommand } | Commands::Comment { subcommand } => match subcommand {
                NoteCommands::Add { env, message } => {
                    let env = resolve_env_name(env, &db)?;
                    let env_name = types::EnvName::new(&env).map_err(|e| e.to_string())?;
                    match ops.log_comment(Some(&env_name), &message) {
                        Ok(resp) => println!("{}", resp),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
                NoteCommands::List { env, all } => {
                    let (env_filter, show_env_col) = if all {
                        (None, true)
                    } else {
                        let env = resolve_env_name(env, &db)?;
                        (
                            Some(types::EnvName::new(&env).map_err(|e| e.to_string())?),
                            false,
                        )
                    };
                    match ops.list_comments(None, env_filter.as_ref()) {
                        Ok(comments) => {
                            if comments.is_empty() {
                                if show_env_col {
                                    println!("No notes found.");
                                } else {
                                    println!("No notes for '{}'", env_filter.unwrap());
                                }
                            } else {
                                use comfy_table::{Cell, Color};
                                let mut table = crate::table::new_table();
                                if show_env_col {
                                    table.set_header(vec!["UUID", "Env", "Note", "Timestamp"]);
                                } else {
                                    table.set_header(vec!["UUID", "Note", "Timestamp"]);
                                }
                                for (uuid, _pp, env_name, msg, _tag, ts) in comments {
                                    let short_uuid = if uuid.len() > 8 {
                                        format!("{}…", &uuid[..8])
                                    } else {
                                        uuid.clone()
                                    };
                                    if show_env_col {
                                        table.add_row(vec![
                                            Cell::new(short_uuid).fg(Color::DarkGrey),
                                            Cell::new(env_name.unwrap_or_else(|| "-".into()))
                                                .fg(Color::Cyan),
                                            Cell::new(msg),
                                            Cell::new(ts).fg(Color::DarkGrey),
                                        ]);
                                    } else {
                                        table.add_row(vec![
                                            Cell::new(short_uuid).fg(Color::DarkGrey),
                                            Cell::new(msg),
                                            Cell::new(ts).fg(Color::DarkGrey),
                                        ]);
                                    }
                                }
                                println!("{}", table);
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
                NoteCommands::Rm { uuid } => match ops.remove_comment(&uuid) {
                    Ok(0) => eprintln!("{} No note found matching '{}'", "✗".red(), uuid),
                    Ok(1) => println!("{} Note {} removed.", "✓".green(), uuid),
                    Ok(n) => println!("{} {} notes removed (prefix '{}')", "⚠".yellow(), n, uuid),
                    Err(e) => eprintln!("Error: {}", e),
                },
            },

            Commands::Label { subcommand } => match subcommand {
                LabelCommands::Add { env, label } => {
                    let env = resolve_env_name(env, &db)?;
                    match db.add_label(&env, &label) {
                        Ok(_) => println!("{} Added label '{}' to '{}'", "✓".green(), label, env),
                        Err(e) => eprintln!("{} {}", "Error:".red(), e),
                    }
                }
                LabelCommands::Rm { env, label } => {
                    let env = resolve_env_name(env, &db)?;
                    match db.remove_label(&env, &label) {
                        Ok(_) => {
                            println!("{} Removed label '{}' from '{}'", "✓".green(), label, env)
                        }
                        Err(e) => eprintln!("{} {}", "Error:".red(), e),
                    }
                }
                LabelCommands::List { env, all } => {
                    if all {
                        match db.get_all_labels() {
                            Ok(entries) => {
                                if entries.is_empty() {
                                    println!("No labels found.");
                                } else {
                                    for (env, labels) in entries {
                                        println!("{}: {}", env, labels.join(", "));
                                    }
                                }
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    } else {
                        let env = resolve_env_name(env, &db)?;
                        match db.get_labels(&env) {
                            Ok(labels) => {
                                if labels.is_empty() {
                                    println!("No labels for '{}'", env);
                                } else {
                                    println!("{}: {}", env, labels.join(", "));
                                }
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                }
            },
            Commands::Find { package, exact } => {
                // Split query into name and optional version at "=="
                let (pkg_query, version_query) = if package.contains("==") {
                    let parts: Vec<&str> = package.split("==").collect();
                    (
                        parts[0].to_string(),
                        Some(parts.get(1).unwrap_or(&"").to_string()),
                    )
                } else {
                    (package.clone(), None)
                };

                let pattern = pkg_query.replace('*', "");
                // pip treats hyphens and underscores as equivalent
                let normalize = |s: &str| s.to_lowercase().replace('-', "_");

                let envs = db.list_envs()?;
                let mut found = Vec::new();

                for (name, path, ..) in &envs {
                    let packages = crate::utils::get_packages(path);
                    for pkg in packages {
                        let pkg_norm = normalize(&pkg.name);
                        let pattern_norm = normalize(&pattern);

                        // Default: substring match. --exact: exact name match
                        let name_match = if exact {
                            pkg_norm == pattern_norm
                        } else {
                            pkg_norm.contains(&pattern_norm)
                        };

                        // Version match with CUDA-awareness
                        let version_match = match (&version_query, &pkg.version) {
                            (Some(q), Some(v)) => {
                                if q.contains('+') {
                                    // Query has +cuXXX: exact match
                                    v == q
                                } else {
                                    // Query without +: match base version (before +)
                                    let base_ver = v.split('+').next().unwrap_or(v);
                                    base_ver.starts_with(q.as_str())
                                }
                            }
                            (Some(_), None) => false,
                            (None, _) => true,
                        };

                        if name_match && version_match {
                            found.push((name.clone(), pkg.name.clone(), pkg.version.clone()));
                        }
                    }
                }

                if found.is_empty() {
                    println!("No environments contain package matching '{}'", package);
                } else {
                    println!("{}", "Package matches:".bold());
                    for (env, pkg_name, version) in found {
                        let ver = version.unwrap_or_else(|| "?".to_string());
                        println!(
                            "  {} {} {} {}",
                            env.cyan(),
                            pkg_name,
                            "→".dimmed(),
                            ver.green()
                        );
                    }
                }
            }
            Commands::Inspect {
                env,
                package,
                names_only,
                long,
            } => {
                let env = resolve_env_name(env, &db)?;
                let envs = db.list_envs()?;
                let env_entry = envs.iter().find(|(n, ..)| n == &env);
                if let Some((name, path, ..)) = env_entry {
                    let packages = crate::utils::get_packages(path);

                    if let Some(package) = package {
                        // Single package detail view
                        let pkg_lower = package.to_lowercase();
                        let found = packages
                            .into_iter()
                            .find(|p| p.name.to_lowercase() == pkg_lower);

                        if let Some(pkg) = found {
                            let ver_str = pkg.version.as_deref().unwrap_or("unknown");
                            let colored_ver = if ver_str.contains("+cu") {
                                ver_str.green().to_string()
                            } else {
                                ver_str.to_string()
                            };
                            let source_str = pkg.install_source.as_deref().unwrap_or("unknown");
                            let colored_source = if source_str == "pypi" {
                                source_str.dimmed().to_string()
                            } else {
                                source_str.cyan().to_string()
                            };
                            println!(
                                "{:12}{}",
                                "Package:".bold(),
                                pkg.name.truecolor(100, 200, 255)
                            );
                            println!("{:12}{}", "Version:".bold(), colored_ver);
                            println!(
                                "{:12}{}",
                                "Installer:".bold(),
                                pkg.installer.as_deref().unwrap_or("unknown").dimmed()
                            );
                            println!("{:12}{}", "Project:".bold(), colored_source);
                            println!(
                                "{:12}{}",
                                "Editable:".bold(),
                                if pkg.is_editable {
                                    "yes".truecolor(100, 200, 255).to_string()
                                } else {
                                    "no".dimmed().to_string()
                                }
                            );
                            if let Some(url) = &pkg.source_url {
                                println!("{:12}{}", "URL:".bold(), url.cyan());
                            }
                            if let Some(commit) = &pkg.commit_id {
                                println!("{:12}{}", "Commit:".bold(), commit.dimmed());
                            }
                            if let Some(ref import) = pkg.import_name {
                                println!(
                                    "{:12}{}",
                                    "Import:".bold(),
                                    import.truecolor(100, 200, 255)
                                );
                            }
                            if let Some(epoch) = pkg.installed_at {
                                use chrono::{Local, TimeZone};
                                if let Some(dt) = Local.timestamp_opt(epoch, 0).single() {
                                    println!(
                                        "{:12}{}",
                                        "Installed:".bold(),
                                        dt.format("%Y-%m-%d %H:%M").to_string().dimmed()
                                    );
                                }
                            }
                        } else {
                            eprintln!("Package '{}' not found in environment '{}'", package, name);
                        }
                    } else {
                        // List all packages
                        let mut sorted = packages;
                        sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                        if names_only {
                            // -1: one name per line
                            for pkg in &sorted {
                                println!("{}", pkg.name);
                            }
                        } else if long {
                            // -l: long format, aligned name + version + installer + date
                            println!(
                                "{} {} — {} package(s)",
                                "●".truecolor(100, 200, 255),
                                name.truecolor(100, 200, 255).bold(),
                                sorted.len()
                            );
                            println!();
                            let max_name = sorted.iter().map(|p| p.name.len()).max().unwrap_or(20);
                            let max_ver = sorted
                                .iter()
                                .map(|p| p.version.as_deref().unwrap_or("?").len())
                                .max()
                                .unwrap_or(10);
                            for pkg in &sorted {
                                let ver = pkg.version.as_deref().unwrap_or("?");
                                let colored_ver = if ver.contains("+cu") {
                                    ver.green().to_string()
                                } else {
                                    ver.dimmed().to_string()
                                };
                                let installer = pkg.installer.as_deref().unwrap_or("?");
                                let editable_mark = if pkg.is_editable { " ✎" } else { "" };
                                let date_str = if let Some(epoch) = pkg.installed_at {
                                    use chrono::{Local, TimeZone};
                                    if let Some(dt) = Local.timestamp_opt(epoch, 0).single() {
                                        dt.format("%Y-%m-%d").to_string()
                                    } else {
                                        String::new()
                                    }
                                } else {
                                    String::new()
                                };
                                println!(
                                    "  {:<nw$}  {:<vw$}  {:<3}  {}{}",
                                    pkg.name.truecolor(100, 200, 255),
                                    colored_ver,
                                    installer.dimmed(),
                                    date_str.dimmed(),
                                    editable_mark,
                                    nw = max_name,
                                    vw = max_ver
                                );
                            }
                        } else {
                            // Default: ls-style column layout
                            println!(
                                "{} {} — {} package(s)",
                                "●".truecolor(100, 200, 255),
                                name.truecolor(100, 200, 255).bold(),
                                sorted.len()
                            );
                            println!();
                            let term_width = terminal_size::terminal_size()
                                .map(|(terminal_size::Width(w), _)| w as usize)
                                .unwrap_or(80);

                            // Build display entries: name(version)
                            let entries: Vec<(String, String)> = sorted
                                .iter()
                                .map(|pkg| {
                                    let ver = pkg.version.as_deref().unwrap_or("?");
                                    let plain = format!("{} ({})", pkg.name, ver);
                                    let colored = format!(
                                        "{} {}{}{}",
                                        pkg.name.truecolor(100, 200, 255),
                                        "(".dimmed(),
                                        if ver.contains("+cu") {
                                            ver.green().to_string()
                                        } else {
                                            ver.dimmed().to_string()
                                        },
                                        ")".dimmed()
                                    );
                                    (plain, colored)
                                })
                                .collect();

                            let max_width =
                                entries.iter().map(|(p, _)| p.len()).max().unwrap_or(20);
                            let col_width = max_width + 2; // 2 char gap
                            let num_cols = (term_width / col_width).max(1);
                            let num_rows = entries.len().div_ceil(num_cols);

                            for row in 0..num_rows {
                                for col in 0..num_cols {
                                    let idx = col * num_rows + row; // column-major
                                    if idx >= entries.len() {
                                        continue;
                                    }
                                    let (ref plain, ref colored) = entries[idx];
                                    if col + 1 < num_cols {
                                        let padding = col_width.saturating_sub(plain.len());
                                        print!("{}{}", colored, " ".repeat(padding));
                                    } else {
                                        print!("{}", colored);
                                    }
                                }
                                println!();
                            }
                        }
                    }
                } else {
                    eprintln!("Environment '{}' not found.", env);
                }
            }
            Commands::Diff {
                env1,
                env2,
                only_diff,
            } => {
                // Compare packages between two environments
                let envs = db.list_envs()?;
                let path1 = envs
                    .iter()
                    .find(|(n, ..)| n == &env1)
                    .map(|(_, p, ..)| p.clone());
                let path2 = envs
                    .iter()
                    .find(|(n, ..)| n == &env2)
                    .map(|(_, p, ..)| p.clone());

                let (path1, path2) = match (path1, path2) {
                    (Some(p1), Some(p2)) => (p1, p2),
                    (None, _) => {
                        eprintln!("{} Environment '{}' not found", "Error:".red(), env1);
                        return Ok(());
                    }
                    (_, None) => {
                        eprintln!("{} Environment '{}' not found", "Error:".red(), env2);
                        return Ok(());
                    }
                };

                let pkgs1: std::collections::HashMap<_, _> = crate::utils::get_packages(&path1)
                    .into_iter()
                    .map(|p| (p.name, p.version))
                    .collect();
                let pkgs2: std::collections::HashMap<_, _> = crate::utils::get_packages(&path2)
                    .into_iter()
                    .map(|p| (p.name, p.version))
                    .collect();

                let mut all_pkgs: Vec<_> = pkgs1.keys().chain(pkgs2.keys()).collect();
                all_pkgs.sort();
                all_pkgs.dedup();

                println!(
                    "{:^30} {:^15} {:^15}",
                    "Package".bold(),
                    env1.cyan(),
                    env2.cyan()
                );
                println!("{}", "─".repeat(60));

                for pkg in all_pkgs {
                    let v1 = pkgs1.get(pkg).and_then(|v| v.clone());
                    let v2 = pkgs2.get(pkg).and_then(|v| v.clone());
                    let is_diff = v1 != v2;

                    if only_diff && !is_diff {
                        continue;
                    }

                    let v1_str = v1.unwrap_or_else(|| "--".to_string());
                    let v2_str = v2.unwrap_or_else(|| "--".to_string());

                    if is_diff {
                        println!(
                            "{:30} {:^15} {:^15}",
                            pkg.yellow(),
                            v1_str.red(),
                            v2_str.green()
                        );
                    } else {
                        println!("{:30} {:^15} {:^15}", pkg, v1_str, v2_str);
                    }
                }
            }
            Commands::Health { name } => {
                let name = resolve_env_name(name, &db)?;
                let env_name = types::EnvName::new(&name).map_err(|e| e.to_string())?;
                match ops.check_health(&env_name) {
                    Ok(report) => {
                        use crate::types::Diagnostic;
                        println!(
                            "{}  {}",
                            "Environment:".bold(),
                            name.truecolor(100, 200, 255)
                        );
                        let label = " Health ";
                        let total_w: usize = 50;
                        let pad = total_w.saturating_sub(label.len()) / 2;
                        println!(
                            "{}{}{}",
                            "─".repeat(pad),
                            label.dimmed(),
                            "─".repeat(total_w - pad - label.len())
                        );
                        for item in &report.items {
                            let (icon, color_msg) = match item.level() {
                                crate::types::HealthLevel::Pass => (
                                    "✓".truecolor(100, 200, 255).to_string(),
                                    item.message().normal().to_string(),
                                ),
                                crate::types::HealthLevel::Info => (
                                    "△".truecolor(255, 182, 193).to_string(),
                                    item.message().truecolor(255, 182, 193).to_string(),
                                ),
                                crate::types::HealthLevel::Warn => (
                                    "⚠".truecolor(255, 140, 0).to_string(),
                                    item.message().truecolor(255, 140, 0).to_string(),
                                ),
                                crate::types::HealthLevel::Fail => {
                                    ("✗".red().to_string(), item.message().red().to_string())
                                }
                            };
                            println!("{} {}", icon, color_msg);
                        }
                        println!();
                        let status = match report.overall() {
                            crate::types::HealthLevel::Pass => {
                                "OK".truecolor(100, 200, 255).bold().to_string()
                            }
                            crate::types::HealthLevel::Info => {
                                "MINOR".truecolor(255, 182, 193).bold().to_string()
                            }
                            crate::types::HealthLevel::Warn => {
                                "DRIFT".truecolor(255, 140, 0).bold().to_string()
                            }
                            crate::types::HealthLevel::Fail => "BROKEN".red().bold().to_string(),
                        };
                        println!("Overall: {}", status);
                    }
                    Err(e) => eprintln!("{} {}", "Error:".red(), e),
                }
            }
            Commands::Activate {
                name,
                path_only,
                last,
            } => {
                // zen activate --last: re-activate most recently used env
                if last {
                    match db.get_last_activated()? {
                        Some((env_name, env_path)) => {
                            if !std::path::Path::new(&env_path).exists() {
                                eprintln!(
                                    "Last activated env '{}' no longer exists on disk.",
                                    env_name
                                );
                                std::process::exit(1);
                            }
                            // Record reactivation at CWD
                            if let Ok(cwd) = std::env::current_dir() {
                                let cwd_str = cwd
                                    .canonicalize()
                                    .unwrap_or(cwd)
                                    .to_string_lossy()
                                    .to_string();
                                let _ = db.record_activation(&cwd_str, &env_name);
                                activity_log::log_activity("cli", "activate", &env_name);
                            }
                            if path_only {
                                println!("{}", env_path);
                            } else {
                                eprintln!("✓ Last activated: {}", env_name);
                            }
                        }
                        None => {
                            eprintln!("No activation history found.");
                            std::process::exit(1);
                        }
                    }
                    return Ok(());
                }

                // zen activate <name>: explicit environment name
                if let Some(ref env_name) = name {
                    let envs = db.list_envs()?;
                    let env = envs.iter().find(|(n, ..)| n == env_name);

                    if let Some((_, path, ..)) = env {
                        // Record activation at CWD
                        if let Ok(cwd) = std::env::current_dir() {
                            let cwd_str = cwd
                                .canonicalize()
                                .unwrap_or(cwd)
                                .to_string_lossy()
                                .to_string();
                            let _ = db.record_activation(&cwd_str, env_name);
                            activity_log::log_activity("cli", "activate", env_name);
                        }
                        if path_only {
                            println!("{}", path);
                        } else {
                            eprintln!(
                                "Shell hook not detected. To enable 'zen activate', add to your shell config:"
                            );
                            eprintln!("  eval \"$(zen hook zsh)\"   # for zsh");
                            eprintln!("  eval \"$(zen hook bash)\"  # for bash");
                        }
                    } else {
                        activity_log::log_activity(
                            "cli",
                            "activate:error",
                            &format!("{} - not found", env_name),
                        );
                        eprintln!("Environment '{}' not found.", env_name);
                        std::process::exit(1);
                    }
                    return Ok(());
                }

                // zen activate (no args): smart selection from project hierarchy
                let cwd = std::env::current_dir()?
                    .canonicalize()?
                    .to_string_lossy()
                    .to_string();

                // === Bidirectional activation search ===
                //
                // DOWNWARD: check subfolder links (up to 2 levels deep)
                //   If someone linked an env to a dir *inside* this project, find it.
                //
                // UPWARD: check exact ancestor paths (up to 2 levels)
                //   If the parent directory itself is linked, find it.
                //   Block umbrella dirs (children of / or $HOME) — they're never projects.
                //
                let home_dir = std::env::var("HOME").unwrap_or_default();
                let stop_dirs: Vec<&str> = vec!["/", "/tmp", "/home", "/root"];

                // 1. Downward: subfolder links (CWD exact + children up to depth 2)
                let mut all_candidates =
                    db.get_activation_candidates(std::slice::from_ref(&cwd))?;
                let subfolder_candidates = db.get_subfolder_candidates(&cwd, 2)?;
                all_candidates.extend(subfolder_candidates);

                // 2. Upward: exact ancestor match (max 2 levels)
                let mut current = std::path::Path::new(&cwd).to_path_buf();
                let root_path = std::path::Path::new("/");
                let home_path = std::path::Path::new(&home_dir);
                let mut up_depth = 0;
                while let Some(parent) = current.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if parent_str == home_dir || stop_dirs.contains(&parent_str.as_str()) {
                        break;
                    }
                    // Block umbrella dirs: children of / or $HOME
                    if parent.parent() == Some(root_path) || parent.parent() == Some(home_path) {
                        break;
                    }
                    up_depth += 1;
                    if up_depth > 2 {
                        break;
                    }
                    let ancestor_candidates = db.get_activation_candidates(&[parent_str])?;
                    all_candidates.extend(ancestor_candidates);
                    current = parent.to_path_buf();
                }

                // Deduplicate by env name (keep first occurrence = highest priority)
                let mut seen = std::collections::HashSet::new();
                let candidates: Vec<_> = all_candidates
                    .into_iter()
                    .filter(|(env_name, _, _, _, _)| seen.insert(env_name.clone()))
                    .collect();

                // Validate on disk
                let valid: Vec<_> = candidates
                    .into_iter()
                    .filter(|(env_name, env_path, _, _, _)| {
                        if std::path::Path::new(env_path).exists() {
                            true
                        } else {
                            eprintln!("⚠ Stale link: '{}' no longer exists on disk", env_name);
                            false
                        }
                    })
                    .collect();

                match valid.len() {
                    0 => {
                        eprintln!("No environments linked to this directory.");
                        eprintln!("Use: {} to link one.", "zen link add <env>".cyan());
                        std::process::exit(1);
                    }
                    1 => {
                        // Auto-select single candidate
                        let (env_name, env_path, project_path, count, _) = &valid[0];
                        let rel = project_path.clone();
                        let _ = db.record_activation(&cwd, env_name);
                        if path_only {
                            eprintln!(
                                "✓ Auto-selecting: {} ({}{})",
                                env_name.cyan(),
                                rel.dimmed(),
                                if *count >= 10 {
                                    " ·frequent".to_string()
                                } else {
                                    String::new()
                                }
                            );
                            println!("{}", env_path);
                        } else {
                            eprintln!("✓ Auto-selecting: {} ({})", env_name.cyan(), rel.dimmed());
                        }
                    }
                    _ => {
                        // Interactive menu on stderr
                        eprintln!("\n{}", "Previously activated environments:".cyan());
                        for (i, (env_name, _, project_path, count, link_type)) in
                            valid.iter().enumerate()
                        {
                            let rel = project_path.clone();
                            let count_str = if *count >= 10 {
                                " ·frequent".to_string()
                            } else {
                                String::new()
                            };
                            let type_marker = if link_type == "user" { " ★" } else { "" };
                            eprintln!(
                                "  {}: {}{} ({}{})",
                                (i + 1).to_string().bold(),
                                env_name.bold(),
                                type_marker,
                                rel.dimmed(),
                                count_str
                            );
                        }
                        eprintln!("  {}: Cancel activation", "0".bold());
                        eprint!("\nSelect [{}]: ", "1".bold());

                        // Read selection from stdin
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        let choice = input.trim();

                        let idx: usize = if choice.is_empty() {
                            0 // Default to first option
                        } else if let Ok(n) = choice.parse::<usize>() {
                            if n == 0 {
                                eprintln!("Cancelled.");
                                std::process::exit(0);
                            }
                            n - 1
                        } else {
                            eprintln!("Invalid selection.");
                            std::process::exit(1);
                        };

                        if idx >= valid.len() {
                            eprintln!("Invalid selection.");
                            std::process::exit(1);
                        }

                        let (env_name, env_path, _, _, _) = &valid[idx];
                        let _ = db.record_activation(&cwd, env_name);
                        if path_only {
                            println!("{}", env_path);
                        } else {
                            eprintln!("Selected: {}", env_name.cyan());
                        }
                    }
                }
            }
            Commands::Hook { shell } => {
                print!("{}", crate::hooks::generate_hook(&shell));
            }
            Commands::Clone { source, name } => {
                let envs = db.list_envs()?;
                let found = envs.iter().find(|(n, ..)| n == &source);
                let (_, source_path, source_py, ..) = match found {
                    Some(e) => e,
                    None => {
                        eprintln!(
                            "{} Source environment '{}' not found.",
                            "Error:".red(),
                            source
                        );
                        return Ok(());
                    }
                };

                // Check if target already exists
                if envs.iter().any(|(n, ..)| n == &name) {
                    activity_log::log_activity(
                        "cli",
                        "clone:error",
                        &format!("{} -> {} - target exists", source, name),
                    );
                    eprintln!("{} Environment '{}' already exists.", "Error:".red(), name);
                    return Ok(());
                }

                println!("Cloning '{}' → '{}'...", source, name);

                // Create target path using configured home
                let target_path = cli.home.join(&name);

                // Copy the entire directory
                let copy_result = std::process::Command::new("cp")
                    .args(["-r", source_path, target_path.to_str().unwrap()])
                    .status();

                if copy_result.is_err() || !copy_result.unwrap().success() {
                    activity_log::log_activity(
                        "cli",
                        "clone:error",
                        &format!("{} -> {} - copy failed", source, name),
                    );
                    eprintln!("{} Failed to copy environment directory.", "Error:".red());
                    return Ok(());
                }

                // Register the new environment
                let new_id = db.register_env(&name, target_path.to_str().unwrap(), source_py)?;

                // Copy package metadata from filesystem
                let packages = utils::get_packages(target_path.to_str().unwrap());
                for pkg in packages {
                    let ver = pkg.version.as_deref().unwrap_or("unknown");
                    db.log_package(new_id, &pkg.name, ver, "pypi")?;
                }

                // Package versions are now tracked dynamically via `zen list --refresh`

                activity_log::log_activity("cli", "clone", &format!("{} -> {}", source, name));
                println!("✓ Environment '{}' cloned successfully!", name);
                println!("  Project: {} ({})", source, source_path);
                println!("  Target: {} ({})", name, target_path.display());
            }
            Commands::Completions { shell } => {
                use clap::CommandFactory;
                use clap_complete::generate;

                let mut cmd = Cli::command();
                let bin_name = cmd.get_name().to_string();
                generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
            }
            Commands::Link { subcommand } => match subcommand {
                LinkCommands::Add { name, path } => {
                    let name = resolve_env_name(name, &db)?;
                    let envs = db.list_envs()?;
                    let env = envs.iter().find(|(n, ..)| n == &name);
                    if let Some((_, _path, ..)) = env {
                        // Get project path: --path override or current dir
                        let project_path = match path {
                            Some(p) => std::path::Path::new(&p)
                                .canonicalize()
                                .map_err(|e| format!("Invalid path '{}': {}", p, e))?
                                .to_string_lossy()
                                .to_string(),
                            None => std::env::current_dir()?
                                .canonicalize()?
                                .to_string_lossy()
                                .to_string(),
                        };

                        // Store in database
                        db.associate_project(&project_path, &name, None, true)?;
                        activity_log::log_activity(
                            "cli",
                            "link:add",
                            &format!("{} -> {}", name, project_path),
                        );
                        println!("Linked '{}' to this project.", name.cyan());
                    } else {
                        eprintln!(
                            "Environment '{}' not found. Run 'zen list' to see available environments.",
                            name
                        );
                    }
                }
                LinkCommands::Rm { name, path } => {
                    let name = resolve_env_name(name, &db)?;
                    let project_path = match path {
                        Some(p) => std::path::Path::new(&p)
                            .canonicalize()
                            .map_err(|e| format!("Invalid path '{}': {}", p, e))?
                            .to_string_lossy()
                            .to_string(),
                        None => std::env::current_dir()?
                            .canonicalize()?
                            .to_string_lossy()
                            .to_string(),
                    };

                    // Get env_id and remove association
                    if let Some(env_id) = db.get_env_id(&name)? {
                        db.remove_project_association(&project_path, env_id)?;
                        activity_log::log_activity(
                            "cli",
                            "link:rm",
                            &format!("{} -> {}", name, project_path),
                        );
                        println!("Unlinked '{}' from this project.", name.yellow());
                    } else {
                        activity_log::log_activity(
                            "cli",
                            "link:rm:error",
                            &format!("{} - not found", name),
                        );
                        eprintln!("Environment '{}' not found.", name);
                    }
                }
                LinkCommands::List { path } => {
                    let project_path = match path {
                        Some(p) => std::path::Path::new(&p)
                            .canonicalize()
                            .map_err(|e| format!("Invalid path '{}': {}", p, e))?
                            .to_string_lossy()
                            .to_string(),
                        None => std::env::current_dir()?
                            .canonicalize()?
                            .to_string_lossy()
                            .to_string(),
                    };

                    // Get linked environments with activation stats
                    let links = db.get_project_links_with_stats(&project_path)?;

                    if links.is_empty() {
                        // Check for inherited (parent path prefix match)
                        let all_projects = db.get_all_project_paths()?;
                        let inherited: Vec<_> = all_projects
                            .iter()
                            .filter(|p| project_path.starts_with(*p) && *p != &project_path)
                            .collect();

                        if !inherited.is_empty() {
                            let parent = inherited.iter().max_by_key(|p| p.len()).unwrap();
                            let parent_links = db.get_project_links_with_stats(parent)?;
                            if !parent_links.is_empty() {
                                println!(
                                    "{} (inherited from {}):",
                                    "Linked environments".cyan(),
                                    parent
                                );
                                for (
                                    env_name,
                                    env_path,
                                    tag,
                                    is_default,
                                    link_type,
                                    count,
                                    last_at,
                                ) in parent_links
                                {
                                    print_link_entry(
                                        &env_name, &env_path, &tag, is_default, &link_type, count,
                                        &last_at,
                                    );
                                }
                                return Ok(());
                            }
                        }
                        println!("No environments linked. Use 'zen link add <env>' to link one.");
                    } else {
                        println!("{}:", "Linked environments".cyan());
                        for (env_name, env_path, tag, is_default, link_type, count, last_at) in
                            links
                        {
                            print_link_entry(
                                &env_name, &env_path, &tag, is_default, &link_type, count, &last_at,
                            );
                        }
                    }
                }
                LinkCommands::Prune => {
                    let pruned = db.prune_stale_links()?;
                    if pruned.is_empty() {
                        println!("No stale links found. All links are valid.");
                    } else {
                        println!("Pruned {} stale link(s):", pruned.len());
                        for (project_path, env_name, reason) in &pruned {
                            println!(
                                "  {} '{}' at {} ({})",
                                "✗".red(),
                                env_name,
                                project_path.dimmed(),
                                reason.dimmed()
                            );
                        }
                    }
                }
                LinkCommands::Reset {
                    path,
                    activations,
                    history,
                    older_than,
                } => {
                    if let Some(p) = path {
                        // Remove ALL links for a specific path
                        let resolved = if p == "." {
                            std::env::current_dir()?
                                .canonicalize()?
                                .to_string_lossy()
                                .to_string()
                        } else {
                            std::path::Path::new(&p)
                                .canonicalize()
                                .unwrap_or_else(|_| std::path::PathBuf::from(&p))
                                .to_string_lossy()
                                .to_string()
                        };
                        let count = db.remove_links_for_path(&resolved)?;
                        if count == 0 {
                            println!("No links found for '{}'", resolved);
                        } else {
                            activity_log::log_activity(
                                "cli",
                                "link:reset",
                                &format!("path:{} ({})", resolved, count),
                            );
                            println!(
                                "{} Removed {} link(s) for '{}'",
                                "✓".green(),
                                count,
                                resolved
                            );
                        }
                    } else if activations {
                        // Remove links that were auto-created by activation (not explicit zen link)
                        let count = db.remove_activation_links(older_than)?;
                        if count == 0 {
                            println!("No auto-created activation links to remove.");
                        } else {
                            println!("{} Removed {} auto-created link(s).", "✓".green(), count);
                        }
                    } else if history {
                        // Just clear counts/timestamps, keep all links
                        let count = db.reset_activation_history(older_than)?;
                        if count == 0 {
                            println!("No activation history to clear.");
                        } else {
                            println!(
                                "{} Cleared activation history for {} link(s).",
                                "✓".green(),
                                count
                            );
                        }
                    } else {
                        // Default: clear all history
                        let count = db.reset_activation_history(older_than)?;
                        if count == 0 {
                            println!("No activation history to clear.");
                        } else {
                            println!(
                                "{} Cleared activation history for {} link(s).",
                                "✓".green(),
                                count
                            );
                        }
                    }
                }
            },
            Commands::Log {
                filter,
                lines,
                clear,
            } => {
                if clear {
                    activity_log::clear_log();
                    println!("Log cleared.");
                    return Ok(());
                }
                let entries = activity_log::read_log(lines, filter.as_deref());
                if entries.is_empty() {
                    println!("No log entries found.");
                } else {
                    for entry in &entries {
                        println!("{}", entry);
                    }
                    println!("{}", format!("({} entries)", entries.len()).dimmed());
                }
            }
            Commands::Mcp => {
                let db_clone = db::Database::open(cli.db_path.as_deref())?;
                tokio::runtime::Runtime::new()?
                    .block_on(crate::mcp::run_server(db_clone, cli.home.clone()))?;
            }
        }
        Ok(())
    })();

    if let Err(ref e) = result {
        eprintln!("Error: {}", e);
    }

    result
}
