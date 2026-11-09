//! lasius — workspace gardener for GitHub orgs.
//!
//! Clone, sync, inspect, and generate editor workspace files for every repo
//! of a GitHub org under one root. Named for *Lasius niger*, the black garden
//! ant — it tends the garden so you don't have to.

mod commands;
mod proc;
mod serve;
mod support;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "lasius",
    version,
    about = "Workspace gardener — manage every repo of a GitHub org under one root",
    long_about = "lasius keeps a directory of independent git clones in sync with a \
                  GitHub org: init bootstraps, clone/update/status maintain, and \
                  code-workspace generates the editor workspace file."
)]
struct Cli {
    /// Workspace root containing lasius.toml (default: discovered upward from cwd)
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold the workspace repo (lasius.toml, .gitignore, doc stubs), clone all org repos, generate the workspace file
    Init {
        /// GitHub org to manage (required when lasius.toml doesn't exist yet)
        #[arg(long)]
        org: Option<String>,
    },
    /// Clone all missing org repos into repos_dir
    Clone,
    /// git pull --ff-only every clone
    #[command(visible_alias = "sync")]
    Update {
        /// Emit JSON instead of the table
        #[arg(long)]
        json: bool,
    },
    /// Show branch / dirty / sync status for every clone
    Status {
        /// Emit JSON instead of the table
        #[arg(long)]
        json: bool,
    },
    /// Workspace-file commands
    #[command(name = "code-workspace")]
    CodeWorkspace {
        #[command(subcommand)]
        action: CodeWorkspaceAction,
    },
    /// Environment + drift diagnostics
    Doctor {
        /// Emit JSON instead of the check table
        #[arg(long)]
        json: bool,
    },
    /// Conformance checks — does the workspace follow the lasius pattern?
    Check {
        /// Check domain to run (structure|clones|generated|policy)
        #[arg(long)]
        domain: Option<String>,
        /// Only run these check ids (repeatable or comma-separated)
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        /// Emit JSON findings instead of the table
        #[arg(long)]
        json: bool,
    },
    /// Open the generated workspace file in an editor
    Open {
        /// Editor command (default: $VISUAL, $EDITOR, or code/zed/cursor/vi)
        #[arg(long)]
        editor: Option<String>,
    },
    /// Remove a clone after safety checks (clean tree, no unpushed commits, …)
    Rm {
        /// Repo name under repos_dir
        name: String,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
        /// Skip all safety checks
        #[arg(long)]
        force: bool,
    },
    /// Run a shell command in every clone
    Exec {
        /// Only run in clones whose name contains this substring
        #[arg(long)]
        filter: Option<String>,
        /// Command + args, after `--`
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        cmd: Vec<String>,
    },
    /// Clone a single org repo by name
    Add {
        /// Repo name
        name: String,
    },
    /// ndjson JSON-RPC over stdio — read-only ops for embedding
    /// (ping, list, status, doctor); same wire as tamias/sepia/cuora/myco serve
    Serve,
    /// Print shell completion script for SHELL
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum CodeWorkspaceAction {
    /// Regenerate the *.code-workspace file from local clones
    Generate {
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = <Cli as clap::CommandFactory>::command();
        clap_complete::generate(*shell, &mut cmd, "lasius", &mut std::io::stdout());
        return;
    }

    let result = match &cli.command {
        Commands::Init { org } => commands::init(cli.root.as_deref(), org.clone(), &cwd),
        command => {
            let ws = match support::resolve(cli.root.as_deref(), &cwd) {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("error: {e:#}");
                    std::process::exit(1);
                }
            };
            match command {
                Commands::Clone => commands::clone(&ws),
                Commands::Update { json } => commands::update(&ws, *json),
                Commands::Status { json } => commands::status(&ws, *json),
                Commands::CodeWorkspace {
                    action: CodeWorkspaceAction::Generate { json },
                } => commands::code_workspace_generate(&ws, *json),
                Commands::Doctor { json } => commands::doctor(&ws, *json),
                Commands::Check { domain, only, json } => {
                    commands::check(&ws, domain.as_deref(), only, *json)
                }
                Commands::Open { editor } => commands::open(&ws, editor.clone()),
                Commands::Rm { name, yes, force } => commands::rm(&ws, name, *yes, *force),
                Commands::Exec { filter, cmd } => commands::exec(&ws, filter.as_deref(), cmd),
                Commands::Add { name } => commands::add(&ws, name),
                Commands::Serve => serve::serve(&ws),
                Commands::Completions { .. } => unreachable!(),
                Commands::Init { .. } => unreachable!(),
            }
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
