//! lasius-forge-gh — the `gh` impl of [`lasius_forge::Forge`].
//! Shells out to the `gh` binary (same convention as
//! `lasius-vcs-git` shelling `git` — zero new dep weight), running
//! every command with the repo root as cwd so `gh` resolves
//! owner/repo from the checkout's remote.
//!
//! `gh pr create` prints the new PR's URL on stdout (no `--json`), so
//! [`crate::gh::pr_number_from_url`] recovers the number; `view` and
//! `list` decode through [`lasius_forge::gh`].

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use lasius_forge::gh as gh_json;
use lasius_forge::{CheckRun, CreatePr, Forge, MergeMethod, PrStateFilter, PullRequest};

/// `gh pr view/list --json` field list — keep in sync with the
/// `GhPr` decoder in `lasius-forge::gh`.
const PR_FIELDS: &str = "number,title,state,url,isDraft,body,headRefName,headRefOid,\
     baseRefName,author,mergeable,mergeStateStatus,reviewDecision,createdAt,updatedAt,\
     mergedAt,statusCheckRollup";

const LIST_FIELDS: &str = "number,title,state,url,isDraft,headRefName,baseRefName,\
     author,updatedAt";

/// Captured `gh` invocation.
struct GhOut {
    code: i32,
    stdout: String,
    stderr: String,
}

/// The GitHub CLI backend.
#[derive(Debug, Clone)]
pub struct GhForge {
    program: String,
}

impl Default for GhForge {
    fn default() -> Self {
        Self::new()
    }
}

impl GhForge {
    /// `gh` resolved from `PATH`.
    pub fn new() -> Self {
        Self {
            program: "gh".to_string(),
        }
    }

    /// A specific `gh` binary (tests, non-PATH installs).
    pub fn with_program(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
        }
    }

    /// `gh -R`-free invocation: cwd is the repo, gh infers owner/repo.
    fn run(&self, repo: &Path, args: &[String]) -> io::Result<GhOut> {
        let o = Command::new(&self.program)
            .args(args)
            .current_dir(repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        Ok(GhOut {
            code: o.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        })
    }

    /// Run `gh …` expecting success; `Err` carries stderr (or stdout
    /// when stderr is empty) like `lasius-vcs-git`'s `git_out`.
    fn ok(&self, repo: &Path, args: &[String]) -> io::Result<String> {
        let out = self.run(repo, args)?;
        if out.code == 0 {
            Ok(out.stdout)
        } else {
            let detail = if out.stderr.trim().is_empty() {
                out.stdout.trim()
            } else {
                out.stderr.trim()
            };
            Err(io::Error::other(format!(
                "gh {}: exit {}: {detail}",
                args.join(" "),
                out.code
            )))
        }
    }
}

impl Forge for GhForge {
    fn create_pr(&self, repo: &Path, req: &CreatePr) -> io::Result<PullRequest> {
        let mut args: Vec<String> = vec![
            "pr".into(),
            "create".into(),
            "--title".into(),
            req.title.clone(),
            "--body".into(),
            req.body.clone(),
        ];
        if let Some(base) = &req.base {
            args.push("--base".into());
            args.push(base.clone());
        }
        if let Some(head) = &req.head {
            args.push("--head".into());
            args.push(head.clone());
        }
        if req.draft {
            args.push("--draft".into());
        }
        let out = self.ok(repo, &args)?;
        // gh prints the PR URL as the last stdout line.
        let url = out
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string();
        let number = gh_json::pr_number_from_url(&url).ok_or_else(|| {
            io::Error::other(format!("gh pr create: could not parse PR URL {url:?}"))
        })?;
        Ok(PullRequest {
            number,
            title: req.title.clone(),
            state: lasius_forge::PrState::Open,
            url,
            is_draft: req.draft,
            head_ref: req.head.clone().unwrap_or_default(),
            head_sha: String::new(),
            base_ref: req.base.clone().unwrap_or_default(),
            body: req.body.clone(),
            author: None,
            mergeable: None,
            merge_state: None,
            review_decision: None,
            created_at: None,
            updated_at: None,
            merged_at: None,
        })
    }

    fn merge_pr(
        &self,
        repo: &Path,
        number: u64,
        method: MergeMethod,
        delete_branch: bool,
    ) -> io::Result<()> {
        let mut args: Vec<String> = vec!["pr".into(), "merge".into(), number.to_string()];
        args.push(
            match method {
                MergeMethod::Squash => "--squash",
                MergeMethod::Merge => "--merge",
                MergeMethod::Rebase => "--rebase",
            }
            .into(),
        );
        if delete_branch {
            args.push("--delete-branch".into());
        }
        self.ok(repo, &args).map(|_| ())
    }

    fn view_pr(&self, repo: &Path, number: u64) -> io::Result<PullRequest> {
        let out = self.ok(
            repo,
            &[
                "pr".into(),
                "view".into(),
                number.to_string(),
                "--json".into(),
                PR_FIELDS.into(),
            ],
        )?;
        gh_json::decode_pr_view(&out)
            .map(|(pr, _)| pr)
            .map_err(|e| io::Error::other(format!("gh pr view: bad JSON: {e}")))
    }

    fn pr_checks(&self, repo: &Path, number: u64) -> io::Result<Vec<CheckRun>> {
        // The rollup rides `pr view --json` — `gh pr checks` exits
        // non-zero on pending/failing checks, which would masquerade
        // as an error here.
        let out = self.ok(
            repo,
            &[
                "pr".into(),
                "view".into(),
                number.to_string(),
                "--json".into(),
                PR_FIELDS.into(),
            ],
        )?;
        gh_json::decode_pr_view(&out)
            .map(|(_, checks)| checks)
            .map_err(|e| io::Error::other(format!("gh pr view: bad JSON: {e}")))
    }

    fn list_prs(
        &self,
        repo: &Path,
        state: PrStateFilter,
        limit: usize,
    ) -> io::Result<Vec<PullRequest>> {
        let mut args: Vec<String> = vec![
            "pr".into(),
            "list".into(),
            "--state".into(),
            match state {
                PrStateFilter::Open => "open",
                PrStateFilter::Closed => "closed",
                PrStateFilter::Merged => "merged",
                PrStateFilter::All => "all",
            }
            .into(),
            "--json".into(),
            LIST_FIELDS.into(),
        ];
        if limit > 0 {
            args.push("--limit".into());
            args.push(limit.to_string());
        }
        let out = self.ok(repo, &args)?;
        gh_json::decode_pr_list(&out)
            .map_err(|e| io::Error::other(format!("gh pr list: bad JSON: {e}")))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::fs;

    /// A fake `gh` shell script; returns (tempdir guard, GhForge).
    /// The script records argv to `$GH_FAKE_LOG` and answers `pr view`
    /// with a fixed rollup payload.
    fn fake_gh(script_body: &str) -> (tempfile::TempDir, GhForge) {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("gh");
        fs::write(&bin, script_body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let forge = GhForge::with_program(bin.to_string_lossy().into_owned());
        (dir, forge)
    }

    #[test]
    fn view_decodes_rollup() {
        let body = r#"#!/bin/sh
echo '{"number":7,"title":"t","state":"OPEN","url":"https://x/pull/7","statusCheckRollup":[{"__typename":"CheckRun","name":"ci","status":"COMPLETED","conclusion":"SUCCESS"}]}'
"#;
        let (_t, forge) = fake_gh(body);
        let repo = tempfile::tempdir().unwrap();
        let pr = forge.view_pr(repo.path(), 7).unwrap();
        assert_eq!(pr.number, 7);
        let checks = forge.pr_checks(repo.path(), 7).unwrap();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].bucket(), "pass");
    }

    #[test]
    fn create_parses_url_and_sends_flags() {
        let body = "#!/bin/sh\necho 'https://github.com/o/r/pull/9'\n";
        let (_t, forge) = fake_gh(body);
        let repo = tempfile::tempdir().unwrap();
        let pr = forge
            .create_pr(
                repo.path(),
                &CreatePr {
                    title: "t".into(),
                    body: "b".into(),
                    base: Some("main".into()),
                    head: Some("heal/x".into()),
                    draft: true,
                },
            )
            .unwrap();
        assert_eq!(pr.number, 9);
        assert_eq!(pr.url, "https://github.com/o/r/pull/9");
        assert!(pr.is_draft);
    }

    #[test]
    fn merge_and_list_round_trip() {
        // `pr merge` succeeds silently; `pr list` answers an empty array.
        let body = "#!/bin/sh\nif [ \"$1 $2\" = 'pr list' ]; then echo '[]'; fi\nexit 0\n";
        let (_t, forge) = fake_gh(body);
        let repo = tempfile::tempdir().unwrap();

        forge
            .merge_pr(repo.path(), 3, MergeMethod::Squash, true)
            .unwrap();
        let prs = forge.list_prs(repo.path(), PrStateFilter::All, 5).unwrap();
        assert!(prs.is_empty());
    }

    #[test]
    fn propagates_gh_failure() {
        let body = "#!/bin/sh\necho 'no such PR' >&2\nexit 1\n";
        let (_t, forge) = fake_gh(body);
        let repo = tempfile::tempdir().unwrap();
        let err = forge.view_pr(repo.path(), 1).unwrap_err();
        assert!(err.to_string().contains("no such PR"));
    }
}
