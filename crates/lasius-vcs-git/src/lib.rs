//! lasius-vcs-git — the git impl of [`lasius_vcs::Vcs`]. Shells
//! out to the `git` binary (same convention as the five `*-git`
//! crates this consolidates — zero new dep weight; git2 remains a
//! possible later impl behind the same trait).

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use lasius_vcs::{CommitInfo, DiffStat, Vcs};

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

    fn push(&self, repo: &Path, remote: &str, refspecs: &[&str]) -> bool {
        let mut args: Vec<&str> = vec!["push", remote];
        args.extend_from_slice(refspecs);
        git_ok(repo, &args)
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
}
