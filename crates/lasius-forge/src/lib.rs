//! lasius-forge — the forge port. One trait covering what agentic
//! workflows (e.g. the `pr-heal` campaign) need from a forge: open,
//! view, list, and merge pull requests and read their check status —
//! without leaking backend CLI syntax (`gh`, later `glab`, …) to
//! callers.
//!
//! Pure types + trait, no process spawning — impls
//! (`lasius-forge-gh`, later `-gitlab`/`-bitbucket`) live in their own
//! crates and shell the forge's CLI. `gh`-shaped JSON decoding lives
//! in [`gh`] so impls stay thin and the types stay forge-neutral.
//!
//! Every method takes the repo root (`&Path`), same convention as
//! [`lasius_vcs::Vcs`]: the backend resolves owner/repo from the
//! checkout's remote, so worktrees and clones work unchanged.

use std::io;
use std::path::Path;

/// A pull request as forges report it — enough for create/view/list
/// round-trips and check-status reads.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub state: PrState,
    /// Web URL of the PR.
    pub url: String,
    #[serde(default)]
    pub is_draft: bool,
    /// Head branch name (or `owner:branch` for cross-repo heads).
    #[serde(default)]
    pub head_ref: String,
    /// Head commit sha when the backend reports it.
    #[serde(default)]
    pub head_sha: String,
    /// Base branch the PR targets.
    #[serde(default)]
    pub base_ref: String,
    #[serde(default)]
    pub body: String,
    /// Author login when reported.
    #[serde(default)]
    pub author: Option<String>,
    /// Backend mergeability hint (`MERGEABLE`/`CONFLICTING`/… when known).
    #[serde(default)]
    pub mergeable: Option<String>,
    /// Backend merge-state hint (`CLEAN`/`BLOCKED`/`DIRTY`/…).
    #[serde(default)]
    pub merge_state: Option<String>,
    /// Review decision (`APPROVED`/`CHANGES_REQUESTED`/`REVIEW_REQUIRED`).
    #[serde(default)]
    pub review_decision: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub merged_at: Option<String>,
}

/// PR lifecycle state, normalized across backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    Open,
    Closed,
    Merged,
    /// Backend reported a state we don't model.
    Unknown,
}

/// Parameters for [`Forge::create_pr`].
#[derive(Debug, Clone, Default)]
pub struct CreatePr {
    pub title: String,
    pub body: String,
    /// Base branch (backend default — usually the repo's default branch).
    pub base: Option<String>,
    /// Head branch/ref (backend default — the checkout's current branch).
    pub head: Option<String>,
    pub draft: bool,
}

/// Merge strategy for [`Forge::merge_pr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeMethod {
    /// Squash-merge — the default for agent-produced fix branches.
    #[default]
    Squash,
    Merge,
    Rebase,
}

/// State filter for [`Forge::list_prs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrStateFilter {
    #[default]
    Open,
    Closed,
    Merged,
    All,
}

/// What produced a [`CheckRun`] — a CI check run or a commit status
/// context (both appear in rollups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    CheckRun,
    StatusContext,
    #[default]
    Unknown,
}

/// One CI check / status context on a PR head.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CheckRun {
    pub name: String,
    #[serde(default)]
    pub kind: CheckKind,
    /// Raw backend state (`COMPLETED`, `IN_PROGRESS`, `SUCCESS`, …).
    #[serde(default)]
    pub state: String,
    /// Raw backend conclusion (`SUCCESS`/`FAILURE`/`TIMED_OUT`/…),
    /// empty while the check is still running.
    #[serde(default)]
    pub conclusion: Option<String>,
    /// Details/logs URL when reported.
    #[serde(default)]
    pub url: Option<String>,
    /// Workflow/check-suite name when reported.
    #[serde(default)]
    pub workflow: Option<String>,
}

impl CheckRun {
    /// Coarse bucket for display/filtering: `pass`, `fail`, `pending`,
    /// or `skipping` (neutral/skipped/cancelled outcomes).
    pub fn bucket(&self) -> &'static str {
        let c = self.conclusion.as_deref().unwrap_or("");
        match c {
            "SUCCESS" | "NEUTRAL" => return "pass",
            "FAILURE" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE" => return "fail",
            "SKIPPED" | "CANCELLED" | "STALE" => return "skipping",
            _ => {}
        }
        match self.state.as_str() {
            "SUCCESS" => "pass",
            "FAILURE" | "ERROR" | "TIMED_OUT" => "fail",
            _ => "pending",
        }
    }
}

/// State of a commit status — the GitHub statuses API vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusState {
    Pending,
    #[default]
    Success,
    Failure,
    Error,
}

impl StatusState {
    /// Wire spelling (`pending`/`success`/`failure`/`error`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Error => "error",
        }
    }
}

/// Parameters for [`Forge::create_status`] — a commit status on a sha.
#[derive(Debug, Clone)]
pub struct CreateStatus {
    /// Commit sha the status describes.
    pub sha: String,
    pub state: StatusState,
    /// Status context (e.g. `docs/deploy`, `ci/bupha`).
    pub context: String,
    /// Short description (forges truncate ~140 chars).
    pub description: Option<String>,
    /// Link back to the run/logs.
    pub target_url: Option<String>,
}

/// A commit status as the forge reports it after creation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommitStatus {
    #[serde(default)]
    pub id: u64,
    /// Raw backend state (`pending`/`success`/`failure`/`error`).
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub target_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// The forge port. All methods take the repo root; `Err` carries the
/// backend's diagnostic so callers can surface it verbatim.
pub trait Forge: Send + Sync {
    /// Open a PR. Returns the created PR (number + URL at minimum).
    fn create_pr(&self, repo: &Path, req: &CreatePr) -> io::Result<PullRequest>;
    /// Merge PR `number` with `method`, optionally deleting the head
    /// branch afterwards.
    fn merge_pr(
        &self,
        repo: &Path,
        number: u64,
        method: MergeMethod,
        delete_branch: bool,
    ) -> io::Result<()>;
    /// Fetch one PR's full record (title/state/refs/mergeability).
    fn view_pr(&self, repo: &Path, number: u64) -> io::Result<PullRequest>;
    /// CI checks / status contexts on the PR head, in backend order.
    fn pr_checks(&self, repo: &Path, number: u64) -> io::Result<Vec<CheckRun>>;
    /// List PRs filtered by `state`, capped at `limit` (0 = backend
    /// default).
    fn list_prs(
        &self,
        repo: &Path,
        state: PrStateFilter,
        limit: usize,
    ) -> io::Result<Vec<PullRequest>>;
    /// Post a commit status on `req.sha` — the CI verdict egress
    /// (pending while running, success/failure at the end).
    fn create_status(&self, repo: &Path, req: &CreateStatus) -> io::Result<CommitStatus>;
}

pub mod gh;
