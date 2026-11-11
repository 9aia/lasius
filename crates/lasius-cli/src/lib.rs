//! lasius — the engine as a library. Everything that used to live
//! in the binary lives here: [`Cli`]/[`Cmd`], workspace resolution,
//! verb dispatch. `run()`/`run_from()` are the whole CLI body;
//! drivers split it into [`parse`]/[`parse_from`] + [`dispatch`] —
//! argv in, exit code out. There is no `lasius` cargo bin: the
//! `lasius` command ships as the tamias pack in `libraries/lasius/`
//! (T-012) — a generated `bins[]` shim → `tamias-driver-lasius cli`
//! → [`run_from`] in-process.
//!
//! lasius — workspace gardener for GitHub orgs.
//!
//! Clone, sync, inspect, and generate editor workspace files for every repo
//! of a GitHub org under one root. Named for *Lasius niger*, the black garden
//! ant — it tends the garden so you don't have to.

mod commands;
mod forge;
mod proc;
mod providers;
mod serve;
mod support;
mod vcs;

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
pub struct Cli {
    /// Workspace root containing lasius.toml (default: discovered upward from cwd)
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
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
    /// VCS verbs — worktrees, commits, fetch/push over the lasius-vcs port
    Vcs {
        #[command(subcommand)]
        action: VcsAction,
    },
    /// Forge verbs — pull requests and checks over the lasius-forge port
    Forge {
        #[command(subcommand)]
        action: ForgeAction,
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
enum VcsAction {
    /// Worktree commands
    Worktree {
        #[command(subcommand)]
        action: VcsWorktreeAction,
    },
    /// Stage all changes and commit in a checkout/worktree
    Commit {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// Commit subject (`-m`)
        #[arg(short = 'm', long)]
        message: String,
        /// Commit body — second paragraph (`--body`)
        #[arg(long)]
        body: Option<String>,
        /// Allow committing with no staged changes (CI retrigger)
        #[arg(long)]
        allow_empty: bool,
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
    /// Push a branch to a remote (default: current branch → origin)
    Push {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// Remote name (default: origin)
        #[arg(long)]
        remote: Option<String>,
        /// Branch to push (default: the branch checked out at repo)
        #[arg(long)]
        branch: Option<String>,
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
    /// Fetch a remote (optionally specific refspecs via --refspec)
    Fetch {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// Remote name (default: origin)
        #[arg(long)]
        remote: Option<String>,
        /// Refspec to fetch (repeatable; empty = remote's configured refspecs)
        #[arg(long)]
        refspec: Vec<String>,
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum VcsWorktreeAction {
    /// Create a worktree at --rev (default HEAD), optionally on a new --branch
    Add {
        /// Clone name under repos_dir, or a checkout path
        repo: String,
        /// Worktree path (default: <ws>/worktrees/<repo>/<branch|rev|HEAD>)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// New branch to create and check out in the worktree
        #[arg(long)]
        branch: Option<String>,
        /// Revision/sha to base the worktree on (default: HEAD)
        #[arg(long)]
        rev: Option<String>,
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
    /// Remove a worktree (--dir is the worktree path)
    Remove {
        /// Clone name under repos_dir, or the owning checkout path
        repo: String,
        /// Worktree directory to remove
        #[arg(long)]
        dir: PathBuf,
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ForgeAction {
    /// Pull-request commands
    Pr {
        #[command(subcommand)]
        action: ForgePrAction,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum MergeMethodArg {
    Squash,
    Merge,
    Rebase,
}

impl From<MergeMethodArg> for lasius_forge::MergeMethod {
    fn from(m: MergeMethodArg) -> Self {
        match m {
            MergeMethodArg::Squash => Self::Squash,
            MergeMethodArg::Merge => Self::Merge,
            MergeMethodArg::Rebase => Self::Rebase,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum PrStateArg {
    Open,
    Closed,
    Merged,
    All,
}

impl From<PrStateArg> for lasius_forge::PrStateFilter {
    fn from(s: PrStateArg) -> Self {
        match s {
            PrStateArg::Open => Self::Open,
            PrStateArg::Closed => Self::Closed,
            PrStateArg::Merged => Self::Merged,
            PrStateArg::All => Self::All,
        }
    }
}

#[derive(Subcommand)]
enum ForgePrAction {
    /// Open a pull request (--title/--body; --base/--head optional)
    Create {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// PR title
        #[arg(long)]
        title: String,
        /// PR body
        #[arg(long, default_value = "")]
        body: String,
        /// Base branch (default: repo default)
        #[arg(long)]
        base: Option<String>,
        /// Head branch/ref (default: the checkout's current branch)
        #[arg(long)]
        head: Option<String>,
        /// Open as a draft PR
        #[arg(long)]
        draft: bool,
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
    /// Merge a pull request (--method, default squash; --delete-branch)
    Merge {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// Pull request number
        number: u64,
        /// Merge strategy
        #[arg(long, value_enum, default_value = "squash")]
        method: MergeMethodArg,
        /// Delete the head branch after merging
        #[arg(long)]
        delete_branch: bool,
        /// Emit JSON instead of the summary line
        #[arg(long)]
        json: bool,
    },
    /// Show one pull request (state, refs, mergeability, review)
    View {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// Pull request number
        number: u64,
        /// Emit JSON instead of the summary
        #[arg(long)]
        json: bool,
    },
    /// List CI checks / status contexts on a PR's head
    Checks {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// Pull request number
        number: u64,
        /// Emit JSON instead of the table
        #[arg(long)]
        json: bool,
    },
    /// List pull requests (--state, --limit)
    List {
        /// Clone name under repos_dir, or a checkout/worktree path
        repo: String,
        /// State filter
        #[arg(long, value_enum, default_value = "open")]
        state: PrStateArg,
        /// Max PRs to list (0 = backend default)
        #[arg(long, default_value_t = 0)]
        limit: usize,
        /// Emit JSON instead of the table
        #[arg(long)]
        json: bool,
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

/// Process argv → a parsed [`Cli`]. Split from [`dispatch`] so
/// drivers can parse *their* argv (a `run` verb, a re-exec)
/// without owning process args. Clap errors/help still exit the
/// process, same as argv parsing.
pub fn parse() -> Cli {
    Cli::parse()
}

/// Same, from an argv iterator — `parse_from(["lasius", "status"])`.
pub fn parse_from<I, T>(itr: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::parse_from(itr)
}

/// The clap verb tree as a [`clap::Command`] — the ops manifest
/// derives itself from this, so a verb can't exist on the CLI and
/// be missing from the driver's wire surface.
pub fn command() -> clap::Command {
    <Cli as clap::CommandFactory>::command()
}

/// One parsed command → exit code. The dispatch every driver
/// shares; argv parsing is a caller concern.
pub fn dispatch(cli: &Cli) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if let Cmd::Completions { shell } = &cli.command {
        let mut cmd = command();
        clap_complete::generate(*shell, &mut cmd, "lasius", &mut std::io::stdout());
        return 0;
    }

    let result = match &cli.command {
        Cmd::Init { org } => commands::init(cli.root.as_deref(), org.clone(), &cwd),
        command => {
            let ws = match support::resolve(cli.root.as_deref(), &cwd) {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("error: {e:#}");
                    return 1;
                }
            };
            match command {
                Cmd::Clone => commands::clone(&ws),
                Cmd::Update { json } => commands::update(&ws, *json),
                Cmd::Status { json } => commands::status(&ws, *json),
                Cmd::CodeWorkspace {
                    action: CodeWorkspaceAction::Generate { json },
                } => commands::code_workspace_generate(&ws, *json),
                Cmd::Doctor { json } => commands::doctor(&ws, *json),
                Cmd::Check { domain, only, json } => {
                    commands::check(&ws, domain.as_deref(), only, *json)
                }
                Cmd::Open { editor } => commands::open(&ws, editor.clone()),
                Cmd::Rm { name, yes, force } => commands::rm(&ws, name, *yes, *force),
                Cmd::Exec { filter, cmd } => commands::exec(&ws, filter.as_deref(), cmd),
                Cmd::Add { name } => commands::add(&ws, name),
                Cmd::Vcs { action } => match action {
                    VcsAction::Worktree { action } => match action {
                        VcsWorktreeAction::Add {
                            repo,
                            dir,
                            branch,
                            rev,
                            json,
                        } => vcs::worktree_add(
                            &ws,
                            repo,
                            dir.clone(),
                            branch.clone(),
                            rev.clone(),
                            *json,
                        ),
                        VcsWorktreeAction::Remove { repo, dir, json } => {
                            vcs::worktree_remove(&ws, repo, dir.clone(), *json)
                        }
                    },
                    VcsAction::Commit {
                        repo,
                        message,
                        body,
                        allow_empty,
                        json,
                    } => vcs::commit(&ws, repo, message, body.clone(), *allow_empty, *json),
                    VcsAction::Push {
                        repo,
                        remote,
                        branch,
                        json,
                    } => vcs::push(&ws, repo, remote.clone(), branch.clone(), *json),
                    VcsAction::Fetch {
                        repo,
                        remote,
                        refspec,
                        json,
                    } => vcs::fetch(&ws, repo, remote.clone(), refspec, *json),
                },
                Cmd::Forge { action } => match action {
                    ForgeAction::Pr { action } => match action {
                        ForgePrAction::Create {
                            repo,
                            title,
                            body,
                            base,
                            head,
                            draft,
                            json,
                        } => forge::pr_create(
                            &ws,
                            repo,
                            lasius_forge::CreatePr {
                                title: title.clone(),
                                body: body.clone(),
                                base: base.clone(),
                                head: head.clone(),
                                draft: *draft,
                            },
                            *json,
                        ),
                        ForgePrAction::Merge {
                            repo,
                            number,
                            method,
                            delete_branch,
                            json,
                        } => forge::pr_merge(
                            &ws,
                            repo,
                            *number,
                            (*method).into(),
                            *delete_branch,
                            *json,
                        ),
                        ForgePrAction::View { repo, number, json } => {
                            forge::pr_view(&ws, repo, *number, *json)
                        }
                        ForgePrAction::Checks { repo, number, json } => {
                            forge::pr_checks(&ws, repo, *number, *json)
                        }
                        ForgePrAction::List {
                            repo,
                            state,
                            limit,
                            json,
                        } => forge::pr_list(&ws, repo, (*state).into(), *limit, *json),
                    },
                },
                Cmd::Serve => serve::serve(&ws),
                Cmd::Completions { .. } => unreachable!(),
                Cmd::Init { .. } => unreachable!(),
            }
        }
    };

    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e:#}");
            1
        }
    }
}

/// The compat CLI body — parse process argv, dispatch, hand back
/// the exit code.
pub fn run() -> i32 {
    run_from(std::env::args_os())
}

/// Same, from an argv iterator — the `run` passthrough entry
/// drivers re-exec (`tamias-driver-lasius run <lasius argv>`).
pub fn run_from<I, T>(argv: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = parse_from(argv);
    dispatch(&cli)
}
