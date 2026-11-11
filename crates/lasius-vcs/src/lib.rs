//! lasius-vcs — the shared VCS port (P-019). One trait covering
//! the union of what five repos' `*-vcs` crates each re-declared:
//! history queries (tarsi), worktree ops (mimus/bupha/kuda), tag
//! ops, and sync/push (sturn's coordination flavor, generalized).
//!
//! Pure types + trait, no I/O — impls (`lasius-vcs-git`, later
//! jj/fossil) live in their own crates. Consumers keep a thin
//! domain trait only where semantics genuinely differ.

use std::path::Path;

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

    // -- worktrees ---------------------------------------------------------

    /// Detached worktree of `sha`/`rev` at `dir`.
    fn worktree_add(&self, repo: &Path, dir: &Path, rev: &str) -> bool;
    fn worktree_remove(&self, repo: &Path, dir: &Path) -> bool;
    /// GC stale administrative worktree entries.
    fn worktree_prune(&self, repo: &Path) -> bool;

    // -- tags / refs -------------------------------------------------------

    fn tag_exists(&self, repo: &Path, tag: &str) -> bool;
    /// Create a tag at HEAD (or `rev` when given).
    fn tag(&self, repo: &Path, tag: &str, rev: Option<&str>) -> bool;
    /// Push a single tag to `remote`.
    fn push_tag(&self, repo: &Path, remote: &str, tag: &str) -> bool;
    /// Delete a local tag.
    fn delete_tag(&self, repo: &Path, tag: &str) -> bool;

    // -- sync / push -------------------------------------------------------

    /// Fetch + prune so remote refs are current. `remote: None`
    /// fetches the default (`origin`).
    fn sync(&self, repo: &Path, remote: Option<&str>) -> bool;
    /// Push `refspecs` (raw git refspec args) to `remote`.
    /// Empty = push the current branch.
    fn push(&self, repo: &Path, remote: &str, refspecs: &[&str]) -> bool;
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
    }
}
