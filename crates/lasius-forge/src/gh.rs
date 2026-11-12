//! `gh`-shaped JSON decoding for [`crate::Forge`] payloads — the
//! port-agnostic layer keeps the gh wire format in one place so
//! `lasius-forge-gh` stays a thin process wrapper. Mirrors the
//! `lasius-core::gh` precedent (repo-list decoding).

use serde::Deserialize;

use crate::{CheckKind, CheckRun, CommitStatus, PrState, PullRequest};

/// Fields requested from `gh pr view --json` / `gh pr list --json`.
/// All optional at decode time — `gh` omits empty fields and list
/// payloads carry a subset.
#[derive(Debug, Deserialize)]
struct GhPr {
    #[serde(default)]
    number: u64,
    #[serde(default)]
    title: String,
    /// `OPEN` / `CLOSED` / `MERGED`.
    #[serde(default)]
    state: String,
    #[serde(default)]
    url: String,
    #[serde(rename = "isDraft", default)]
    is_draft: bool,
    #[serde(rename = "headRefName", default)]
    head_ref_name: String,
    #[serde(rename = "headRefOid", default)]
    head_ref_oid: String,
    #[serde(rename = "baseRefName", default)]
    base_ref_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    author: Option<GhAuthor>,
    #[serde(default)]
    mergeable: Option<String>,
    #[serde(rename = "mergeStateStatus", default)]
    merge_state_status: Option<String>,
    #[serde(rename = "reviewDecision", default)]
    review_decision: Option<String>,
    #[serde(rename = "createdAt", default)]
    created_at: Option<String>,
    #[serde(rename = "updatedAt", default)]
    updated_at: Option<String>,
    #[serde(rename = "mergedAt", default)]
    merged_at: Option<String>,
    /// `statusCheckRollup` — CheckRun and StatusContext entries mixed.
    #[serde(rename = "statusCheckRollup", default)]
    status_check_rollup: Vec<GhRollup>,
}

#[derive(Debug, Deserialize)]
struct GhAuthor {
    #[serde(default)]
    login: String,
}

/// One `statusCheckRollup` entry — `__typename` discriminates
/// `CheckRun` (name/status/conclusion/detailsUrl) from
/// `StatusContext` (context/state/targetUrl).
#[derive(Debug, Deserialize)]
struct GhRollup {
    #[serde(rename = "__typename", default)]
    typename: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(rename = "detailsUrl", default)]
    details_url: Option<String>,
    #[serde(rename = "targetUrl", default)]
    target_url: Option<String>,
    #[serde(rename = "workflowName", default)]
    workflow_name: Option<String>,
}

fn pr_state(s: &str) -> PrState {
    match s {
        "OPEN" => PrState::Open,
        "CLOSED" => PrState::Closed,
        "MERGED" => PrState::Merged,
        _ => PrState::Unknown,
    }
}

fn to_pr(g: GhPr) -> PullRequest {
    PullRequest {
        number: g.number,
        title: g.title,
        state: pr_state(&g.state),
        url: g.url,
        is_draft: g.is_draft,
        head_ref: g.head_ref_name,
        head_sha: g.head_ref_oid,
        base_ref: g.base_ref_name,
        body: g.body,
        author: g.author.map(|a| a.login).filter(|l| !l.is_empty()),
        mergeable: g.mergeable,
        merge_state: g.merge_state_status,
        review_decision: g.review_decision,
        created_at: g.created_at,
        updated_at: g.updated_at,
        merged_at: g.merged_at,
    }
}

fn to_check(r: GhRollup) -> CheckRun {
    let is_check_run = r.typename == "CheckRun";
    CheckRun {
        name: r.name.or(r.context).unwrap_or_default(),
        kind: if is_check_run {
            CheckKind::CheckRun
        } else if r.typename == "StatusContext" {
            CheckKind::StatusContext
        } else {
            CheckKind::Unknown
        },
        // CheckRun reports `status` (QUEUED/IN_PROGRESS/COMPLETED);
        // StatusContext reports `state` (PENDING/SUCCESS/FAILURE/…).
        state: r.status.or(r.state).unwrap_or_default(),
        conclusion: r.conclusion,
        url: r.details_url.or(r.target_url),
        workflow: r.workflow_name,
    }
}

/// Decode `gh pr view --json …` output → (PR, its check rollup).
/// The rollup rides the same payload, so `view` and `checks` share
/// one gh call shape.
pub fn decode_pr_view(json: &str) -> Result<(PullRequest, Vec<CheckRun>), serde_json::Error> {
    let mut g: GhPr = serde_json::from_str(json)?;
    let checks = std::mem::take(&mut g.status_check_rollup)
        .into_iter()
        .map(to_check)
        .collect();
    Ok((to_pr(g), checks))
}

/// Decode `gh pr list --json …` output → PRs in backend order.
pub fn decode_pr_list(json: &str) -> Result<Vec<PullRequest>, serde_json::Error> {
    let items: Vec<GhPr> = serde_json::from_str(json)?;
    Ok(items.into_iter().map(to_pr).collect())
}

/// One entry of the `POST /repos/{o}/{r}/statuses/{sha}` response —
/// the API returns an array of the created status objects.
#[derive(Debug, Deserialize)]
struct GhStatus {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    state: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "target_url", default)]
    target_url: Option<String>,
    #[serde(rename = "created_at", default)]
    created_at: Option<String>,
}

/// Decode a commit-status POST response → the first created status.
/// The endpoint answers an array (usually one element); an empty
/// array is a decode error — nothing was recorded.
pub fn decode_status(json: &str) -> Result<CommitStatus, serde_json::Error> {
    let items: Vec<GhStatus> = serde_json::from_str(json)?;
    items
        .into_iter()
        .next()
        .map(|s| CommitStatus {
            id: s.id,
            state: s.state,
            context: s.context,
            description: s.description,
            target_url: s.target_url,
            created_at: s.created_at,
        })
        .ok_or_else(|| serde::de::Error::custom("commit-status response was an empty array"))
}

/// Extract the PR number from a `…/pull/<N>` URL — `gh pr create`
/// prints the URL on stdout, not JSON.
pub fn pr_number_from_url(url: &str) -> Option<u64> {
    url.rsplit("/pull/")
        .next()
        .and_then(|tail| tail.split('/').next())
        .and_then(|n| n.parse().ok())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const VIEW: &str = r#"{
        "number": 42,
        "title": "fix: heal",
        "state": "OPEN",
        "url": "https://github.com/o/r/pull/42",
        "isDraft": false,
        "headRefName": "heal/x",
        "headRefOid": "abc123",
        "baseRefName": "main",
        "body": "details",
        "author": {"login": "devin"},
        "mergeable": "MERGEABLE",
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "createdAt": "2025-01-01T00:00:00Z",
        "updatedAt": "2025-01-02T00:00:00Z",
        "mergedAt": null,
        "statusCheckRollup": [
            {"__typename":"CheckRun","name":"build","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://ci/1","workflowName":"ci"},
            {"__typename":"CheckRun","name":"test","status":"IN_PROGRESS","conclusion":null,"detailsUrl":"https://ci/2","workflowName":"ci"},
            {"__typename":"StatusContext","context":"lint","state":"FAILURE","targetUrl":"https://ci/3"}
        ]
    }"#;

    #[test]
    fn decodes_pr_view_and_rollup() {
        let (pr, checks) = decode_pr_view(VIEW).unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(pr.state, PrState::Open);
        assert_eq!(pr.head_ref, "heal/x");
        assert_eq!(pr.head_sha, "abc123");
        assert_eq!(pr.base_ref, "main");
        assert_eq!(pr.author.as_deref(), Some("devin"));
        assert_eq!(pr.review_decision.as_deref(), Some("APPROVED"));
        assert!(!pr.is_draft);

        assert_eq!(checks.len(), 3);
        assert_eq!(checks[0].name, "build");
        assert_eq!(checks[0].kind, CheckKind::CheckRun);
        assert_eq!(checks[0].bucket(), "pass");
        assert_eq!(checks[0].workflow.as_deref(), Some("ci"));
        assert_eq!(checks[1].bucket(), "pending");
        assert_eq!(checks[2].name, "lint");
        assert_eq!(checks[2].kind, CheckKind::StatusContext);
        assert_eq!(checks[2].bucket(), "fail");
        assert_eq!(checks[2].url.as_deref(), Some("https://ci/3"));
    }

    #[test]
    fn decodes_pr_list_subset() {
        let json = r#"[
            {"number":1,"title":"a","state":"MERGED","url":"u","headRefName":"b","baseRefName":"main"},
            {"number":2,"title":"c","state":"CLOSED","url":"u2","isDraft":true}
        ]"#;
        let prs = decode_pr_list(json).unwrap();
        assert_eq!(prs.len(), 2);
        assert_eq!(prs[0].state, PrState::Merged);
        assert_eq!(prs[0].head_ref, "b");
        assert_eq!(prs[1].state, PrState::Closed);
        assert!(prs[1].is_draft);
        assert!(prs[1].head_ref.is_empty());
    }

    #[test]
    fn unknown_state_and_typename_degrade() {
        let json = r#"{"number":1,"title":"t","state":"WHATEVER","url":"u",
            "statusCheckRollup":[{"__typename":"Other","name":"x"}]}"#;
        let (pr, checks) = decode_pr_view(json).unwrap();
        assert_eq!(pr.state, PrState::Unknown);
        assert_eq!(checks[0].kind, CheckKind::Unknown);
        assert_eq!(checks[0].bucket(), "pending");
    }

    #[test]
    fn parses_pr_number_from_url() {
        assert_eq!(
            pr_number_from_url("https://github.com/o/r/pull/123"),
            Some(123)
        );
        assert_eq!(pr_number_from_url("https://x/pull/7/files"), Some(7));
        assert_eq!(pr_number_from_url("not a url"), None);
    }

    #[test]
    fn decodes_status_response() {
        let json = r#"[{"id":7,"state":"pending","context":"docs/deploy",
            "description":"push.docs.abc","target_url":"https://x","created_at":"2025-01-01T00:00:00Z"}]"#;
        let st = decode_status(json).unwrap();
        assert_eq!(st.id, 7);
        assert_eq!(st.state, "pending");
        assert_eq!(st.context, "docs/deploy");
        assert_eq!(st.target_url.as_deref(), Some("https://x"));
        assert!(decode_status("[]").is_err());
        assert!(decode_status("nope").is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_pr_view("nope").is_err());
        assert!(decode_pr_list("{}").is_err());
    }
}
