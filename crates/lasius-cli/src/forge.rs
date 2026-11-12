//! `lasius forge …` — forge verbs over the [`lasius_forge`] port.
//! Today the only impl is [`lasius_forge_gh::GhForge`]; the
//! `provider` key in `lasius.toml` selects the backend, same seam as
//! repo discovery ([`crate::providers`]).
//!
//! The `repo` argument resolves via [`crate::support::repo_dir`] —
//! clone name or any checkout path (a PR worktree shares its
//! repo's remote, so `gh` resolves owner/repo from it unchanged).

use anyhow::Result;
use lasius_core::ProviderKind;
use lasius_forge::{
    CheckRun, CreatePr, CreateStatus, Forge, MergeMethod, PrState, PrStateFilter, PullRequest,
};
use lasius_forge_gh::GhForge;

use crate::support::{self, Workspace};

/// The configured forge backend. Only `github` exists today; new
/// `ProviderKind`s add their impl here.
fn forge(ws: &Workspace) -> Box<dyn Forge> {
    match ws.config.provider {
        ProviderKind::Github => Box::new(GhForge::new()),
    }
}

fn state_label(s: PrState) -> &'static str {
    match s {
        PrState::Open => "open",
        PrState::Closed => "closed",
        PrState::Merged => "merged",
        PrState::Unknown => "unknown",
    }
}

fn print_pr(pr: &PullRequest) {
    println!(
        "#{} [{}] {} — {} ({} → {})",
        pr.number,
        state_label(pr.state),
        pr.title,
        pr.url,
        pr.head_ref,
        pr.base_ref
    );
}

fn print_check(c: &CheckRun) {
    println!(
        "{:<8} {:<28} {}",
        c.bucket(),
        c.name,
        c.url.as_deref().unwrap_or("")
    );
}

/// `lasius forge pr create <repo>` — open a PR from the checkout's
/// head (or `req.head`) against `req.base`.
pub fn pr_create(ws: &Workspace, repo: &str, req: CreatePr, json: bool) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let pr = forge(ws).create_pr(&dir, &req)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&pr)?);
    } else {
        println!("created PR #{} {}", pr.number, pr.url);
    }
    Ok(())
}

/// `lasius forge pr merge <repo> <number>` — merge with `--method`
/// (default squash), optionally `--delete-branch`.
pub fn pr_merge(
    ws: &Workspace,
    repo: &str,
    number: u64,
    method: MergeMethod,
    delete_branch: bool,
    json: bool,
) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    forge(ws).merge_pr(&dir, number, method, delete_branch)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "merged": true,
                "number": number,
                "method": method,
                "delete_branch": delete_branch,
            }))?
        );
    } else {
        println!("merged PR #{number}");
    }
    Ok(())
}

/// `lasius forge pr view <repo> <number>` — full PR record.
pub fn pr_view(ws: &Workspace, repo: &str, number: u64, json: bool) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let pr = forge(ws).view_pr(&dir, number)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&pr)?);
    } else {
        print_pr(&pr);
        if let Some(state) = &pr.merge_state {
            println!("merge-state: {state}");
        }
        if let Some(review) = &pr.review_decision {
            println!("review: {review}");
        }
    }
    Ok(())
}

/// `lasius forge pr checks <repo> <number>` — CI checks on the head.
pub fn pr_checks(ws: &Workspace, repo: &str, number: u64, json: bool) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let checks = forge(ws).pr_checks(&dir, number)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        for c in &checks {
            print_check(c);
        }
        if checks.is_empty() {
            println!("no checks reported");
        }
    }
    Ok(())
}

/// `lasius forge pr list <repo>` — PRs filtered by `--state`.
pub fn pr_list(
    ws: &Workspace,
    repo: &str,
    state: PrStateFilter,
    limit: usize,
    json: bool,
) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let prs = forge(ws).list_prs(&dir, state, limit)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&prs)?);
    } else {
        for pr in &prs {
            print_pr(pr);
        }
        if prs.is_empty() {
            println!("no pull requests");
        }
    }
    Ok(())
}

/// `lasius forge status create <repo>` — post a commit status on a
/// sha (`--state pending|success|failure|error`, `--context`).
pub fn status_create(ws: &Workspace, repo: &str, req: CreateStatus, json: bool) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let st = forge(ws).create_status(&dir, &req)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&st)?);
    } else {
        println!("status {} {} on {}", st.state, st.context, req.sha);
    }
    Ok(())
}
