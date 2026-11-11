//! lasius-vcs — the shared VCS port (P-019). One trait covering
//! the union of what five repos' `*-vcs` crates each re-declared:
//! history queries (tarsi), worktree ops (mimus/bupha/kuda), tag
//! ops, and sync/push (sturn's coordination flavor, generalized).
//!
//! Pure types + trait, no I/O — impls (`lasius-vcs-git`, later
//! jj/fossil) live in their own crates. Consumers keep a thin
//! domain trait only where semantics genuinely differ.

use std::io;
use std::path::{Path, PathBuf};

/// A commit as history queries report it — enough for changelog
/// generation (tarsi) and verification (mimus) without leaking
/// backend syntax.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    /// First line of the message.
    pub subject: String,
    /// Remaining body (empty when absent).
    #[serde(default)]
    pub body: String,
    /// Author name + email as the backend reports them.
    #[serde(default)]
    pub author: String,
    /// Committer timestamp, unix seconds.
    #[serde(default)]
    pub timestamp: i64,
    /// Repo-relative paths touched (empty = backend didn't report).
    #[serde(default)]
    pub paths: Vec<String>,
}

/// One cheap `log`-style entry — `%H %as %s`, newest first.
/// Lighter than [`CommitInfo`] (no body/paths); for ledger-style
/// consumers that only need hash/date/subject.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    /// Full commit hash.
    pub hash: String,
    /// Author date, `YYYY-MM-DD`.
    pub date: String,
    /// Subject line.
    pub subject: String,
}

/// Per-file line delta from `diff_stat`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffStat {
    pub path: String,
    pub added: u64,
    pub deleted: u64,
}

/// The VCS port. All methods take the repo root; `None`/`Err`
/// conventions are per-method (bool = success probe, Option =
/// lookup miss).
pub trait Vcs: Send + Sync {
    // -- objects / history -------------------------------------------------

    /// Current HEAD sha.
    fn head(&self, repo: &Path) -> Option<String>;
    /// Resolve a revision (`HEAD`, tag, sha, `main~3`) to an id.
    fn resolve(&self, repo: &Path, rev: &str) -> Option<String>;
    /// Does the object exist locally?
    fn has_commit(&self, repo: &Path, sha: &str) -> bool;
    /// Commits reachable from `rev` (e.g. `"HEAD"`, `"v1.0.0..HEAD"`),
    /// newest first, capped at `limit` (0 = backend default/all).
    /// `paths` are populated when the backend reports them cheaply.
    fn commits(&self, repo: &Path, rev: &str, limit: usize) -> Vec<CommitInfo>;
    /// Per-file added/deleted counts between two revisions.
    fn diff_stat(&self, repo: &Path, from: &str, to: &str) -> Vec<DiffStat>;
    /// All tag names (annotated + lightweight), sorted.
    fn tags(&self, repo: &Path) -> Vec<String>;
    /// `git describe`-style: nearest tag + distance, or sha fallback.
    fn describe(&self, repo: &Path) -> Option<String>;
    /// Cheap newest-first listing of `rev` — hash/date/subject only
    /// (no per-commit diff pass, unlike [`Vcs::commits`]). Empty on
    /// unborn HEAD, unresolvable `rev`, or backend failure.
    fn log(&self, repo: &Path, rev: &str, limit: usize) -> Vec<LogEntry>;

    // -- repo discovery ---------------------------------------------------

    /// Is `start` inside a backend work tree? `false` on any
    /// failure — including a missing backend binary; use
    /// [`Vcs::exec_out`] when "not a repo" must be distinguished
    /// from "backend broken".
    fn is_git_repo(&self, start: &Path) -> bool;
    /// Worktree root containing `start`
    /// (`rev-parse --show-toplevel`); `None` outside a repo.
    fn repo_root(&self, start: &Path) -> Option<PathBuf>;

    // -- branches ---------------------------------------------------------

    /// `refs/heads/<name>` exists.
    fn branch_exists(&self, repo: &Path, name: &str) -> bool;
    /// `git branch <name> <start>` — create a branch without
    /// switching to it.
    fn branch(&self, repo: &Path, name: &str, start: &str) -> bool;
    /// Create `name` as an orphan branch via plumbing — an empty
    /// root commit, create-only `update-ref` (an existing `name` is
    /// an error, never a reset). Never disturbs the index or
    /// working tree of the current checkout.
    fn create_orphan_branch(&self, repo: &Path, name: &str, msg: &str) -> io::Result<()>;

    // -- worktrees ---------------------------------------------------------

    /// Detached worktree of `sha`/`rev` at `dir`.
    fn worktree_add(&self, repo: &Path, dir: &Path, rev: &str) -> bool;
    /// Worktree at `dir` on a *new* branch `branch` created at `rev`
    /// (`None` = HEAD). Errors carry the backend's stderr so callers
    /// can report why (missing rev, existing branch, …).
    fn worktree_add_branch(
        &self,
        repo: &Path,
        dir: &Path,
        branch: &str,
        rev: Option<&str>,
    ) -> io::Result<()>;
    /// Worktree at `dir` on the *existing* branch `branch`
    /// (`git worktree add <dir> <branch>` — fails when another
    /// worktree holds it). Unlike [`Vcs::worktree_add_branch`] this
    /// does not create the branch.
    fn worktree_add_existing(&self, repo: &Path, dir: &Path, branch: &str) -> io::Result<()>;
    fn worktree_remove(&self, repo: &Path, dir: &Path) -> bool;
    /// GC stale administrative worktree entries.
    fn worktree_prune(&self, repo: &Path) -> bool;
    /// Shared git dir for all of `repo`'s worktrees
    /// (`<main>/.git`, resolved from inside any of them);
    /// `None` outside a repo or on a bare repo.
    fn common_dir(&self, repo: &Path) -> Option<PathBuf>;
    /// The main (first) worktree of the repo `repo` belongs to —
    /// `repo` itself when it is the main checkout. Falls back to
    /// `worktree list`'s first entry where the backend lacks
    /// `--path-format`.
    fn main_worktree(&self, repo: &Path) -> Option<PathBuf>;

    // -- working tree / commits (agent-facing mutations) -------------------

    /// Branch checked out at `repo`; `None` when detached or unknown.
    fn current_branch(&self, repo: &Path) -> Option<String>;
    /// Stage every change (tracked + untracked) under `repo`.
    fn stage_all(&self, repo: &Path) -> io::Result<()>;
    /// Commit staged changes. `body` becomes a second `-m` paragraph;
    /// `allow_empty` permits a commit with no staged diff (CI retrigger).
    fn commit(
        &self,
        repo: &Path,
        subject: &str,
        body: Option<&str>,
        allow_empty: bool,
    ) -> io::Result<()>;
    /// Uncommitted/untracked changes under `repo`
    /// (`status --porcelain` scoped to `repo` itself). `Err` on
    /// backend failure — "status failed" must never read as clean.
    fn has_changes(&self, repo: &Path) -> io::Result<bool>;
    /// Is the checkout at `repo` on a detached HEAD? `Some(true)`
    /// detached, `Some(false)` on a branch, `None` when HEAD doesn't
    /// resolve (unborn branch, not a checkout — commits there are
    /// still reachable once the branch is born).
    fn is_detached(&self, repo: &Path) -> Option<bool>;
    /// Attach the checkout at `repo` to `name`, keeping its current
    /// HEAD as the branch tip (`checkout -B` — create-or-reset).
    /// The repair for a detached worktree; the backend refuses when
    /// another worktree holds `name`.
    fn attach_branch(&self, repo: &Path, name: &str) -> io::Result<()>;

    // -- divergence / rebase ----------------------------------------------

    /// Commits `HEAD` has that `rev` doesn't; `None` when `rev`
    /// doesn't resolve (no such local or remote ref).
    fn commits_ahead(&self, repo: &Path, rev: &str) -> Option<usize>;
    /// Commits `rev` has that `HEAD` doesn't — the mirror of
    /// [`Vcs::commits_ahead`]; `None` when `rev` doesn't resolve.
    fn commits_behind(&self, repo: &Path, rev: &str) -> Option<usize>;
    /// `merge --ff-only <rev>` — fast-forward HEAD to `rev`.
    /// Fails when the histories diverged (that's [`Vcs::rebase`]'s
    /// job).
    fn merge_ff(&self, repo: &Path, rev: &str) -> io::Result<()>;
    /// `rebase --autostash <rev>` — replay local commits onto
    /// `rev`, stashing any dirty tree first. A conflict stops the
    /// rebase mid-flight ([`Vcs::rebase_in_progress`]) and surfaces
    /// as `Err`.
    fn rebase(&self, repo: &Path, rev: &str) -> io::Result<()>;
    /// `rebase --abort` — bail a stopped rebase, restoring the
    /// pre-rebase tip.
    fn rebase_abort(&self, repo: &Path) -> io::Result<()>;
    /// A rebase or merge is stopped mid-flight in `repo` — the
    /// state dirs / `MERGE_HEAD` exist.
    fn rebase_in_progress(&self, repo: &Path) -> bool;
    /// Worktree-relative paths with unresolved merge conflicts.
    fn unmerged_files(&self, repo: &Path) -> Vec<String>;
    /// Does `remote`'s fetch refspec publish `branch` into
    /// `refs/remotes/<remote>/`? A `--single-branch` clone tracks
    /// only some branches, so `<remote>/<branch>` can sit stale
    /// forever while pushes still succeed.
    fn tracks_branch(&self, repo: &Path, remote: &str, branch: &str) -> bool;

    // -- sync / push -------------------------------------------------------

    /// Fetch + prune so remote refs are current. `remote: None`
    /// fetches the default (`origin`).
    fn sync(&self, repo: &Path, remote: Option<&str>) -> bool;
    /// Fetch `remote` without pruning; empty `refspecs` fetches the
    /// remote's configured refspecs (e.g. all branches).
    fn fetch(&self, repo: &Path, remote: &str, refspecs: &[&str]) -> io::Result<()>;
    /// Push `refspecs` (raw git refspec args) to `remote`.
    /// Empty = push the current branch.
    fn push(&self, repo: &Path, remote: &str, refspecs: &[&str]) -> bool;
    /// Push `branch` (`None` = the branch checked out at `repo`) to
    /// `remote`, setting upstream. Errors carry the backend's stderr.
    fn push_branch(&self, repo: &Path, remote: &str, branch: Option<&str>) -> io::Result<()>;

    // -- tags / refs -------------------------------------------------------

    fn tag_exists(&self, repo: &Path, tag: &str) -> bool;
    /// Create a tag at HEAD (or `rev` when given).
    fn tag(&self, repo: &Path, tag: &str, rev: Option<&str>) -> bool;
    /// Push a single tag to `remote`.
    fn push_tag(&self, repo: &Path, remote: &str, tag: &str) -> bool;
    /// Delete a local tag.
    fn delete_tag(&self, repo: &Path, tag: &str) -> bool;

    // -- raw passthrough ---------------------------------------------------

    /// Backend CLI probe — `args` appended after the repo selector;
    /// exit status only, output silenced. Escape hatch for
    /// operations the typed surface doesn't cover (e.g. arbitrary
    /// plumbing); prefer the typed methods.
    fn exec(&self, repo: &Path, args: &[&str]) -> bool;
    /// Backend CLI → trimmed stdout; stdin is `/dev/null` so
    /// plumbing that reads stdin gets an immediate EOF. `Err`
    /// carries the backend's stderr. `-c key=value` args may precede
    /// the subcommand (e.g. committer-identity injection for
    /// `commit`/`commit-tree` on machines with no configured
    /// identity).
    fn exec_out(&self, repo: &Path, args: &[&str]) -> io::Result<String>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn commit_info_serde_round_trip() {
        let c = CommitInfo {
            sha: "abc123".into(),
            subject: "feat: x".into(),
            body: "details".into(),
            author: "a <a@b>".into(),
            timestamp: 1700000000,
            paths: vec!["src/lib.rs".into()],
        };
        let v = serde_json::to_string(&c).unwrap();
        let back: CommitInfo = serde_json::from_str(&v).unwrap();
        assert_eq!(c, back);

        let e = LogEntry {
            hash: "def456".into(),
            date: "2026-01-01".into(),
            subject: "feat: y".into(),
        };
        let v = serde_json::to_string(&e).unwrap();
        let back: LogEntry = serde_json::from_str(&v).unwrap();
        assert_eq!(e, back);
    }
}
