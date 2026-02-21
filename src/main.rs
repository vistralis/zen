// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::type_complexity)]

mod activity_log;
mod commands;
mod context;
mod db;
mod error;
mod hooks;
mod mcp;
mod ops;
mod repl;
mod table;
mod types;
mod utils;
mod validation;

use crate::db::Database;
use crate::types::EnvName;
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

#[derive(ValueEnum, Clone, Debug)]
enum ListSort {
    Name,
    Date,
}

#[derive(ValueEnum, Clone, Debug)]
enum ListFormatArg {
    Auto,
    Minimal,
    Compact,
    Wide,
}

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
        name: EnvName,

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

        /// Extra positional args (hidden, used for typo detection)
        #[arg(hide = true, trailing_var_arg = true)]
        rest: Vec<String>,
    },
    /// Register an existing virtual environment
    Add {
        /// Path to venv root, bin/python, or bin/activate
        path: PathBuf,
        /// Override the inferred environment name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Rename an existing environment
    Rename {
        /// Current name
        old: EnvName,
        /// New name
        new: EnvName,
    },
    /// List all managed environments
    #[command(visible_alias = "ls")]
    List {
        /// Wildcard pattern to filter environments (e.g., *ai*)
        pattern: Option<String>,
        /// Sort by field
        #[arg(long, default_value = "name")]
        sort: ListSort,
        /// Filter by label (e.g., --label ml)
        #[arg(long)]
        label: Option<String>,
        /// Output format
        #[arg(long, default_value = "auto")]
        format: ListFormatArg,
        /// Names only, one per line (like ls -1)
        #[arg(short = '1')]
        oneline: bool,
        /// Long format with paths (like ls -l)
        #[arg(short = 'l')]
        long_format: bool,
    },
    /// Remove an environment from the database and disk
    Rm {
        /// Name of the environment to remove
        name: EnvName,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Remove from database only, keep files on disk
        #[arg(long)]
        cached: bool,
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
        name: EnvName,
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
        source: EnvName,
        /// Name for the new environment
        name: EnvName,
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
        env1: EnvName,
        /// Second environment
        env2: EnvName,
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
    /// List all templates, or inspect one by name
    List {
        /// Optional template name to inspect
        name: Option<String>,
    },
    /// Remove a template
    Rm { name: String },
    /// Update unpinned dependencies for a template
    Update { name: String },
    /// Inspect template contents (Docker-style layered view)
    Inspect {
        /// Template name (e.g., ml-cu130 or ml-cu130:latest)
        name: String,
    },
    /// Edit a template — add/drop packages or steps
    ///
    /// One-shot mode:
    ///   zen template edit ml-cu130 drop bitsandbytes
    ///   zen template edit ml-cu130 drop 2
    ///   zen template edit ml-cu130 add numpy --step 1
    ///   zen template edit ml-cu130 add bb --wheel /path/to/bb.whl
    ///
    /// Interactive mode (no action → enters recording session):
    ///   zen template edit ml-cu130
    Edit {
        /// Template name (e.g., ml-cu130 or ml-cu130:latest)
        name: String,
        /// Action: "add" or "drop"
        action: Option<String>,
        /// Target package name, step number, or packages to add
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        /// Step number to add to (inherits install_args from existing step)
        #[arg(long)]
        step: Option<i64>,
        /// Wheel file path for the package being added
        #[arg(long)]
        wheel: Option<String>,
        /// Custom index URL
        #[arg(long, name = "index-url")]
        index_url: Option<String>,
    },
    /// Drop a package or step from the current recording session
    Drop {
        /// Package name or step number to remove
        target: String,
    },
    /// Export a template to a portable TOML file
    ///
    /// Examples:
    ///   zen template export ml-base               # writes ml-base.toml
    ///   zen template export ml-base -o custom.toml
    #[clap(name = "export")]
    ExportTpl {
        /// Template name (e.g., ml-base or ml-base:v2)
        name: String,
        /// Output file path (default: <name>.toml)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import a template from a TOML file
    ///
    /// Examples:
    ///   zen template import ml-base.toml
    #[clap(name = "import")]
    ImportTpl {
        /// Path to TOML file
        file: String,
    },
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
        let ops = crate::ops::ZenOps::new(&db, cli.home.clone(), crate::context::OutputMode::Cli);
        match command {
            Commands::Create {
                name,
                python: user_python,
                template,
                strict,
                ml,
                cuda,
                rm,
                rest,
            } => {
                commands::create::run(
                    &db,
                    &ops,
                    name,
                    user_python,
                    template,
                    strict,
                    ml,
                    cuda,
                    rm,
                    rest,
                    &cli.home,
                )?;
            }
            Commands::Add { path, name } => {
                commands::add::run(&db, path, name)?;
            }
            Commands::Rename { old, new } => {
                commands::rename::run(&db, &old, &new)?;
            }
            Commands::List {
                pattern,
                sort,
                label,
                format,
                oneline,
                long_format,
            } => {
                let sort_str = match sort {
                    ListSort::Name => "name",
                    ListSort::Date => "date",
                };
                let list_format = match format {
                    ListFormatArg::Minimal => commands::list::ListFormat::Minimal,
                    ListFormatArg::Compact => commands::list::ListFormat::Compact,
                    ListFormatArg::Wide => commands::list::ListFormat::Wide,
                    ListFormatArg::Auto => {
                        use terminal_size::{Width, terminal_size};
                        match terminal_size() {
                            Some((Width(w), _)) if w < 200 => commands::list::ListFormat::Minimal,
                            Some(_) => commands::list::ListFormat::Compact,
                            None => commands::list::ListFormat::Minimal,
                        }
                    }
                };
                commands::list::run(
                    &ops,
                    &db,
                    &cli.home,
                    pattern,
                    sort_str,
                    label,
                    list_format,
                    oneline,
                    long_format,
                )?;
            }
            Commands::Rm { name, yes, cached } => {
                commands::rm::run(&ops, &db, &name, yes, cached, &cli.home)?;
            }
            Commands::Config { key, value } => {
                commands::config::run(&db, key, value)?;
            }
            Commands::Reset { yes } => {
                commands::reset::run(yes)?;
            }
            Commands::Template { subcommand } => match subcommand {
                TemplateCommands::Create {
                    name,
                    python: user_python,
                } => {
                    commands::template::run_create(&db, name, user_python)?;
                }
                TemplateCommands::Save => {
                    commands::template::run_save(&db)?;
                }
                TemplateCommands::Exit => {
                    commands::template::run_exit(&db)?;
                }
                TemplateCommands::List { name } => {
                    commands::template::run_list(&db, name)?;
                }
                TemplateCommands::Rm { name } => {
                    commands::template::run_rm(&db, &name)?;
                }
                TemplateCommands::Update { name: _ } => {
                    println!("Template update is not yet implemented.");
                }
                TemplateCommands::Inspect { name } => {
                    commands::template::run_inspect(&db, &name)?;
                }
                TemplateCommands::Edit {
                    name,
                    action,
                    args,
                    step,
                    wheel,
                    index_url,
                } => {
                    commands::template::run_edit(&db, &name, action, args, step, wheel, index_url)?;
                }
                TemplateCommands::Drop { target } => {
                    commands::template::run_drop(&db, &target)?;
                }
                TemplateCommands::ExportTpl { name, output } => {
                    commands::template::run_export_tpl(&db, &name, output)?;
                }
                TemplateCommands::ImportTpl { file } => {
                    commands::template::run_import_tpl(&db, &file)?;
                }
            },
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
                let db_ref = &db;
                commands::install::run(
                    &db,
                    &packages,
                    env,
                    cli_index_url,
                    extra_index_url,
                    editable,
                    pre,
                    upgrade,
                    dry_run,
                    move || resolve_env_name(None, db_ref),
                )?;
            }
            Commands::Run { name, command } => {
                commands::run::run(&ops, &name, command)?;
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

                commands::uninstall::run(&ops, &db, &env_name, packages)?;
            }
            Commands::Info { name } => {
                let name = resolve_env_name(name, &db)?;
                commands::info::run(&ops, &name)?;
            }
            Commands::Status => {
                let db_path = cli.db_path.clone().unwrap_or_else(|| {
                    let home = home::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                    home.join(".config").join("zen").join("zen.db")
                });
                commands::status::run(&ops, &db, &cli.home, &db_path)?;
            }

            Commands::Export { file } => {
                commands::export::run(&db, file)?;
            }
            Commands::Import { file } => {
                commands::import::run(&db, file)?;
            }
            Commands::Setup { subcommand } => match subcommand {
                SetupCommands::Init { path, yes } => {
                    commands::setup::run_init(&ops, path, yes)?;
                }
                SetupCommands::StackInfo => {
                    commands::setup::run_stack_info(&db)?;
                }
            },
            Commands::Note { subcommand } | Commands::Comment { subcommand } => match subcommand {
                NoteCommands::Add { env, message } => {
                    let env = resolve_env_name(env, &db)?;
                    let env_name = types::EnvName::new(&env).map_err(|e| e.to_string())?;
                    commands::note::add(&ops, &env_name, &message)?;
                }
                NoteCommands::List { env, all } => {
                    let env_filter = if all {
                        None
                    } else {
                        let env = resolve_env_name(env, &db)?;
                        Some(types::EnvName::new(&env).map_err(|e| e.to_string())?)
                    };
                    commands::note::list(&ops, env_filter.as_ref())?;
                }
                NoteCommands::Rm { uuid } => {
                    commands::note::rm(&ops, &uuid)?;
                }
            },

            Commands::Label { subcommand } => match subcommand {
                LabelCommands::Add { env, label } => {
                    let env = resolve_env_name(env, &db)?;
                    commands::label::add(&db, &env, &label)?;
                }
                LabelCommands::Rm { env, label } => {
                    let env = resolve_env_name(env, &db)?;
                    commands::label::rm(&db, &env, &label)?;
                }
                LabelCommands::List { env, all } => {
                    let env_resolved = if !all {
                        Some(resolve_env_name(env, &db)?)
                    } else {
                        None
                    };
                    commands::label::list(&db, env_resolved.as_deref(), all)?;
                }
            },
            Commands::Find { package, exact } => {
                commands::find::run(&db, &package, exact)?;
            }
            Commands::Inspect {
                env,
                package,
                names_only,
                long,
            } => {
                let env = resolve_env_name(env, &db)?;
                commands::inspect::run(&db, &env, package, names_only, long)?;
            }
            Commands::Diff {
                env1,
                env2,
                only_diff,
            } => {
                commands::diff::run(&db, &env1, &env2, only_diff)?;
            }
            Commands::Health { name } => {
                let name = resolve_env_name(name, &db)?;
                let env_name = types::EnvName::new(&name).map_err(|e| e.to_string())?;
                commands::health::run(&ops, &env_name)?;
            }
            Commands::Activate {
                name,
                path_only,
                last,
            } => {
                commands::activate::run(&db, name, path_only, last)?;
            }
            Commands::Hook { shell } => {
                print!("{}", crate::hooks::generate_hook(&shell));
            }
            Commands::Clone { source, name } => {
                commands::clone::run(&db, &source, &name, &cli.home)?;
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
                    commands::link::run_add(&db, &name, path)?;
                }
                LinkCommands::Rm { name, path } => {
                    let name = resolve_env_name(name, &db)?;
                    commands::link::run_rm(&db, &name, path)?;
                }
                LinkCommands::List { path } => {
                    commands::link::run_list(&db, path)?;
                }
                LinkCommands::Prune => {
                    commands::link::run_prune(&db)?;
                }
                LinkCommands::Reset {
                    path,
                    activations,
                    history,
                    older_than,
                } => {
                    commands::link::run_reset(&db, path, activations, history, older_than)?;
                }
            },
            Commands::Log {
                filter,
                lines,
                clear,
            } => {
                commands::log::run(filter, lines, clear)?;
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
