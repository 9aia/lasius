//! lasius-vcs-git — the git impl of [`lasius_vcs::Vcs`]. Shells
//! out to the `git` binary (same convention as the five `*-git`
//! crates this consolidates — zero new dep weight; git2 remains a
//! possible later impl behind the same trait).

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lasius_vcs::{CommitInfo, DiffStat, LogEntry, Vcs};

/// The git CLI backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct GitVcs;

/// `git -C <repo> <args>…` → stdout on success.
fn git_out(repo: &Path, args: &[&str]) -> io::Result<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        Err(io::Error::other(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr).trim()
        )))
    }
}

/// `git -C <repo> <args>…` → exit status.
fn git_ok(repo: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

/// `git -C <repo> <args>…` → trimmed stdout, stdin `/dev/null`.
/// Shared by [`Vcs::exec_out`] and the plumbing helpers.
fn exec_out(repo: &Path, args: &[&str]) -> io::Result<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        Err(io::Error::other(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr).trim()
        )))
    }
}

/// `dir`'s path can't ride git's argv — refuse rather than pass an
/// empty string that would target the wrong directory.
fn path_arg(p: &Path) -> io::Result<&str> {
    p.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path is not utf-8: {}", p.display()),
        )
    })
}

impl Vcs for GitVcs {
    // -- objects / history -------------------------------------------------

    fn head(&self, repo: &Path) -> Option<String> {
        git_out(repo, &["rev-parse", "HEAD"])
            .ok()
            .map(|s| first_line(&s))
            .filter(|s| !s.is_empty())
    }

    fn resolve(&self, repo: &Path, rev: &str) -> Option<String> {
        let spec = format!("{rev}^{{commit}}");
        git_out(repo, &["rev-parse", "-q", "--verify", &spec])
            .ok()
            .map(|s| first_line(&s))
            .filter(|s| !s.is_empty())
    }

    fn has_commit(&self, repo: &Path, sha: &str) -> bool {
        git_ok(repo, &["cat-file", "-e", sha])
    }

    fn commits(&self, repo: &Path, rev: &str, limit: usize) -> Vec<CommitInfo> {
        // Metadata pass — one spawn; paths come from a per-commit
        // diff-tree pass (deterministic, no format ambiguity).
        let mut args: Vec<String> = vec![
            "log".into(),
            "--format=%x1f%H%x1f%an%x1f%at%x1f%s%x1f%b%x1e".to_string(),
            rev.to_string(),
        ];
        if limit > 0 {
            args.push(format!("-n{limit}"));
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Ok(out) = git_out(repo, &arg_refs) else {
            return Vec::new();
        };
        let mut commits = Vec::new();
        for rec in out.split('\x1e') {
            // Records are separated by \x1e and may carry a leading
            // newline + the format's leading \x1f.
            let rec = rec.trim().trim_start_matches('\x1f');
            if rec.is_empty() {
                continue;
            }
            let mut fields = rec.splitn(5, '\x1f');
            let (Some(sha), Some(author), Some(ts), Some(subject), body) = (
                fields.next(),
                fields.next(),
                fields.next(),
                fields.next(),
                fields.next(),
            ) else {
                continue;
            };
            let sha = sha.trim().to_string();
            if sha.is_empty() {
                continue;
            }
            let paths = git_out(
                repo,
                &[
                    "diff-tree",
                    "--no-commit-id",
                    "-r",
                    "--name-only",
                    "--root",
                    &sha,
                ],
            )
            .map(|p| {
                p.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
            commits.push(CommitInfo {
                sha,
                subject: subject.trim().to_string(),
                body: body.unwrap_or("").trim().to_string(),
                author: author.trim().to_string(),
                timestamp: ts.trim().parse().unwrap_or(0),
                paths,
            });
        }
        commits
    }

    fn diff_stat(&self, repo: &Path, from: &str, to: &str) -> Vec<DiffStat> {
        let range = format!("{from}...{to}");
        let Ok(out) = git_out(repo, &["diff", "--numstat", &range]) else {
            return Vec::new();
        };
        out.lines()
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let added = parts.next()?.parse().ok()?;
                let deleted = parts.next()?.parse().ok()?;
                let path = parts.next()?.to_string();
                Some(DiffStat {
                    path,
                    added,
                    deleted,
                })
            })
            .collect()
    }

    fn tags(&self, repo: &Path) -> Vec<String> {
        git_out(repo, &["tag", "-l"])
            .map(|s| {
                s.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn describe(&self, repo: &Path) -> Option<String> {
        git_out(repo, &["describe", "--tags", "--always"])
            .ok()
            .map(|s| first_line(&s))
            .filter(|s| !s.is_empty())
    }

    fn log(&self, repo: &Path, rev: &str, limit: usize) -> Vec<LogEntry> {
        // NUL-separated fields keep subjects with spaces safe;
        // unborn HEAD / bad rev / non-repo all surface as Err → [].
        let mut args: Vec<String> = vec!["log".into(), "--format=%H%x00%as%x00%s".into()];
        if limit > 0 {
            args.push(format!("-{limit}"));
        }
        args.push(rev.to_string());
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Ok(out) = exec_out(repo, &arg_refs) else {
            return Vec::new();
        };
        out.lines()
            .filter_map(|line| {
                let mut parts = line.split('\0');
                Some(LogEntry {
                    hash: parts.next()?.to_string(),
                    date: parts.next()?.to_string(),
                    subject: parts.next().unwrap_or_default().to_string(),
                })
            })
            .filter(|e| !e.hash.is_empty())
            .collect()
    }

    // -- repo discovery -----------------------------------------------------

    fn is_git_repo(&self, start: &Path) -> bool {
        exec_out(start, &["rev-parse", "--is-inside-work-tree"])
            .map(|s| s == "true")
            .unwrap_or(false)
    }

    fn repo_root(&self, start: &Path) -> Option<PathBuf> {
        exec_out(start, &["rev-parse", "--show-toplevel"])
            .ok()
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
    }

    // -- branches -----------------------------------------------------------

    fn branch_exists(&self, repo: &Path, name: &str) -> bool {
        git_ok(
            repo,
            &[
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{name}"),
            ],
        )
    }

    fn branch(&self, repo: &Path, name: &str, start: &str) -> bool {
        git_ok(repo, &["branch", name, start])
    }

    fn create_orphan_branch(&self, repo: &Path, name: &str, msg: &str) -> io::Result<()> {
        // Plumbing, not `checkout --orphan`: mktree an empty tree,
        // commit-tree a root commit, update-ref the branch — the
        // current checkout's index/worktree stay untouched. The
        // empty old-value makes update-ref create-only: a race past
        // branch_exists can't reset `name` to an empty root.
        let tree = exec_out(repo, &["mktree"])?;
        // commit-tree needs a committer identity; when the repo
        // can't resolve one, inject a lasius fallback for this one
        // invocation (a configured identity always wins — the probe
        // sees it first).
        let fallback: &[&str] = if exec_out(repo, &["var", "GIT_COMMITTER_IDENT"]).is_err() {
            &[
                "-c",
                "user.name=lasius",
                "-c",
                "user.email=lasius@localhost",
            ]
        } else {
            &[]
        };
        let mut ct_args: Vec<&str> = fallback.to_vec();
        ct_args.extend(["commit-tree", tree.trim(), "-m", msg]);
        let commit = exec_out(repo, &ct_args)?;
        exec_out(
            repo,
            &[
                "update-ref",
                &format!("refs/heads/{name}"),
                commit.trim(),
                "",
            ],
        )?;
        Ok(())
    }

    // -- worktrees ---------------------------------------------------------

    fn worktree_add(&self, repo: &Path, dir: &Path, rev: &str) -> bool {
        git_ok(
            repo,
            &[
                "worktree",
                "add",
                "--detach",
                dir.to_str().unwrap_or_default(),
                rev,
            ],
        )
    }

    fn worktree_add_branch(
        &self,
        repo: &Path,
        dir: &Path,
        branch: &str,
        rev: Option<&str>,
    ) -> io::Result<()> {
        let mut args: Vec<&str> = vec!["worktree", "add", "-b", branch];
        args.push(dir.to_str().unwrap_or_default());
        if let Some(r) = rev {
            args.push(r);
        }
        git_out(repo, &args).map(|_| ())
    }

    fn worktree_add_existing(&self, repo: &Path, dir: &Path, branch: &str) -> io::Result<()> {
        git_out(repo, &["worktree", "add", path_arg(dir)?, branch]).map(|_| ())
    }

    fn worktree_remove(&self, repo: &Path, dir: &Path) -> bool {
        git_ok(
            repo,
            &[
                "worktree",
                "remove",
                "--force",
                dir.to_str().unwrap_or_default(),
            ],
        )
    }

    fn worktree_prune(&self, repo: &Path) -> bool {
        git_ok(repo, &["worktree", "prune"])
    }

    fn common_dir(&self, repo: &Path) -> Option<PathBuf> {
        exec_out(
            repo,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| !p.as_os_str().is_empty())
    }

    fn main_worktree(&self, repo: &Path) -> Option<PathBuf> {
        match self.common_dir(repo) {
            Some(common) => {
                // `<main>/.git` → `<main>`; a bare repo's common dir
                // isn't named `.git` → no checkout root.
                if common.file_name().is_some_and(|n| n == ".git") {
                    common.parent().map(Path::to_path_buf)
                } else {
                    None
                }
            }
            // Old git without `--path-format`: `worktree list`'s
            // first entry is always the main worktree.
            None => {
                let out = git_out(repo, &["worktree", "list"]).ok()?;
                let path = out.lines().next()?.split_whitespace().next()?;
                Some(PathBuf::from(path))
            }
        }
    }

    // -- working tree / commits ---------------------------------------------

    fn current_branch(&self, repo: &Path) -> Option<String> {
        git_out(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
            .ok()
            .map(|s| first_line(&s))
            .filter(|s| !s.is_empty() && s != "HEAD")
    }

    fn stage_all(&self, repo: &Path) -> io::Result<()> {
        git_out(repo, &["add", "-A"]).map(|_| ())
    }

    fn commit(
        &self,
        repo: &Path,
        subject: &str,
        body: Option<&str>,
        allow_empty: bool,
    ) -> io::Result<()> {
        let mut args: Vec<&str> = vec!["commit", "-m", subject];
        if let Some(b) = body {
            args.push("-m");
            args.push(b);
        }
        if allow_empty {
            args.push("--allow-empty");
        }
        git_out(repo, &args).map(|_| ())
    }

    fn has_changes(&self, repo: &Path) -> io::Result<bool> {
        // Scoped to `repo` itself (`-- .`): a plain subdirectory of
        // a bigger repo doesn't count changes elsewhere. Err
        // propagates — "status failed" is never "clean".
        exec_out(repo, &["status", "--porcelain", "--", "."]).map(|s| !s.is_empty())
    }

    fn is_detached(&self, repo: &Path) -> Option<bool> {
        // HEAD must resolve first: unborn branch / non-checkout →
        // None (a commit there still lands on the unborn branch).
        self.head(repo)?;
        Some(self.current_branch(repo).is_none())
    }

    fn attach_branch(&self, repo: &Path, name: &str) -> io::Result<()> {
        if self.current_branch(repo).as_deref() == Some(name) {
            return Ok(());
        }
        // `checkout -B` keeps the current HEAD as the branch tip —
        // a branch left behind by an earlier run loses to the
        // commits actually here; git refuses when another worktree
        // holds `name`.
        git_out(repo, &["checkout", "-B", name]).map(|_| ())
    }

    // -- divergence / rebase ------------------------------------------------

    fn commits_ahead(&self, repo: &Path, rev: &str) -> Option<usize> {
        let range = format!("{rev}..HEAD");
        exec_out(repo, &["rev-list", "--count", &range])
            .ok()?
            .parse()
            .ok()
    }

    fn commits_behind(&self, repo: &Path, rev: &str) -> Option<usize> {
        let range = format!("HEAD..{rev}");
        exec_out(repo, &["rev-list", "--count", &range])
            .ok()?
            .parse()
            .ok()
    }

    fn merge_ff(&self, repo: &Path, rev: &str) -> io::Result<()> {
        exec_out(repo, &["merge", "--ff-only", rev]).map(|_| ())
    }

    fn rebase(&self, repo: &Path, rev: &str) -> io::Result<()> {
        exec_out(repo, &["rebase", "--autostash", rev]).map(|_| ())
    }

    fn rebase_abort(&self, repo: &Path) -> io::Result<()> {
        exec_out(repo, &["rebase", "--abort"]).map(|_| ())
    }

    fn rebase_in_progress(&self, repo: &Path) -> bool {
        // `REBASE_HEAD` alone persists after a *finished* rebase —
        // only the state dirs (and MERGE_HEAD for a stopped merge)
        // signal mid-flight.
        ["rebase-merge", "rebase-apply"].iter().any(|state| {
            git_out(repo, &["rev-parse", "--git-path", state])
                .map(|p| {
                    let p = p.trim();
                    let p = Path::new(p);
                    // --git-path may answer relative to `repo`
                    repo.join(p).exists() || p.exists()
                })
                .unwrap_or(false)
        }) || git_ok(repo, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
    }

    fn unmerged_files(&self, repo: &Path) -> Vec<String> {
        exec_out(repo, &["diff", "--name-only", "--diff-filter=U"])
            .map(|s| {
                s.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn tracks_branch(&self, repo: &Path, remote: &str, branch: &str) -> bool {
        // Does the remote's fetch refspec publish `branch` into
        // `refs/remotes/<remote>/`? A `--single-branch` clone only
        // tracks some branches, so `<remote>/<branch>` can sit stale
        // forever while pushes still succeed.
        let key = format!("remote.{remote}.fetch");
        let Ok(specs) = exec_out(repo, &["config", "--get-all", &key]) else {
            return false;
        };
        let want = format!("refs/remotes/{remote}/{branch}");
        let all = format!("refs/remotes/{remote}/*");
        specs.lines().any(|line| {
            let spec = line.trim().trim_start_matches('+');
            matches!(spec.rsplit_once(':'), Some((_, dst)) if dst == want || dst == all)
        })
    }

    // -- tags / refs -------------------------------------------------------

    fn tag_exists(&self, repo: &Path, tag: &str) -> bool {
        git_ok(
            repo,
            &["rev-parse", "-q", "--verify", &format!("refs/tags/{tag}")],
        )
    }

    fn tag(&self, repo: &Path, tag: &str, rev: Option<&str>) -> bool {
        let mut args = vec!["tag", tag];
        if let Some(r) = rev {
            args.push(r);
        }
        git_ok(repo, &args)
    }

    fn push_tag(&self, repo: &Path, remote: &str, tag: &str) -> bool {
        git_ok(repo, &["push", remote, tag])
    }

    fn delete_tag(&self, repo: &Path, tag: &str) -> bool {
        git_ok(repo, &["tag", "-d", tag])
    }

    // -- sync / push -------------------------------------------------------

    fn sync(&self, repo: &Path, remote: Option<&str>) -> bool {
        git_ok(repo, &["fetch", remote.unwrap_or("origin"), "--prune"])
    }

    fn fetch(&self, repo: &Path, remote: &str, refspecs: &[&str]) -> io::Result<()> {
        let mut args: Vec<&str> = vec!["fetch", remote];
        args.extend_from_slice(refspecs);
        git_out(repo, &args).map(|_| ())
    }

    fn push(&self, repo: &Path, remote: &str, refspecs: &[&str]) -> bool {
        let mut args: Vec<&str> = vec!["push", remote];
        args.extend_from_slice(refspecs);
        git_ok(repo, &args)
    }

    fn push_branch(&self, repo: &Path, remote: &str, branch: Option<&str>) -> io::Result<()> {
        // `HEAD` alone pushes the checked-out branch under its own name.
        git_out(repo, &["push", "-u", remote, branch.unwrap_or("HEAD")]).map(|_| ())
    }

    // -- raw passthrough -----------------------------------------------------

    fn exec(&self, repo: &Path, args: &[&str]) -> bool {
        git_ok(repo, args)
    }

    fn exec_out(&self, repo: &Path, args: &[&str]) -> io::Result<String> {
        exec_out(repo, args)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    /// A throwaway repo with two commits + a tag.
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let run = |args: &[&str]| {
            let o = Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(args)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .output()
                .unwrap();
            assert!(o.status.success(), "git {args:?}: {:?}", o);
        };
        run(&["init", "-q", "."]);
        std::fs::write(repo.join("a.txt"), "one\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-qm", "feat: first"]);
        run(&["tag", "v0.1.0"]);
        std::fs::write(repo.join("b.txt"), "two\nthree\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-qm", "fix: second"]);
        (dir, repo)
    }

    #[test]
    fn history_ops() {
        let (_t, repo) = fixture();
        let v = GitVcs;
        assert!(v.head(&repo).is_some());
        assert_eq!(v.tags(&repo), vec!["v0.1.0"]);
        assert!(v.tag_exists(&repo, "v0.1.0"));
        let commits = v.commits(&repo, "HEAD", 0);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "fix: second");
        assert_eq!(commits[0].paths, vec!["b.txt"]);
        assert_eq!(commits[1].subject, "feat: first");
        assert_eq!(commits[1].paths, vec!["a.txt"]);
        let range = v.commits(&repo, "v0.1.0..HEAD", 0);
        assert_eq!(range.len(), 1);
        let stat = v.diff_stat(&repo, "v0.1.0", "HEAD");
        assert_eq!(stat.len(), 1);
        assert_eq!(stat[0].path, "b.txt");
        assert!(v.describe(&repo).unwrap().contains("v0.1.0"));
    }

    #[test]
    fn worktree_and_tag_lifecycle() {
        let (_t, repo) = fixture();
        let v = GitVcs;
        let wt = repo.parent().unwrap().join("wt");
        assert!(v.worktree_add(&repo, &wt, "HEAD"));
        assert!(wt.join("a.txt").exists());
        assert!(v.worktree_remove(&repo, &wt));
        assert!(v.tag(&repo, "v9.9.9", None));
        assert!(v.tag_exists(&repo, "v9.9.9"));
        assert!(v.delete_tag(&repo, "v9.9.9"));
        assert!(!v.tag_exists(&repo, "v9.9.9"));
    }

    #[test]
    fn agent_mutation_ops() {
        let (_t, repo) = fixture();
        let v = GitVcs;
        assert!(v.current_branch(&repo).is_some());

        // commit identity — repo-local config is shared with worktrees
        assert!(git_ok(&repo, &["config", "user.email", "t@t"]));
        assert!(git_ok(&repo, &["config", "user.name", "t"]));

        // bare remote wired as origin
        let remote = repo.parent().unwrap().join("remote.git");
        let st = Command::new("git")
            .args(["init", "-q", "--bare"])
            .arg(&remote)
            .status()
            .unwrap();
        assert!(st.success());
        assert!(git_ok(
            &repo,
            &["remote", "add", "origin", remote.to_str().unwrap()]
        ));

        // worktree on a new branch at HEAD
        let wt = repo.parent().unwrap().join("wt-branch");
        let head = v.head(&repo).unwrap();
        v.worktree_add_branch(&repo, &wt, "heal/x", Some(&head))
            .unwrap();
        assert_eq!(v.current_branch(&wt).as_deref(), Some("heal/x"));

        // edit → stage → commit
        std::fs::write(wt.join("c.txt"), "x\n").unwrap();
        v.stage_all(&wt).unwrap();
        v.commit(&wt, "fix: heal", Some("details"), false).unwrap();
        let new_head = v.head(&wt).unwrap();
        assert_ne!(new_head, head);

        // push current branch, then fetch back
        v.push_branch(&wt, "origin", None).unwrap();
        assert!(v.has_commit(&remote, &new_head));
        v.fetch(&wt, "origin", &[]).unwrap();
        v.fetch(&repo, "origin", &["heal/x"]).unwrap();
    }

    /// Repo-local commit identity for `v.commit` (fixture commits
    /// ride env vars that don't persist).
    fn ident(repo: &Path) {
        assert!(git_ok(repo, &["config", "user.email", "t@t"]));
        assert!(git_ok(repo, &["config", "user.name", "t"]));
    }

    #[test]
    fn repo_discovery_log_and_exec() {
        let (_t, repo) = fixture();
        let v = GitVcs;
        assert!(v.is_git_repo(&repo));
        let sub = repo.join("sub/dir");
        std::fs::create_dir_all(&sub).unwrap();
        assert!(v.is_git_repo(&sub));
        assert_eq!(
            v.repo_root(&sub).unwrap().canonicalize().unwrap(),
            repo.canonicalize().unwrap()
        );
        let plain = tempfile::tempdir().unwrap();
        assert!(!v.is_git_repo(plain.path()));
        assert!(v.repo_root(plain.path()).is_none());

        // cheap log listing — hash/date/subject, newest first
        let entries = v.log(&repo, "HEAD", 0);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].subject, "fix: second");
        assert_eq!(entries[0].hash, v.head(&repo).unwrap());
        assert_eq!(entries[0].date.len(), 10);
        assert_eq!(v.log(&repo, "HEAD", 1).len(), 1);
        assert!(v.log(&repo, "no-such-rev", 0).is_empty());

        // raw passthrough
        assert!(v.exec(&repo, &["--version"]));
        assert_eq!(
            v.exec_out(&repo, &["rev-parse", "--is-inside-work-tree"])
                .unwrap(),
            "true"
        );
        let e = v
            .exec_out(&repo, &["rev-parse", "--verify", "nope"])
            .unwrap_err();
        assert!(e.to_string().contains("rev-parse"));
    }

    #[test]
    fn branch_ops_and_orphan() {
        let (_t, repo) = fixture();
        let v = GitVcs;
        assert!(!v.branch_exists(&repo, "side"));
        assert!(v.branch(&repo, "side", "HEAD"));
        assert!(v.branch_exists(&repo, "side"));

        // orphan branch: plumbing root commit, checkout untouched,
        // create-only (second attempt fails, ref unchanged)
        let before = v.current_branch(&repo);
        v.create_orphan_branch(&repo, "project", "init ledger")
            .unwrap();
        assert!(v.branch_exists(&repo, "project"));
        let log = v.log(&repo, "project", 0);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].subject, "init ledger");
        // root commit has no parents
        let parents = v
            .exec_out(&repo, &["rev-list", "--parents", "-n", "1", "project"])
            .unwrap();
        assert_eq!(parents.split_whitespace().count(), 1);
        assert_eq!(v.current_branch(&repo), before);
        assert!(v.create_orphan_branch(&repo, "project", "clobber").is_err());
    }

    #[test]
    fn worktree_on_existing_branch_and_common_dir() {
        let (_t, repo) = fixture();
        let v = GitVcs;
        assert!(v.branch(&repo, "side", "HEAD"));
        let wt = repo.parent().unwrap().join("wt-side");
        v.worktree_add_existing(&repo, &wt, "side").unwrap();
        assert_eq!(v.current_branch(&wt).as_deref(), Some("side"));
        // the branch is held — a second checkout of it is refused
        let wt2 = repo.parent().unwrap().join("wt-side-2");
        assert!(v.worktree_add_existing(&repo, &wt2, "side").is_err());
        assert!(v.worktree_remove(&repo, &wt));

        // common dir + main worktree resolve from a linked worktree
        let wt3 = repo.parent().unwrap().join("wt3");
        v.worktree_add(&repo, &wt3, "HEAD");
        assert_eq!(
            v.common_dir(&wt3).unwrap().canonicalize().unwrap(),
            repo.join(".git").canonicalize().unwrap()
        );
        assert_eq!(
            v.main_worktree(&wt3).unwrap().canonicalize().unwrap(),
            repo.canonicalize().unwrap()
        );
        assert_eq!(
            v.main_worktree(&repo).unwrap().canonicalize().unwrap(),
            repo.canonicalize().unwrap()
        );
    }

    #[test]
    fn detached_detect_attach_and_has_changes() {
        let (_t, repo) = fixture();
        ident(&repo);
        let v = GitVcs;
        let wt = repo.parent().unwrap().join("wt-detached");
        assert!(v.worktree_add(&repo, &wt, "HEAD~1"));

        assert_eq!(v.is_detached(&repo), Some(false));
        assert_eq!(v.is_detached(&wt), Some(true));
        let plain = tempfile::tempdir().unwrap();
        assert_eq!(v.is_detached(plain.path()), None);

        // has_changes: clean → false, dirty → true, non-repo → Err
        assert!(!v.has_changes(&repo).unwrap());
        std::fs::write(repo.join("dirty.txt"), "x\n").unwrap();
        assert!(v.has_changes(&repo).unwrap());
        std::fs::remove_file(repo.join("dirty.txt")).unwrap();
        assert!(v.has_changes(plain.path()).is_err());

        // commit on the detached worktree, then attach a branch
        std::fs::write(wt.join("c.txt"), "x\n").unwrap();
        v.stage_all(&wt).unwrap();
        v.commit(&wt, "wt: detached commit", None, false).unwrap();
        let wt_head = v.head(&wt).unwrap();
        v.attach_branch(&wt, "ledger").unwrap();
        assert_eq!(v.current_branch(&wt).as_deref(), Some("ledger"));
        assert_eq!(v.head(&wt).unwrap(), wt_head);
        v.attach_branch(&wt, "ledger").unwrap(); // already attached → no-op
                                                 // a branch held by the main checkout is refused, not stolen
        let main_branch = v.current_branch(&repo).unwrap();
        assert!(v.attach_branch(&wt, &main_branch).is_err());
        assert!(v.worktree_remove(&repo, &wt));
    }

    #[test]
    fn ahead_behind_merge_ff_and_rebase() {
        let (_t, repo) = fixture();
        ident(&repo);
        let v = GitVcs;
        let head = v.head(&repo).unwrap();

        assert_eq!(v.commits_ahead(&repo, "HEAD~1"), Some(1));
        assert_eq!(v.commits_behind(&repo, "HEAD~1"), Some(0));
        assert_eq!(v.commits_ahead(&repo, "refs/heads/ghost"), None);

        // detached worktree at HEAD~1 — behind the main tip
        let wt = repo.parent().unwrap().join("wt-behind");
        assert!(v.worktree_add(&repo, &wt, "HEAD~1"));
        assert_eq!(v.commits_behind(&wt, &head), Some(1));
        assert_eq!(v.commits_ahead(&wt, &head), Some(0));

        // fast-forward the detached HEAD to the main tip
        v.merge_ff(&wt, &head).unwrap();
        assert_eq!(v.head(&wt).unwrap(), head);
        assert!(v.worktree_remove(&repo, &wt));

        // diverge: wt commits on a.txt at HEAD~1, repo commits on
        // a.txt at HEAD → rebase conflicts, abort restores
        let wt = repo.parent().unwrap().join("wt-conflict");
        assert!(v.worktree_add(&repo, &wt, "HEAD~1"));
        std::fs::write(wt.join("a.txt"), "wt side\n").unwrap();
        v.stage_all(&wt).unwrap();
        v.commit(&wt, "wt: a", None, false).unwrap();
        std::fs::write(repo.join("a.txt"), "main side\n").unwrap();
        v.stage_all(&repo).unwrap();
        v.commit(&repo, "main: a", None, false).unwrap();
        let main_head = v.head(&repo).unwrap();

        assert!(!v.rebase_in_progress(&wt));
        assert!(v.rebase(&wt, &main_head).is_err());
        assert!(v.rebase_in_progress(&wt));
        assert!(v.unmerged_files(&wt).contains(&"a.txt".to_string()));
        v.rebase_abort(&wt).unwrap();
        assert!(!v.rebase_in_progress(&wt));
        assert!(v.unmerged_files(&wt).is_empty());

        // histories still diverged → ff-only merge refuses
        assert!(v.merge_ff(&wt, &main_head).is_err());
        assert!(v.worktree_remove(&repo, &wt));

        // clean rebase path: a worktree already based on the main
        // tip replays its own commit without conflict
        let wt = repo.parent().unwrap().join("wt-clean");
        assert!(v.worktree_add(&repo, &wt, &main_head));
        std::fs::write(wt.join("wt-only.txt"), "wt\n").unwrap();
        v.stage_all(&wt).unwrap();
        v.commit(&wt, "wt: other file", None, false).unwrap();
        v.rebase(&wt, &main_head).unwrap();
        assert!(!v.rebase_in_progress(&wt));
        assert!(v.worktree_remove(&repo, &wt));
    }

    #[test]
    fn tracks_branch_and_has_changes_scope() {
        let (_t, repo) = fixture();
        let v = GitVcs;
        // no remote configured → nothing tracked
        assert!(!v.tracks_branch(&repo, "origin", "main"));
        assert!(git_ok(&repo, &["config", "remote.origin.url", "x"]));
        assert!(git_ok(
            &repo,
            &[
                "config",
                "remote.origin.fetch",
                "+refs/heads/main:refs/remotes/origin/main"
            ]
        ));
        assert!(!v.tracks_branch(&repo, "origin", "project"));
        assert!(git_ok(
            &repo,
            &[
                "config",
                "--replace-all",
                "remote.origin.fetch",
                "+refs/heads/*:refs/remotes/origin/*"
            ]
        ));
        assert!(v.tracks_branch(&repo, "origin", "project"));
        // an explicit single-branch spec also counts — and excludes
        // everything else
        assert!(git_ok(
            &repo,
            &[
                "config",
                "--replace-all",
                "remote.origin.fetch",
                "+refs/heads/project:refs/remotes/origin/project"
            ]
        ));
        assert!(v.tracks_branch(&repo, "origin", "project"));
        assert!(!v.tracks_branch(&repo, "origin", "docs"));
    }
}
