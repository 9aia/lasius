//! `lasius conform` — org docs contract scaffolding for clones.
//!
//! The workspace `AGENTS.md` "Docs contract" requires every repo to
//! carry a README skeleton, an AGENTS.md dev map, a `cypera.toml`
//! docs-generation config, `SECURITY.md`/`CONTRIBUTING.md`, and a
//! `docs/` VitePress worktree with a `docs:check` drift gate — and no
//! CHANGELOG. `conform` brings an existing clone up to that contract:
//! it writes **only missing files** (never overwrites), marks stubs
//! with TODOs where content needs human/agent input, and reports
//! drift it can't fix (a stray CHANGELOG, a missing docs worktree).
//!
//! `clone_all` calls [`scaffold_clones`] so `init`/`clone`/`add`
//! drop the stubs into every clone automatically.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::support::{self, Workspace};

/// What conform did (or would do, under `--dry-run`) to one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    /// File was missing and has been written.
    Created,
    /// File was missing; `--dry-run` kept it untouched.
    WouldCreate,
    /// File already exists — never overwritten.
    Present,
    /// Contract violation conform can't fix mechanically.
    Warn,
    /// FYI — e.g. no docs worktree to hang `docs/package.json` on.
    Info,
}

impl ItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "wrote",
            Self::WouldCreate => "would-write",
            Self::Present => "skip",
            Self::Warn => "warn",
            Self::Info => "info",
        }
    }
}

/// One conform finding for one repo path.
#[derive(Debug)]
pub struct ConformItem {
    /// Repo-relative path (`README.md`, `docs/package.json`, …).
    pub path: String,
    pub status: ItemStatus,
    pub note: String,
}

/// A file the contract wants, plus its stub body.
struct Stub {
    /// Repo-relative path.
    path: &'static str,
    body: String,
}

fn stubs(name: &str) -> Vec<Stub> {
    vec![
        Stub {
            path: "README.md",
            body: readme_stub(name),
        },
        Stub {
            path: "AGENTS.md",
            body: agents_stub(name),
        },
        Stub {
            path: "cypera.toml",
            body: cypera_stub(name),
        },
        Stub {
            path: "SECURITY.md",
            body: SECURITY_STUB.to_string(),
        },
        Stub {
            path: "CONTRIBUTING.md",
            body: contributing_stub(),
        },
    ]
}

/// Apply the docs contract to one clone directory. Returns the
/// findings; writes happen unless `dry_run`.
pub fn conform_repo(dir: &Path, name: &str, dry_run: bool) -> Result<Vec<ConformItem>> {
    let mut items = Vec::new();

    for stub in stubs(name) {
        let path = dir.join(stub.path);
        if path.exists() {
            items.push(ConformItem {
                path: stub.path.to_string(),
                status: ItemStatus::Present,
                note: "exists — not overwritten".into(),
            });
            continue;
        }
        if dry_run {
            items.push(ConformItem {
                path: stub.path.to_string(),
                status: ItemStatus::WouldCreate,
                note: "missing".into(),
            });
        } else {
            fs::write(&path, &stub.body)
                .with_context(|| format!("cannot write {}", path.display()))?;
            items.push(ConformItem {
                path: stub.path.to_string(),
                status: ItemStatus::Created,
                note: "scaffolded — TODOs need real content".into(),
            });
        }
    }

    // docs/ worktree: hang package.json on it only when the worktree
    // exists — conform never creates git worktrees itself.
    let docs = dir.join("docs");
    let pkg = docs.join("package.json");
    if docs.is_dir() {
        if pkg.exists() {
            items.push(ConformItem {
                path: "docs/package.json".into(),
                status: ItemStatus::Present,
                note: "exists — not overwritten".into(),
            });
        } else if dry_run {
            items.push(ConformItem {
                path: "docs/package.json".into(),
                status: ItemStatus::WouldCreate,
                note: "docs/ exists, package.json missing".into(),
            });
        } else {
            let body = docs_package_stub(name);
            fs::write(&pkg, body).with_context(|| format!("cannot write {}", pkg.display()))?;
            items.push(ConformItem {
                path: "docs/package.json".into(),
                status: ItemStatus::Created,
                note: "scaffolded — docs:check fails until generators are wired".into(),
            });
        }
    } else {
        items.push(ConformItem {
            path: "docs/".into(),
            status: ItemStatus::Info,
            note: "no docs/ worktree — create it per the repo's docs bootstrap".into(),
        });
    }

    // Contract says no CHANGELOG — flag it, don't delete (content may
    // need porting into release notes first).
    if dir.join("CHANGELOG.md").exists() {
        items.push(ConformItem {
            path: "CHANGELOG.md".into(),
            status: ItemStatus::Warn,
            note:
                "contract says no CHANGELOG — git history + tarsi release notes are the changelog"
                    .into(),
        });
    }

    Ok(items)
}

/// Conform every local clone; returns files written. Used by
/// `clone_all` — quiet per-file `wrote` lines, summary returned.
pub fn scaffold_clones(ws: &Workspace) -> Result<usize> {
    let names = support::local_repos(ws)?;
    let mut written = 0;
    for name in names {
        let dir = ws.repo_path(&name);
        for item in conform_repo(&dir, &name, false)? {
            if item.status == ItemStatus::Created {
                println!("wrote  {:<22} {}", name, item.path);
                written += 1;
            }
        }
    }
    Ok(written)
}

/// `lasius conform [name]` — bring clones up to the docs contract.
pub fn conform(ws: &Workspace, name: Option<&str>, dry_run: bool, json: bool) -> Result<()> {
    let names: Vec<String> = match name {
        Some(n) => {
            if !lasius_core::names::is_safe_repo_name(n) {
                anyhow::bail!("unsafe repo name: {n:?}");
            }
            let dir = ws.repo_path(n);
            if !dir.join(".git").exists() {
                anyhow::bail!("{n} is not a lasius clone (no .git)");
            }
            vec![n.to_string()]
        }
        None => support::local_repos(ws)?,
    };

    let mut all: Vec<(String, Vec<ConformItem>)> = Vec::new();
    for n in &names {
        let items = conform_repo(&ws.repo_path(n), n, dry_run)?;
        all.push((n.clone(), items));
    }

    if json {
        let doc: Vec<serde_json::Value> = all
            .iter()
            .flat_map(|(repo, items)| {
                items.iter().map(move |i| {
                    serde_json::json!({
                        "repo": repo,
                        "path": i.path,
                        "status": i.status.as_str(),
                        "note": i.note,
                    })
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&doc)?);
    } else {
        for (repo, items) in &all {
            println!("== {repo} ==");
            for i in items {
                let note = if i.note.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", i.note)
                };
                println!("{:<12} {:<24}{}", i.status.as_str(), i.path, note);
            }
        }
        let changed = all
            .iter()
            .flat_map(|(_, items)| items.iter())
            .filter(|i| matches!(i.status, ItemStatus::Created | ItemStatus::WouldCreate))
            .count();
        let warns = all
            .iter()
            .flat_map(|(_, items)| items.iter())
            .filter(|i| i.status == ItemStatus::Warn)
            .count();
        println!(
            "done: {changed} file(s) {}, {warns} warning(s)",
            if dry_run {
                "would be written"
            } else {
                "written"
            }
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Templates — stubs only. Every TODO marks content that needs real
// human/agent input; nothing here fabricates crate tables or status.
// ---------------------------------------------------------------------

fn readme_stub(name: &str) -> String {
    format!(
        "# {name}\n\n\
         <!-- TODO: what it is and why — one succinct paragraph.\n\
              No crate tables, verb dumps, or status essays; link\n\
              AGENTS.md / the docs site for depth. Verify status\n\
              claims against the code before writing them. -->\n\n\
         ## Quickstart\n\n\
         <!-- TODO: minimal build/test/run commands — crib from\n\
              AGENTS.md Commands; verify they work. -->\n\n\
         ## Docs\n\n\
         - [AGENTS.md](AGENTS.md) — agent/dev map: layout, commands,\n\
           boundary, rules\n\
         - `docs/` worktree — VitePress + Diátaxis site\n\
           <!-- TODO: public repos deploy to https://9aia.com/{name}/;\n\
                private repos keep the source, no deploy. -->\n"
    )
}

fn agents_stub(name: &str) -> String {
    format!(
        "# {name}\n\n\
         <!-- TODO: what it is — one paragraph (what/why/boundary in\n\
              brief). Naming note if the repo is named for something. -->\n\n\
         ## Layout\n\n\
         <!-- TODO: crate/module table or map — must match source\n\
              exactly; generate where a generator exists. -->\n\n\
         ## Commands\n\n\
         <!-- TODO: build/test/dev commands (cargo xtask …, bun …). -->\n\n\
         ## Boundary\n\n\
         <!-- TODO: what this repo owns vs its neighbors — what does\n\
              NOT belong here. -->\n\n\
         ## Rules\n\n\
         <!-- TODO: lint/test/convention rules agents must follow. -->\n\n\
         ## Docs\n\n\
         `docs/` worktree — VitePress + Diátaxis (getting-started,\n\
         how-to, reference, explanation)\n\
         <!-- TODO: public repos deploy to GitHub Pages at\n\
              https://9aia.com/{name}/; private repos keep the site\n\
              source, no deploy. `docs/package.json` carries the\n\
              `docs:check` drift gate — generated reference must not\n\
              drift from source. -->\n\n\
         <!-- myco:agent-awareness -->\n\n\
         ## Agent awareness\n\n\
         - Session start / any work discussion: run `myco status --prompt`\n\
           before proposing next steps — know the board first.\n\
         - User describes new work → offer to capture it (`myco todo add`,\n\
           `issue new`, `proposal new`). Don't wait to be asked.\n\
         - Picking up a task: `myco resume <ID>` is the briefing.\n\
         - Wrapping up: `myco handoff <ID>` anything resumable; keep\n\
           statuses honest (claimed → in-progress → blocked/done).\n\n\
         <!-- /myco:agent-awareness -->\n\n\
         <!-- sturn:agent-awareness -->\n\n\
         ## Sturn awareness\n\n\
         - **No PRs.** The pushed branch/commit is the deliverable;\n\
           review streams continuously (sturn WAL), merge is a\n\
           human/sturn call.\n\
         - sturn-wired repo (`sturn.toml` present): claim before edits —\n\
           `sturn claim \"<glob>\" --ws <ws>` surfaces conflicts in ms.\n\
         - Ops via `sturn write|rename|delete`; `sturn snap` folds the ws\n\
           into a real commit; `sturn push` publishes heads +\n\
           refs/sturn/*.\n\
         - `sturn leave` on exit — drops the actor's claims.\n\n\
         <!-- /sturn:agent-awareness -->\n\n\
         <!-- kuda:agent-awareness -->\n\n\
         ## Kuda awareness\n\n\
         - `.kuda/` cards are committed artifacts — read them before\n\
           re-exploring (`kuda map`, `kuda card <mod>`); `kuda status`\n\
           reports per-card drift vs HEAD.\n\
         - Code changes that invalidate a card → regenerate it\n\
           (`kuda sync`) in the same change; cards ride the diff they\n\
           describe.\n\
         - `.kuda/index.sqlite` is gitignored + rebuildable; the `*.yaml`\n\
           cards are the checked-in source of truth.\n\n\
         <!-- /kuda:agent-awareness -->\n"
    )
}

fn cypera_stub(name: &str) -> String {
    format!(
        "# {name} — docs/reference generation contract for `cypera`\n\
         # TODO: declare generators as they come online —\n\
         # [crate_map] layers + [[crate_map.splices]], [cli], [config].\n\
         # Schema lives in ../cypera crates/cypera-core/src/repo_config.rs.\n\
         # Until then `docs/package.json`'s docs:check has nothing to\n\
         # verify — keep it failing loudly rather than vacuously green.\n"
    )
}

const SECURITY_STUB: &str = "\
# Security\n\
\n\
Report vulnerabilities privately to the org maintainers — do not\n\
open public issues for security reports.\n";

fn contributing_stub() -> String {
    "\
# Contributing\n\
\n\
<!-- TODO: contribution policy — issues/PRs enabled or disabled,\n\
     setup steps, where the ledger lives. -->\n\
\n\
See [AGENTS.md](AGENTS.md) for layout, commands, and rules.\n"
        .to_string()
}

fn docs_package_stub(name: &str) -> String {
    // docs:check fails loudly until cypera generators are wired —
    // a vacuous green gate would lie about drift.
    serde_json::to_string_pretty(&serde_json::json!({
        "name": format!("{name}-docs"),
        "private": true,
        "type": "module",
        "scripts": {
            "docs:dev": "vitepress dev",
            "docs:build": "vitepress build",
            "docs:check": "bun -e \"console.error('TODO: wire docs generators — see cypera.toml'); process.exit(1)\""
        },
        "devDependencies": {
            "vitepress": "^1.6.3",
            "vitepress-plugin-llms": "^1.14.0",
            "gh-pages": "^6.3.0"
        }
    }))
    .unwrap_or_else(|_| "{}".to_string())
    + "\n"
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn readme_stub_has_contract_sections() {
        let body = readme_stub("foo");
        assert!(body.contains("# foo"));
        assert!(body.contains("## Quickstart"));
        assert!(body.contains("## Docs"));
        assert!(body.contains("AGENTS.md"));
        assert!(body.contains("TODO"));
    }

    #[test]
    fn agents_stub_has_dev_map_and_awareness() {
        let body = agents_stub("foo");
        for section in [
            "## Layout",
            "## Commands",
            "## Boundary",
            "## Rules",
            "## Docs",
            "myco:agent-awareness",
            "sturn:agent-awareness",
            "kuda:agent-awareness",
        ] {
            assert!(body.contains(section), "missing {section}");
        }
    }

    #[test]
    fn cypera_stub_is_toml_comment_only() {
        let body = cypera_stub("foo");
        assert!(body.contains("# foo"));
        // a stub must not declare generators — every line is a comment
        for line in body.lines() {
            assert!(
                line.trim_start().starts_with('#'),
                "non-comment line: {line}"
            );
        }
    }

    #[test]
    fn docs_package_stub_is_valid_json_with_check_script() {
        let body = docs_package_stub("foo");
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["name"], "foo-docs");
        assert!(v["scripts"]["docs:check"]
            .as_str()
            .unwrap()
            .contains("TODO"));
    }

    #[test]
    fn conform_repo_creates_missing_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(dir.join("README.md"), "# real readme\n").unwrap();
        fs::write(dir.join("CHANGELOG.md"), "# old\n").unwrap();

        let items = conform_repo(dir, "foo", false).unwrap();

        // README preserved
        assert_eq!(
            fs::read_to_string(dir.join("README.md")).unwrap(),
            "# real readme\n"
        );
        // missing stubs created
        for f in ["AGENTS.md", "cypera.toml", "SECURITY.md", "CONTRIBUTING.md"] {
            assert!(dir.join(f).exists(), "{f} not created");
        }
        assert!(dir.join("docs/package.json").exists());
        // CHANGELOG untouched but flagged
        assert_eq!(
            fs::read_to_string(dir.join("CHANGELOG.md")).unwrap(),
            "# old\n"
        );
        assert!(items
            .iter()
            .any(|i| i.path == "CHANGELOG.md" && i.status == ItemStatus::Warn));
        assert!(items
            .iter()
            .any(|i| i.path == "README.md" && i.status == ItemStatus::Present));
        assert!(items
            .iter()
            .any(|i| i.path == "AGENTS.md" && i.status == ItemStatus::Created));
    }

    #[test]
    fn conform_repo_dry_run_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let items = conform_repo(dir, "foo", true).unwrap();
        assert!(!dir.join("README.md").exists());
        assert!(items.iter().all(|i| i.status != ItemStatus::Created));
        assert!(items.iter().any(|i| i.status == ItemStatus::WouldCreate));
        // no docs/ dir → info finding, no package.json attempted
        assert!(items
            .iter()
            .any(|i| i.path == "docs/" && i.status == ItemStatus::Info));
    }
}
