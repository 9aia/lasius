//! `lasius vcs …` — VCS mutation verbs over the [`lasius_vcs`] port
//! (git impl: [`lasius_vcs_git::GitVcs`]). These are the building
//! blocks agentic campaigns (e.g. `pr-heal`) compose: worktree at a
//! rev, edit, commit, push — no raw `git` strings in campaign YAML.
//!
//! The `repo` argument resolves via [`crate::support::repo_dir`]: a
//! lasius clone name or any existing directory (worktrees included).

use std::path::PathBuf;

use anyhow::{Context, Result};
use lasius_vcs::Vcs;
use lasius_vcs_git::GitVcs;

use crate::support::{self, Workspace};

fn vcs() -> GitVcs {
    GitVcs
}

/// `lasius vcs worktree add <repo>` — new worktree at `--rev`
/// (default HEAD), optionally on a new `--branch`.
pub fn worktree_add(
    ws: &Workspace,
    repo: &str,
    dir: Option<PathBuf>,
    branch: Option<String>,
    rev: Option<String>,
    json: bool,
) -> Result<()> {
    let repo_dir = support::repo_dir(ws, repo)?;
    // Default location: `<ws>/worktrees/<repo>/<branch|rev|HEAD>` —
    // outside every checkout so `git add -A` never sweeps it up.
    let dir = dir.unwrap_or_else(|| {
        let name = branch
            .clone()
            .or_else(|| rev.clone())
            .unwrap_or_else(|| "HEAD".to_string());
        let owner = repo_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "repo".to_string());
        ws.root.join("worktrees").join(owner).join(name)
    });
    // A relative --dir would resolve inside the repo (git -C cwd) —
    // anchor it to the caller's cwd instead.
    let dir = if dir.is_relative() {
        std::env::current_dir()
            .map(|cwd| cwd.join(&dir))
            .unwrap_or(dir)
    } else {
        dir
    };
    let v = vcs();
    let res: std::io::Result<()> = match &branch {
        Some(b) => v.worktree_add_branch(&repo_dir, &dir, b, rev.as_deref()),
        None => {
            let r = rev.as_deref().unwrap_or("HEAD");
            if v.worktree_add(&repo_dir, &dir, r) {
                Ok(())
            } else {
                Err(std::io::Error::other(format!(
                    "git worktree add failed for {}",
                    dir.display()
                )))
            }
        }
    };
    res.with_context(|| format!("worktree add {}", dir.display()))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "repo": repo_dir,
                "dir": dir,
                "branch": branch,
                "rev": rev,
            }))?
        );
    } else {
        println!("worktree {}", dir.display());
    }
    Ok(())
}

/// `lasius vcs worktree remove <repo>` — remove the worktree at `--dir`.
pub fn worktree_remove(ws: &Workspace, repo: &str, dir: PathBuf, json: bool) -> Result<()> {
    let repo_dir = support::repo_dir(ws, repo)?;
    if !vcs().worktree_remove(&repo_dir, &dir) {
        anyhow::bail!("git worktree remove failed for {}", dir.display());
    }
    if json {
        println!("{}", serde_json::json!({"removed": dir}));
    } else {
        println!("removed worktree {}", dir.display());
    }
    Ok(())
}

/// `lasius vcs commit <repo>` — stage everything and commit.
pub fn commit(
    ws: &Workspace,
    repo: &str,
    message: &str,
    body: Option<String>,
    allow_empty: bool,
    json: bool,
) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let v = vcs();
    v.stage_all(&dir).context("git add -A")?;
    v.commit(&dir, message, body.as_deref(), allow_empty)
        .context("git commit")?;
    let head = v.head(&dir).unwrap_or_default();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "repo": dir,
                "sha": head,
                "subject": message,
            }))?
        );
    } else {
        println!("committed {head} {message}");
    }
    Ok(())
}

/// `lasius vcs push <repo>` — push `--branch` (default: checked-out
/// branch) to `--remote` (default origin), setting upstream.
pub fn push(
    ws: &Workspace,
    repo: &str,
    remote: Option<String>,
    branch: Option<String>,
    json: bool,
) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let v = vcs();
    let remote = remote.unwrap_or_else(|| "origin".to_string());
    let branch = match branch {
        Some(b) => Some(b),
        None => v.current_branch(&dir),
    };
    v.push_branch(&dir, &remote, branch.as_deref())
        .with_context(|| format!("git push {remote}"))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "repo": dir,
                "remote": remote,
                "branch": branch,
            }))?
        );
    } else {
        println!("pushed {} to {}", branch.unwrap_or_default(), remote);
    }
    Ok(())
}

/// `lasius vcs fetch <repo>` — fetch `--remote` (default origin),
/// optionally limited to `--refspec` entries (repeatable).
pub fn fetch(
    ws: &Workspace,
    repo: &str,
    remote: Option<String>,
    refspecs: &[String],
    json: bool,
) -> Result<()> {
    let dir = support::repo_dir(ws, repo)?;
    let remote = remote.unwrap_or_else(|| "origin".to_string());
    let specs: Vec<&str> = refspecs.iter().map(String::as_str).collect();
    vcs()
        .fetch(&dir, &remote, &specs)
        .with_context(|| format!("git fetch {remote}"))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "repo": dir,
                "remote": remote,
                "refspecs": refspecs,
            }))?
        );
    } else {
        println!("fetched {remote}");
    }
    Ok(())
}
