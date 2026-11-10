//! Command implementations. Each returns `Ok(())` or an error that main
//! prints as `error: …` with exit code 1.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command as ProcCommand;

use anyhow::{Context, Result};
use lasius_core::{
    classify_failure, parse_git_status, tail_text, FailureKind, Repo, StatusRow, WorkspaceConfig,
};

use crate::proc::{self, ProcOutput};
use crate::support::{self, Workspace, CONFIG_FILE};

fn git(repo: &Path, args: &[&str]) -> Result<ProcOutput> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    proc::capture_labeled("git", "git", &owned, Some(repo))
}

/// Format raw process output as a failure detail: classified with an
/// `access-denied:` / `not-found:` prefix when recognized, otherwise the
/// raw text — always capped to the last ~300 chars.
fn failure_detail(raw: &str) -> String {
    let snippet = tail_text(raw.trim(), 300);
    if snippet.is_empty() {
        return snippet;
    }
    match classify_failure(raw) {
        FailureKind::AccessDenied => format!("access-denied: {snippet}"),
        FailureKind::NotFound => format!("not-found: {snippet}"),
        FailureKind::Other => snippet,
    }
}

/// stderr preferred, stdout as fallback — the usual git/gh failure shape.
fn failure_text(o: &ProcOutput) -> &str {
    if o.stderr.trim().is_empty() {
        o.stdout.trim()
    } else {
        o.stderr.trim()
    }
}

/// Outcome of attempting one clone.
enum CloneOutcome {
    Cloned,
    Skipped,
    Warned,
    Failed(String),
}

/// Clone every repo in `repos` that isn't already present. Prints per-repo
/// lines in org order, refreshes the workspace file, returns failure count.
fn clone_all(ws: &Workspace, repos: &[Repo]) -> Result<()> {
    fs::create_dir_all(ws.repos_path())
        .with_context(|| format!("cannot create {}", ws.repos_path().display()))?;

    let outcomes = proc::par_map(repos, |repo| {
        let target = ws.repo_path(&repo.name);
        let git_dir = target.join(".git");
        // exists() on a path inside a plain file errors — treat probe failure
        // as "absent" so a stray file just warns.
        let has_git = git_dir.try_exists().unwrap_or(false);
        if has_git {
            return (repo.name.clone(), CloneOutcome::Skipped);
        }
        if target.try_exists().unwrap_or(false) {
            return (repo.name.clone(), CloneOutcome::Warned);
        }
        let out = proc::capture(
            "git",
            &[
                "clone".to_string(),
                repo.clone_url.clone(),
                target.to_string_lossy().into_owned(),
            ],
            None,
        );
        match out {
            Ok(o) if o.success() => (repo.name.clone(), CloneOutcome::Cloned),
            Ok(o) => {
                let detail = failure_detail(failure_text(&o));
                (repo.name.clone(), CloneOutcome::Failed(detail))
            }
            Err(e) => (repo.name.clone(), CloneOutcome::Failed(e.to_string())),
        }
    });

    let mut cloned = 0;
    let mut skipped = 0;
    let mut warned = 0;
    let mut failures: Vec<String> = Vec::new();
    for (name, outcome) in outcomes {
        match outcome {
            CloneOutcome::Cloned => {
                println!("cloned  {name}");
                cloned += 1;
            }
            CloneOutcome::Skipped => {
                println!("skip    {name} (already cloned)");
                skipped += 1;
            }
            CloneOutcome::Warned => {
                println!("warn    {name} exists but is not a git repo — leaving it alone");
                warned += 1;
            }
            CloneOutcome::Failed(detail) => {
                eprintln!("FAIL    {name} {detail}");
                failures.push(format!("{name}: {detail}"));
            }
        }
    }
    println!(
        "done: {cloned} cloned, {skipped} already present, {warned} warned, {} failed",
        failures.len()
    );

    // Always refresh — partial clones are still valid workspace members.
    support::refresh_workspace(ws)?;

    if !failures.is_empty() {
        anyhow::bail!(
            "failed to clone {} repo(s): {}",
            failures.len(),
            failures.join("; ")
        );
    }
    Ok(())
}

/// `lasius init` — scaffold the workspace repo, then clone + generate.
///
/// Creates `lasius.toml`, `.gitignore` entries, and README/AGENTS stubs
/// when missing (all idempotent), then runs the same clone-all flow as
/// `lasius clone`.
pub fn init(root_flag: Option<&Path>, org: Option<String>, cwd: &Path) -> Result<()> {
    let root = root_flag
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cwd.to_path_buf());
    let config_path = root.join(CONFIG_FILE);

    if !config_path.is_file() {
        let org = org
            .or_else(|| infer_org_from_remote(&root))
            .context("no lasius.toml found and no --org given — pass --org <org>")?;
        let config = WorkspaceConfig {
            org,
            provider: Default::default(),
            repos_dir: "repos".to_string(),
            workspace_file: None,
            exclude: Vec::new(),
            include_private: false,
            checks: Default::default(),
            policy: Default::default(),
        };
        fs::write(&config_path, config.to_toml())
            .with_context(|| format!("cannot write {}", config_path.display()))?;
        println!("wrote {}", config_path.display());
    }

    let ws = support::load(&root)?;

    // .gitignore: ensure repos_dir + workspace file are ignored.
    let needed = [
        format!("/{}/", ws.config.repos_dir),
        ws.config.workspace_file_name(),
    ];
    let gitignore = root.join(".gitignore");
    let existing = fs::read_to_string(&gitignore).unwrap_or_default();
    let present: std::collections::BTreeSet<&str> = existing.lines().map(str::trim).collect();
    let missing: Vec<&String> = needed
        .iter()
        .filter(|p| !present.contains(p.as_str()))
        .collect();
    if !missing.is_empty() {
        let mut content = existing;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("\n# Managed by lasius\n");
        for p in &missing {
            content.push_str(p);
            content.push('\n');
        }
        fs::write(&gitignore, content)
            .with_context(|| format!("cannot write {}", gitignore.display()))?;
        println!("updated {} ({})", gitignore.display(), missing.len());
    }

    // Doc stubs — only when absent, never overwrite.
    for (name, body) in [
        ("README.md", readme_stub(&ws.config.org)),
        ("AGENTS.md", agents_stub(&ws.config.org)),
    ] {
        let path = root.join(name);
        if !path.exists() {
            fs::write(&path, body).with_context(|| format!("cannot write {}", path.display()))?;
            println!("wrote {}", path.display());
        }
    }

    let repos = support::org_repos(&ws)?;
    clone_all(&ws, &repos)
}

fn readme_stub(org: &str) -> String {
    format!(
        "# {org} workspace\n\n\
         Metarepo for the [{org}](https://github.com/{org}) GitHub org — config \
         only, managed by [lasius](https://github.com/9aia/lasius).\n\n\
         `repos/` holds independent clones (gitignored); `*.code-workspace` is \
         generated. See `lasius.toml` and `lasius --help`.\n"
    )
}

fn agents_stub(org: &str) -> String {
    format!(
        "# {org} workspace — agent guide\n\n\
         Metarepo root for the {org} org. Config only — repo management is \
         delegated to [lasius](https://github.com/9aia/lasius) via \
         `lasius.toml`. All org repositories are cloned under `repos/` \
         (gitignored); each is an independent repo with its own `AGENTS.md` — \
         read it before working inside.\n\n\
         ## Workspace commands\n\n\
         - `lasius init` — scaffold + clone all org repos + generate the \
         workspace file\n\
         - `lasius update` — fast-forward pull every clone\n\
         - `lasius status` — branch / dirty / sync matrix (`--json`)\n\
         - `lasius code-workspace generate` — regenerate the workspace file\n\
         - `lasius doctor` — env + drift diagnostics\n\
         - `lasius exec -- <cmd>` — run a command in every clone\n\
         - `lasius rm <repo>` — remove a clone after safety checks\n"
    )
}

/// Best-effort owner inference from `origin`'s github URL. GitHub-specific —
/// only used by `init` when no `lasius.toml` exists yet, i.e. when the
/// provider is necessarily the default (`github`).
fn infer_org_from_remote(root: &Path) -> Option<String> {
    let out = proc::capture(
        "git",
        &[
            "remote".to_string(),
            "get-url".to_string(),
            "origin".to_string(),
        ],
        Some(root),
    )
    .ok()?;
    if !out.success() {
        return None;
    }
    let url = out.stdout.trim();
    // git@github.com:owner/repo.git or https://github.com/owner/repo(.git)?
    let rest = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    rest.split('/').next().map(str::to_string)
}

/// `lasius clone`.
pub fn clone(ws: &Workspace) -> Result<()> {
    let repos = support::org_repos(ws)?;
    clone_all(ws, &repos)
}

/// One repo's pull outcome for `--json` output.
#[derive(serde::Serialize)]
pub struct UpdateRow {
    name: String,
    ok: bool,
    detail: String,
}

/// `lasius update` (alias `sync`) — pull --ff-only everywhere.
pub fn update(ws: &Workspace, json: bool) -> Result<()> {
    let names = support::local_repos(ws)?;
    let results = proc::par_map(&names, |name| {
        let dir = ws.repo_path(name);
        let out = git(&dir, &["pull", "--ff-only"]);
        (name.clone(), out)
    });

    let mut rows = Vec::new();
    let mut failed = 0;
    for (name, out) in &results {
        let (ok, detail) = match out {
            Ok(o) if o.success() => (true, o.last_stdout_line().to_string()),
            Ok(o) => {
                let detail = failure_detail(failure_text(o));
                (false, detail)
            }
            Err(e) => (false, e.to_string()),
        };
        if !ok {
            failed += 1;
        }
        rows.push(UpdateRow {
            name: name.clone(),
            ok,
            detail,
        });
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for r in &rows {
            println!(
                "{:<8}{:<22} {}",
                if r.ok { "ok" } else { "FAIL" },
                r.name,
                r.detail
            );
        }
    }
    if failed > 0 {
        anyhow::bail!("{failed} repo(s) failed to update");
    }
    Ok(())
}

/// A per-repo status failure: repo name + human-readable detail.
pub type StatusFailure = (String, String);

/// Collect per-repo status rows plus failure pairs.
/// Shared by `status` (prints) and `serve status` (JSON).
pub fn collect_status(ws: &Workspace) -> Result<(Vec<StatusRow>, Vec<StatusFailure>)> {
    let names = support::local_repos(ws)?;
    let results = proc::par_map(&names, |name| {
        let dir = ws.repo_path(name);
        let out = git(&dir, &["status", "--porcelain=v2", "--branch"]);
        (name.clone(), out)
    });

    let mut rows: Vec<StatusRow> = Vec::new();
    let mut failures: Vec<StatusFailure> = Vec::new();
    for (name, out) in results {
        match out {
            Ok(o) if o.success() => {
                rows.push(StatusRow {
                    name,
                    status: parse_git_status(&o.stdout),
                });
            }
            Ok(o) => {
                let detail = format!(
                    "git status {name} failed with exit code {}: {}",
                    o.code,
                    failure_detail(failure_text(&o))
                );
                failures.push((name, detail));
            }
            Err(e) => {
                let detail = format!("git status {name}: {e}");
                failures.push((name, detail));
            }
        }
    }
    Ok((rows, failures))
}

/// Status rows only — bails if any repo failed (used by `serve status`).
pub fn status_rows(ws: &Workspace) -> Result<Vec<StatusRow>> {
    let (rows, failures) = collect_status(ws)?;
    if !failures.is_empty() {
        let detail = failures
            .iter()
            .map(|(n, d)| format!("{n}: {d}"))
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!("{} repo(s) failed status: {detail}", failures.len());
    }
    Ok(rows)
}

/// `lasius status` — table (or `--json`) plus per-repo ERR lines.
pub fn status(ws: &Workspace, json: bool) -> Result<()> {
    let (rows, failures) = collect_status(ws)?;
    for (name, detail) in &failures {
        eprintln!("ERR     {:<22} {detail}", name);
    }

    if json {
        let doc: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "branch": r.status.branch,
                    "upstream": r.status.upstream,
                    "ahead": r.status.ahead,
                    "behind": r.status.behind,
                    "dirty": r.status.dirty,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&doc)?);
    } else {
        println!("{}", lasius_core::format_status_table(&rows));
    }

    if !failures.is_empty() {
        let detail = failures
            .iter()
            .map(|(n, d)| format!("{n}: {d}"))
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!("{} repo(s) failed status: {detail}", failures.len());
    }
    Ok(())
}

/// `lasius code-workspace generate`.
pub fn code_workspace_generate(ws: &Workspace, json: bool) -> Result<()> {
    let count = support::refresh_workspace(ws)?;
    let path = ws.workspace_file_path();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "path": path,
                "repos": count,
            }))?
        );
    } else {
        println!("wrote {} ({count} repo folders)", path.display());
    }
    Ok(())
}

/// Doctor check rows `(name, ok, detail)` — shared by `doctor` (prints)
/// and `serve doctor` (JSON). No printing happens here.
pub fn doctor_checks(ws: &Workspace) -> Result<Vec<(String, bool, String)>> {
    let mut checks: Vec<(String, bool, String)> = Vec::new();

    // git
    match proc::capture("git", &["--version".to_string()], None) {
        Ok(o) if o.success() => {
            checks.push(("git".into(), true, o.stdout.trim().to_string()));
        }
        Ok(o) => checks.push(("git".into(), false, o.stderr.trim().to_string())),
        Err(e) => checks.push(("git".into(), false, e.to_string())),
    }

    // provider auth probe + org repo list
    let provider = crate::providers::for_kind(ws.config.provider);
    let (probe_prog, probe_args) = provider.auth_probe();
    match proc::capture(probe_prog, &probe_args, None) {
        Ok(o) if o.success() => {
            checks.push((provider.auth_check().into(), true, "authenticated".into()));
        }
        Ok(o) => checks.push((
            provider.auth_check().into(),
            false,
            format!("exit {}: {}", o.code, o.stderr.trim()),
        )),
        Err(e) => checks.push((provider.auth_check().into(), false, e.to_string())),
    }

    let org = match support::org_repos(ws) {
        Ok(repos) => {
            checks.push((
                provider.list_label().into(),
                true,
                format!("{} repos", repos.len()),
            ));
            Some(repos)
        }
        Err(e) => {
            checks.push((provider.list_label().into(), false, format!("{e:#}")));
            None
        }
    };

    // drift: org vs local
    if let Ok(local) = support::local_repos(ws) {
        if let Some(repos) = &org {
            let org_names: std::collections::BTreeSet<&str> =
                repos.iter().map(|r| r.name.as_str()).collect();
            let local_set: std::collections::BTreeSet<&str> =
                local.iter().map(|s| s.as_str()).collect();
            let missing: Vec<&&str> = org_names.difference(&local_set).collect();
            let extra: Vec<&&str> = local_set.difference(&org_names).collect();
            let ok = missing.is_empty() && extra.is_empty();
            let names = |v: &[&&str]| {
                v.iter()
                    .take(5)
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let detail = format!(
                "{} missing locally ({}), {} not in org ({})",
                missing.len(),
                names(&missing),
                extra.len(),
                names(&extra)
            );
            checks.push(("drift".into(), ok, detail));
        }
        checks.push(("local clones".into(), true, format!("{}", local.len())));
    }

    // workspace file freshness
    let ws_path = ws.workspace_file_path();
    match fs::read_to_string(&ws_path) {
        Ok(on_disk) => {
            let names = support::local_repos(ws).unwrap_or_default();
            let fresh =
                lasius_core::build_workspace_file(&ws.config.org, &ws.config.repos_dir, &names);
            checks.push((
                "workspace file".into(),
                on_disk == fresh,
                if on_disk == fresh {
                    "up to date".into()
                } else {
                    "stale — run `lasius code-workspace generate`".into()
                },
            ));
        }
        Err(_) => checks.push((
            "workspace file".into(),
            false,
            format!(
                "{} missing — run `lasius code-workspace generate`",
                ws_path.display()
            ),
        )),
    }

    Ok(checks)
}

/// `lasius doctor` — environment + drift diagnostics. Exit 1 on any failure.
pub fn doctor(ws: &Workspace, json: bool) -> Result<()> {
    let checks = doctor_checks(ws)?;
    let failed = checks.iter().filter(|(_, ok, _)| !*ok).count();
    if json {
        let doc: Vec<serde_json::Value> = checks
            .iter()
            .map(|(name, ok, detail)| serde_json::json!({"name": name, "ok": ok, "detail": detail}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&doc)?);
    } else {
        for (name, ok, detail) in &checks {
            println!(
                "{} {:<14} {}",
                if *ok { "ok  " } else { "FAIL" },
                name,
                detail
            );
        }
    }
    if failed > 0 {
        anyhow::bail!("{failed} check(s) failed");
    }
    Ok(())
}

/// `lasius open` — generate the workspace file if needed, then open it.
pub fn open(ws: &Workspace, editor: Option<String>) -> Result<()> {
    let path = ws.workspace_file_path();
    if !path.is_file() {
        support::refresh_workspace(ws)?;
    }
    let editor = editor
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| {
            for candidate in ["code", "zed", "cursor"] {
                if which(candidate) {
                    return candidate.to_string();
                }
            }
            "vi".to_string()
        });

    let is_gui = ["code", "zed", "cursor", "code-insiders"].contains(&editor.as_str());
    let mut cmd = ProcCommand::new(&editor);
    cmd.arg(&path);
    if is_gui {
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
    }
    let status = cmd
        .status()
        .with_context(|| format!("cannot launch editor {editor:?}"))?;
    if !status.success() {
        anyhow::bail!("editor {editor:?} exited with {status}");
    }
    Ok(())
}

fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

/// `lasius rm <name>` — remove a clone after safety checks.
pub fn rm(ws: &Workspace, name: &str, yes: bool, force: bool) -> Result<()> {
    if !lasius_core::names::is_safe_repo_name(name) {
        anyhow::bail!("unsafe repo name: {name:?}");
    }
    let dir = ws.repo_path(name);
    if !dir.join(".git").exists() {
        anyhow::bail!("{} is not a lasius clone (no .git)", dir.display());
    }

    let mut problems: Vec<String> = Vec::new();
    if !force {
        // dirty tree
        if let Ok(out) = git(&dir, &["status", "--porcelain"]) {
            if out.success() && !out.stdout.trim().is_empty() {
                problems.push("working tree is dirty".into());
            }
        }
        // unpushed commits
        if let Ok(out) = git(
            &dir,
            &["log", "--branches", "--not", "--remotes", "--oneline"],
        ) {
            if out.success() && !out.stdout.trim().is_empty() {
                problems.push("has unpushed commits".into());
            }
        }
        // stashes
        if let Ok(out) = git(&dir, &["stash", "list"]) {
            if out.success() && !out.stdout.trim().is_empty() {
                problems.push("has stashes".into());
            }
        }
        // linked worktrees
        if let Ok(out) = git(&dir, &["worktree", "list"]) {
            if out.success() && out.stdout.lines().count() > 1 {
                problems.push("has linked worktrees".into());
            }
        }
        // extra remotes
        if let Ok(out) = git(&dir, &["remote"]) {
            if out.success() && out.stdout.lines().count() > 1 {
                problems.push("has extra remotes".into());
            }
        }
    }

    if !problems.is_empty() {
        for p in &problems {
            eprintln!("blocker: {p}");
        }
        anyhow::bail!("refusing to remove {name} — resolve or re-run with --force");
    }

    if !yes {
        eprint!("remove {}? [y/N] ", dir.display());
        std::io::stderr().flush().ok();
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            println!("aborted");
            return Ok(());
        }
    }

    fs::remove_dir_all(&dir).with_context(|| format!("cannot remove {}", dir.display()))?;
    println!("removed {}", dir.display());
    let count = support::refresh_workspace(ws)?;
    println!("workspace file regenerated ({count} repo folders)");
    Ok(())
}

/// `lasius exec -- <cmd>` — run a shell command in every (filtered) clone.
pub fn exec(ws: &Workspace, filter: Option<&str>, cmd: &[String]) -> Result<()> {
    if cmd.is_empty() {
        anyhow::bail!("exec requires a command: `lasius exec -- <cmd> [args…]`");
    }
    let names = support::local_repos(ws)?;
    let selected: Vec<String> = match filter {
        Some(f) => names.into_iter().filter(|n| n.contains(f)).collect(),
        None => names,
    };
    if selected.is_empty() {
        println!("no matching clones");
        return Ok(());
    }

    let command_line = cmd.join(" ");
    let results = proc::par_map(&selected, |name| {
        let dir = ws.repo_path(name);
        let out = proc::capture("sh", &["-c".to_string(), command_line.clone()], Some(&dir));
        (name.clone(), out)
    });

    let mut failed = 0;
    for (name, out) in results {
        println!("== {name} ==");
        match out {
            Ok(o) => {
                if !o.stdout.is_empty() {
                    print!("{}", o.stdout);
                }
                if !o.stderr.is_empty() {
                    eprint!("{}", o.stderr);
                }
                if !o.success() {
                    println!("-- exit {}", o.code);
                    failed += 1;
                }
            }
            Err(e) => {
                eprintln!("{e}");
                failed += 1;
            }
        }
    }
    if failed > 0 {
        anyhow::bail!("{failed} repo(s) failed");
    }
    Ok(())
}

/// `lasius add <name>` — clone one org repo by name, then refresh workspace.
pub fn add(ws: &Workspace, name: &str) -> Result<()> {
    if !lasius_core::names::is_safe_repo_name(name) {
        anyhow::bail!("unsafe repo name: {name:?}");
    }
    let repos = support::org_repos(ws)?;
    let repo = repos.iter().find(|r| r.name == name).with_context(|| {
        format!(
            "{name} not found in org {} (or is archived/excluded)",
            ws.config.org
        )
    })?;
    clone_all(ws, std::slice::from_ref(repo))
}

/// Facts for one clone — all git/fs probes done here, in parallel.
fn repo_facts(ws: &Workspace, name: &str) -> lasius_core::RepoFacts {
    use lasius_core::RepoFacts;
    let dir = ws.repo_path(name);
    let mut f = RepoFacts {
        name: name.to_string(),
        is_git: dir.join(".git").exists(),
        ..Default::default()
    };
    if !f.is_git {
        return f;
    }
    f.remote = git(&dir, &["remote", "get-url", "origin"])
        .ok()
        .filter(|o| o.success())
        .map(|o| o.stdout.trim().to_string());

    // Probe reachability — GIT_TERMINAL_PROMPT=0 + ssh BatchMode keep
    // credential/host-key prompts from hanging the check. BatchMode also
    // overrides any user-set GIT_SSH for this probe only.
    if f.remote.is_some() {
        let out = proc::capture_env(
            "git",
            &[
                "ls-remote".to_string(),
                "origin".to_string(),
                "HEAD".to_string(),
            ],
            Some(&dir),
            &[
                ("GIT_TERMINAL_PROMPT", "0"),
                ("GIT_SSH_COMMAND", "ssh -o BatchMode=yes"),
            ],
        );
        match out {
            Ok(o) if o.success() => f.remote_ok = Some(true),
            Ok(o) => {
                f.remote_ok = Some(false);
                f.remote_error = Some(tail_text(failure_text(&o).trim(), 200));
            }
            // spawn itself failed — leave unprobed rather than guessing.
            Err(_) => {}
        }
    }

    // worktrees: porcelain "worktree <path>" lines; first is the main checkout
    if let Ok(o) = git(&dir, &["worktree", "list", "--porcelain"]) {
        if o.success() {
            let mut paths = Vec::new();
            for line in o.stdout.lines() {
                if let Some(p) = line.strip_prefix("worktree ") {
                    paths.push(p.to_string());
                }
            }
            f.worktree_names = paths
                .iter()
                .filter_map(|p| std::path::Path::new(p).file_name())
                .map(|n| n.to_string_lossy().to_string())
                .collect();
            f.broken_worktrees = paths
                .into_iter()
                .skip(1)
                .filter(|p| !std::path::Path::new(p).exists())
                .collect();
        }
    }

    f.has_nested_git = has_nested_git_dir(&dir);
    f.missing_files = ws
        .config
        .policy
        .require_files
        .iter()
        .filter(|file| !dir.join(file).exists())
        .cloned()
        .collect();
    f
}

/// Depth-limited scan for a `.git` *directory* below the top level
/// (worktrees/submodules use `.git` files, so dirs only = real nesting).
fn has_nested_git_dir(dir: &Path) -> bool {
    fn walk(dir: &Path, depth: u8) -> bool {
        if depth == 0 {
            return false;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            if name == ".git" && p.is_dir() {
                return true;
            }
            if p.is_dir()
                && !name.to_string_lossy().starts_with('.')
                && name != "target"
                && name != "node_modules"
                && walk(&p, depth - 1)
            {
                return true;
            }
        }
        false
    }
    // start below the repo root itself
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir()
                && e.file_name() != ".git"
                && !e.file_name().to_string_lossy().starts_with('.')
                && e.file_name() != "target"
                && e.file_name() != "node_modules"
                && walk(&p, 3)
            {
                return true;
            }
        }
    }
    false
}

/// `lasius check` — conformance findings over the workspace.
pub fn check(ws: &Workspace, domain: Option<&str>, only: &[String], json: bool) -> Result<()> {
    use lasius_core::{run_checks, Facts, Severity};

    let local_names = support::local_repos(ws)?;
    let repos_dir = ws.config.repos_path(&ws.root);
    let gitignore = fs::read_to_string(ws.root.join(".gitignore")).ok();
    let readme = ws.root.join("README.md").is_file();
    let agents = ws.root.join("AGENTS.md").is_file();

    let ws_path = ws.workspace_file_path();
    let workspace_file = fs::read_to_string(&ws_path).ok();
    let workspace_fresh = workspace_file.as_deref().is_some_and(|on_disk| {
        let fresh =
            lasius_core::build_workspace_file(&ws.config.org, &ws.config.repos_dir, &local_names);
        on_disk == fresh
    });

    let org_result = support::org_repos(ws);
    let org_names = org_result
        .as_ref()
        .ok()
        .map(|repos| repos.iter().map(|r| r.name.clone()).collect());
    let org_error = org_result.err().map(|e| tail_text(&format!("{e:#}"), 300));

    let repo_facts: Vec<lasius_core::RepoFacts> =
        proc::par_map(&local_names, |n| repo_facts(ws, n));

    let facts = Facts {
        config: &ws.config,
        gitignore,
        readme,
        agents,
        repos_dir_exists: repos_dir.is_dir(),
        local_names: local_names.clone(),
        workspace_file,
        workspace_fresh,
        org_names,
        org_error,
        repos: repo_facts,
    };

    // domain filter → check-id groups
    let only_ids: Vec<String> = if only.is_empty() {
        Vec::new()
    } else {
        only.to_vec()
    };
    let _ = domain; // domain selection lands with the env-domain merge
    let findings = run_checks(
        &facts,
        if only_ids.is_empty() {
            None
        } else {
            Some(&only_ids)
        },
    );

    if json {
        let doc: Vec<serde_json::Value> = findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "check": f.check,
                    "severity": match f.severity {
                        Severity::Info => "info",
                        Severity::Warn => "warn",
                        Severity::Error => "error",
                    },
                    "repo": f.repo,
                    "message": f.message,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&doc)?);
    } else {
        for f in &findings {
            let tag = match f.severity {
                Severity::Info => "info ",
                Severity::Warn => "warn ",
                Severity::Error => "ERROR",
            };
            match &f.repo {
                Some(r) => println!("{tag} {:<22} {}", r, f.message),
                None => println!("{tag} {:<22} {}", f.check, f.message),
            }
        }
        if findings.is_empty() {
            println!("all checks passed");
        }
    }

    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    if errors > 0 {
        anyhow::bail!("{errors} conformance check(s) failed");
    }
    Ok(())
}
