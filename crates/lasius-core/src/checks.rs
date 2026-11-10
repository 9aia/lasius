//! Conformance checks — pure evaluation over facts gathered by the cli.
//!
//! Domains: `structure`, `clones`, `generated`, `policy`. (`env` stays in
//! `doctor` for now.) Each check is a function `(&Facts) -> Vec<Finding>`;
//! findings carry a stable kebab-case id so `[checks] disabled` and
//! `severity` overrides in `lasius.toml` can target them.

use crate::config::WorkspaceConfig;

/// Finding severity — `Error` fails `lasius check` (exit 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational — never fails.
    Info,
    /// Suspicious — reported, doesn't fail.
    Warn,
    /// Broken — fails the run.
    Error,
}

impl Severity {
    /// Parse a `[checks.severity]` value.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// One conformance finding.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Stable kebab-case check id (e.g. `"worktree-health"`).
    pub check: &'static str,
    /// Effective severity (after config overrides).
    pub severity: Severity,
    /// Repo the finding is about, if per-repo.
    pub repo: Option<String>,
    /// Human-readable detail.
    pub message: String,
}

impl Finding {
    fn new(check: &'static str, severity: Severity, repo: Option<String>, msg: String) -> Self {
        Self {
            check,
            severity,
            repo,
            message: msg,
        }
    }
}

/// Per-repo facts gathered by the cli (all I/O already done).
#[derive(Debug, Clone, Default)]
pub struct RepoFacts {
    /// Directory name under `repos_dir`.
    pub name: String,
    /// Whether `<repo>/.git` exists.
    pub is_git: bool,
    /// `git remote get-url origin`, if any.
    pub remote: Option<String>,
    /// Worktree entry names present (basenames of `git worktree list`).
    pub worktree_names: Vec<String>,
    /// Linked-worktree paths that no longer exist on disk.
    pub broken_worktrees: Vec<String>,
    /// A nested `.git` directory exists below the top level.
    pub has_nested_git: bool,
    /// `[policy.require_files]` entries missing from the clone.
    pub missing_files: Vec<String>,
    /// `git ls-remote origin HEAD` probe: `Some(true)` reachable,
    /// `Some(false)` failed, `None` not probed (no origin / not a clone).
    pub remote_ok: Option<bool>,
    /// stderr tail when `remote_ok` is `Some(false)`.
    pub remote_error: Option<String>,
}

/// Everything the checks need — gathered once by the cli.
#[derive(Debug)]
pub struct Facts<'a> {
    /// Parsed workspace config.
    pub config: &'a WorkspaceConfig,
    /// `.gitignore` contents, if readable.
    pub gitignore: Option<String>,
    /// `README.md` exists at root.
    pub readme: bool,
    /// `AGENTS.md` exists at root.
    pub agents: bool,
    /// `repos_dir` exists.
    pub repos_dir_exists: bool,
    /// Local clone names under `repos_dir`.
    pub local_names: Vec<String>,
    /// Generated workspace file contents, if present.
    pub workspace_file: Option<String>,
    /// Whether the on-disk workspace file matches a fresh generation.
    pub workspace_fresh: bool,
    /// Org repo names from the configured provider, if discovery succeeded.
    pub org_names: Option<Vec<String>>,
    /// Why org discovery failed (truncated error text), if it did.
    pub org_error: Option<String>,
    /// Per-repo facts (parallel-gathered).
    pub repos: Vec<RepoFacts>,
}

fn sev(config: &WorkspaceConfig, id: &str, base: Severity) -> Severity {
    config
        .checks
        .severity
        .get(id)
        .and_then(|s| Severity::parse(s))
        .unwrap_or(base)
}

fn enabled(config: &WorkspaceConfig, id: &str) -> bool {
    config.checks.is_enabled(id)
}

fn push(
    out: &mut Vec<Finding>,
    config: &WorkspaceConfig,
    id: &'static str,
    base: Severity,
    repo: Option<String>,
    msg: String,
) {
    if enabled(config, id) {
        out.push(Finding::new(id, sev(config, id, base), repo, msg));
    }
}

/// All checks, filtered by `only` (None = all enabled) and config.
pub fn run(facts: &Facts, only: Option<&[String]>) -> Vec<Finding> {
    let mut out = Vec::new();
    let want = |id: &str| only.map(|o| o.iter().any(|s| s == id)).unwrap_or(true);
    if want("gitignore-coverage") {
        gitignore_coverage(facts, &mut out);
    }
    if want("docs-stubs") {
        docs_stubs(facts, &mut out);
    }
    if want("repos-dir") {
        repos_dir(facts, &mut out);
    }
    if want("clone-validity") {
        clone_validity(facts, &mut out);
    }
    if want("remote-match") {
        remote_match(facts, &mut out);
    }
    if want("remote-reachable") {
        remote_reachable(facts, &mut out);
    }
    if want("worktree-health") {
        worktree_health(facts, &mut out);
    }
    if want("nested-git") {
        nested_git(facts, &mut out);
    }
    if want("workspace-freshness") {
        workspace_freshness(facts, &mut out);
    }
    if want("org-drift") {
        org_drift(facts, &mut out);
    }
    if want("required-worktrees") {
        required_worktrees(facts, &mut out);
    }
    if want("required-files") {
        required_files(facts, &mut out);
    }
    out.sort_by(|a, b| {
        a.check
            .cmp(b.check)
            .then_with(|| a.repo.cmp(&b.repo))
            .then_with(|| a.message.cmp(&b.message))
    });
    out
}

fn gitignore_coverage(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    match &f.gitignore {
        None => push(
            out,
            cfg,
            "gitignore-coverage",
            Severity::Error,
            None,
            ".gitignore missing — run `lasius init`".into(),
        ),
        Some(text) => {
            // Accept `dir`, `/dir`, `dir/`, `/dir/` forms.
            let has = |needle: &str| {
                text.lines()
                    .any(|l| l.trim().trim_end_matches('/') == needle.trim_end_matches('/'))
            };
            if !has(&format!("/{}", cfg.repos_dir)) {
                push(
                    out,
                    cfg,
                    "gitignore-coverage",
                    Severity::Error,
                    None,
                    format!(".gitignore does not ignore /{}/", cfg.repos_dir),
                );
            }
            if !has(&cfg.workspace_file_name()) {
                push(
                    out,
                    cfg,
                    "gitignore-coverage",
                    Severity::Warn,
                    None,
                    format!(".gitignore does not ignore {}", cfg.workspace_file_name()),
                );
            }
        }
    }
}

fn docs_stubs(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    if !f.readme {
        push(
            out,
            cfg,
            "docs-stubs",
            Severity::Warn,
            None,
            "README.md missing".into(),
        );
    }
    if !f.agents {
        push(
            out,
            cfg,
            "docs-stubs",
            Severity::Warn,
            None,
            "AGENTS.md missing".into(),
        );
    }
}

fn repos_dir(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    if !f.repos_dir_exists {
        push(
            out,
            cfg,
            "repos-dir",
            Severity::Info,
            None,
            format!("{}/ does not exist yet — nothing cloned", cfg.repos_dir),
        );
    }
}

fn clone_validity(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    for name in &f.local_names {
        if !crate::names::is_safe_repo_name(name) {
            push(
                out,
                cfg,
                "clone-validity",
                Severity::Error,
                Some(name.clone()),
                "unsafe directory name under repos_dir".into(),
            );
            continue;
        }
        let facts = f.repos.iter().find(|r| &r.name == name);
        match facts {
            Some(r) if !r.is_git => push(
                out,
                cfg,
                "clone-validity",
                Severity::Error,
                Some(name.clone()),
                "directory exists but is not a git clone".into(),
            ),
            _ => {}
        }
    }
}

fn remote_match(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    let expected_suffix = format!("{}/", cfg.org);
    for r in &f.repos {
        if !r.is_git {
            continue;
        }
        let Some(remote) = &r.remote else {
            push(
                out,
                cfg,
                "remote-match",
                Severity::Warn,
                Some(r.name.clone()),
                "no origin remote".into(),
            );
            continue;
        };
        // Match `org/<name>` at the end of the remote URL (ssh or https).
        let tail = format!("{}/{}", cfg.org, r.name);
        if !remote.ends_with(&tail) && !remote.contains(&expected_suffix) {
            let base = if cfg.policy.remote_must_match_org {
                Severity::Error
            } else {
                Severity::Warn
            };
            push(
                out,
                cfg,
                "remote-match",
                base,
                Some(r.name.clone()),
                format!("origin is {remote}, expected …/{tail}"),
            );
        }
    }
}

fn remote_reachable(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    for r in &f.repos {
        // No origin → remote-match already reports it; not a clone → skip.
        if !r.is_git || r.remote.is_none() {
            continue;
        }
        if r.remote_ok == Some(false) {
            let mut msg =
                "origin unreachable — credentials/permissions problem, or remote renamed?"
                    .to_string();
            if let Some(err) = r.remote_error.as_deref().filter(|e| !e.is_empty()) {
                msg.push_str(&format!(" ({err})"));
            }
            push(
                out,
                cfg,
                "remote-reachable",
                Severity::Warn,
                Some(r.name.clone()),
                msg,
            );
        }
    }
}

fn worktree_health(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    for r in &f.repos {
        for path in &r.broken_worktrees {
            push(
                out,
                cfg,
                "worktree-health",
                Severity::Error,
                Some(r.name.clone()),
                format!("linked worktree path missing: {path} — run `git worktree repair`"),
            );
        }
    }
}

fn nested_git(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    for r in &f.repos {
        if r.has_nested_git {
            push(
                out,
                cfg,
                "nested-git",
                Severity::Warn,
                Some(r.name.clone()),
                "nested .git directory inside clone".into(),
            );
        }
    }
}

fn workspace_freshness(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    match &f.workspace_file {
        None => push(
            out,
            cfg,
            "workspace-freshness",
            Severity::Error,
            None,
            format!(
                "{} missing — run `lasius code-workspace generate`",
                cfg.workspace_file_name()
            ),
        ),
        Some(_) if !f.workspace_fresh => push(
            out,
            cfg,
            "workspace-freshness",
            Severity::Error,
            None,
            format!(
                "{} is stale — run `lasius code-workspace generate`",
                cfg.workspace_file_name()
            ),
        ),
        Some(_) => {}
    }
}

fn org_drift(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    let Some(org) = &f.org_names else {
        let msg = match &f.org_error {
            Some(err) => {
                let hint = match crate::failure::classify(err) {
                    crate::failure::FailureKind::AccessDenied => match cfg.provider {
                        crate::ProviderKind::Github => {
                            " — looks like an auth/permission problem; check `gh auth status`"
                        }
                    },
                    crate::failure::FailureKind::NotFound => {
                        " — org may not exist or the token can't see it"
                    }
                    crate::failure::FailureKind::Other => "",
                };
                format!("org discovery failed: {err}{hint} — drift check skipped")
            }
            None => "org repo list unavailable — drift check skipped".into(),
        };
        push(out, cfg, "org-drift", Severity::Info, None, msg);
        return;
    };
    let org_set: std::collections::BTreeSet<&str> = org
        .iter()
        .filter(|n| !cfg.is_excluded(n))
        .map(|s| s.as_str())
        .collect();
    let local_set: std::collections::BTreeSet<&str> =
        f.local_names.iter().map(|s| s.as_str()).collect();
    for missing in org_set.difference(&local_set) {
        push(
            out,
            cfg,
            "org-drift",
            Severity::Error,
            None,
            format!("org repo {missing} not cloned — run `lasius clone`"),
        );
    }
    for extra in local_set.difference(&org_set) {
        push(
            out,
            cfg,
            "org-drift",
            Severity::Warn,
            Some((*extra).to_string()),
            "local clone not in org (archived or deleted upstream?)".into(),
        );
    }
}

fn required_worktrees(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    for want in &cfg.policy.require_worktrees {
        for r in &f.repos {
            if !r.worktree_names.iter().any(|w| w == want) {
                push(
                    out,
                    cfg,
                    "required-worktrees",
                    Severity::Warn,
                    Some(r.name.clone()),
                    format!("missing required worktree `{want}`"),
                );
            }
        }
    }
}

fn required_files(f: &Facts, out: &mut Vec<Finding>) {
    let cfg = f.config;
    for r in &f.repos {
        for missing in &r.missing_files {
            push(
                out,
                cfg,
                "required-files",
                Severity::Warn,
                Some(r.name.clone()),
                format!("missing required file `{missing}`"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn cfg(toml: &str) -> WorkspaceConfig {
        WorkspaceConfig::from_toml(toml).unwrap()
    }

    fn base_facts<'a>(config: &'a WorkspaceConfig) -> Facts<'a> {
        Facts {
            config,
            gitignore: Some("/repos/\no.code-workspace\n".into()),
            readme: true,
            agents: true,
            repos_dir_exists: true,
            local_names: vec![],
            workspace_file: Some("{}".into()),
            workspace_fresh: true,
            org_names: Some(vec![]),
            org_error: None,
            repos: vec![],
        }
    }

    #[test]
    fn clean_workspace_has_no_findings() {
        let c = cfg("org = \"o\"\n");
        let f = base_facts(&c);
        assert!(run(&f, None).is_empty());
    }

    #[test]
    fn gitignore_missing_is_error() {
        let c = cfg("org = \"o\"\n");
        let mut f = base_facts(&c);
        f.gitignore = None;
        let v = run(&f, None);
        assert!(v
            .iter()
            .any(|x| x.check == "gitignore-coverage" && x.severity == Severity::Error));
    }

    #[test]
    fn missing_repos_dir_entry_flagged() {
        let c = cfg("org = \"o\"\nrepos_dir = \"src\"\n");
        let mut f = base_facts(&c);
        f.gitignore = Some("/repos/\n".into());
        let v = run(&f, None);
        assert!(v.iter().any(|x| x.message.contains("/src/")));
    }

    #[test]
    fn broken_worktree_is_error() {
        let c = cfg("org = \"o\"\n");
        let mut f = base_facts(&c);
        f.repos.push(RepoFacts {
            name: "a".into(),
            is_git: true,
            broken_worktrees: vec!["/gone/path".into()],
            ..Default::default()
        });
        let v = run(&f, None);
        assert!(v
            .iter()
            .any(|x| x.check == "worktree-health" && x.repo.as_deref() == Some("a")));
    }

    #[test]
    fn remote_match_respects_org() {
        let c = cfg("org = \"acme\"\n");
        let mut f = base_facts(&c);
        f.repos.push(RepoFacts {
            name: "ui".into(),
            is_git: true,
            remote: Some("git@github.com:other/ui.git".into()),
            ..Default::default()
        });
        let v = run(&f, None);
        assert!(v
            .iter()
            .any(|x| x.check == "remote-match" && x.severity == Severity::Warn));
    }

    #[test]
    fn remote_match_ok_for_org() {
        let c = cfg("org = \"acme\"\n");
        let mut f = base_facts(&c);
        f.repos.push(RepoFacts {
            name: "ui".into(),
            is_git: true,
            remote: Some("git@github.com:acme/ui.git".into()),
            ..Default::default()
        });
        assert!(!run(&f, None).iter().any(|x| x.check == "remote-match"));
    }

    #[test]
    fn drift_flags_missing_and_extra() {
        let c = cfg("org = \"o\"\nexclude = [\"skipme\"]\n");
        let mut f = base_facts(&c);
        f.org_names = Some(vec!["a".into(), "skipme".into(), "b".into()]);
        f.local_names = vec!["a".into(), "z".into()];
        let v = run(&f, None);
        assert!(v
            .iter()
            .any(|x| x.message.contains("b") && x.severity == Severity::Error));
        assert!(v
            .iter()
            .any(|x| x.repo.as_deref() == Some("z") && x.severity == Severity::Warn));
        assert!(!v.iter().any(|x| x.message.contains("skipme")));
    }

    #[test]
    fn policy_requirements_emit_warnings() {
        let c = cfg("org = \"o\"\n[policy]\nrequire_worktrees = [\"project\"]\nrequire_files = [\"AGENTS.md\"]\n");
        let mut f = base_facts(&c);
        f.repos.push(RepoFacts {
            name: "a".into(),
            is_git: true,
            missing_files: vec!["AGENTS.md".into()],
            ..Default::default()
        });
        let v = run(&f, None);
        assert!(v.iter().any(|x| x.check == "required-worktrees"));
        assert!(v.iter().any(|x| x.check == "required-files"));
    }

    #[test]
    fn remote_reachable_flags_unreachable_origin() {
        let c = cfg("org = \"o\"\n");
        let mut f = base_facts(&c);
        f.repos.push(RepoFacts {
            name: "a".into(),
            is_git: true,
            remote: Some("git@github.com:o/a.git".into()),
            remote_ok: Some(false),
            remote_error: Some("Permission denied (publickey)".into()),
            ..Default::default()
        });
        let v = run(&f, None);
        let hit = v
            .iter()
            .find(|x| x.check == "remote-reachable" && x.repo.as_deref() == Some("a"));
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().severity, Severity::Warn);
        assert!(hit.unwrap().message.contains("Permission denied"));
    }

    #[test]
    fn remote_reachable_skips_unprobed_and_reachable() {
        let c = cfg("org = \"o\"\n");
        let mut f = base_facts(&c);
        for (name, remote_ok) in [("ok", Some(true)), ("skip", None)] {
            f.repos.push(RepoFacts {
                name: name.into(),
                is_git: true,
                remote: Some("git@github.com:o/x.git".into()),
                remote_ok,
                ..Default::default()
            });
        }
        // no origin at all → remote-match covers it, remote-reachable stays silent
        f.repos.push(RepoFacts {
            name: "noremote".into(),
            is_git: true,
            ..Default::default()
        });
        assert!(!run(&f, None).iter().any(|x| x.check == "remote-reachable"));
    }

    #[test]
    fn org_drift_reports_discovery_error() {
        let c = cfg("org = \"o\"\n");
        let mut f = base_facts(&c);
        f.org_names = None;
        f.org_error = Some(
            "gh repo list failed with exit code 1: gh: not logged in — run `gh auth login`".into(),
        );
        let v = run(&f, None);
        let hit = v.iter().find(|x| x.check == "org-drift").unwrap();
        assert_eq!(hit.severity, Severity::Info);
        assert!(hit.message.contains("org discovery failed"));
        assert!(hit.message.contains("auth/permission"));
    }

    #[test]
    fn org_drift_generic_when_no_error_detail() {
        let c = cfg("org = \"o\"\n");
        let mut f = base_facts(&c);
        f.org_names = None;
        let v = run(&f, None);
        assert!(v
            .iter()
            .any(|x| x.check == "org-drift" && x.message.contains("unavailable")));
    }

    #[test]
    fn disabled_and_only_filters() {
        let c = cfg("org = \"o\"\n[checks]\ndisabled = [\"docs-stubs\"]\n");
        let mut f = base_facts(&c);
        f.readme = false;
        f.agents = false;
        assert!(!run(&f, None).iter().any(|x| x.check == "docs-stubs"));
        let only = vec!["docs-stubs".to_string()];
        assert!(run(&f, Some(&only)).is_empty());
    }

    #[test]
    fn severity_override_upgrades() {
        let c = cfg("org = \"o\"\n[checks]\nseverity = { nested-git = \"error\" }\n");
        let mut f = base_facts(&c);
        f.repos.push(RepoFacts {
            name: "a".into(),
            has_nested_git: true,
            ..Default::default()
        });
        let v = run(&f, None);
        assert!(v.iter().any(|x| x.severity == Severity::Error));
    }
}
