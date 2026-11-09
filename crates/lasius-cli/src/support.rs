//! Workspace resolution — find `lasius.toml`, load config, scan clones.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lasius_core::{Repo, WorkspaceConfig};

use crate::proc;

pub const CONFIG_FILE: &str = "lasius.toml";

/// A resolved workspace: root dir + parsed config.
pub struct Workspace {
    pub root: PathBuf,
    pub config: WorkspaceConfig,
}

impl Workspace {
    pub fn repos_path(&self) -> PathBuf {
        self.config.repos_path(&self.root)
    }

    pub fn repo_path(&self, name: &str) -> PathBuf {
        self.config.repo_path(&self.root, name)
    }

    pub fn workspace_file_path(&self) -> PathBuf {
        self.config.workspace_file_path(&self.root)
    }
}

/// Load the workspace at `root` (must contain `lasius.toml`).
pub fn load(root: &Path) -> Result<Workspace> {
    let path = root.join(CONFIG_FILE);
    let source =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let config = WorkspaceConfig::from_toml(&source).context("invalid lasius.toml")?;
    Ok(Workspace {
        root: root.to_path_buf(),
        config,
    })
}

/// Resolve the workspace root: explicit `--root`, else walk up from `cwd`
/// looking for `lasius.toml`.
pub fn resolve(root_flag: Option<&Path>, cwd: &Path) -> Result<Workspace> {
    if let Some(dir) = root_flag {
        return load(dir);
    }
    let mut dir = Some(cwd);
    while let Some(d) = dir {
        if d.join(CONFIG_FILE).is_file() {
            return load(d);
        }
        dir = d.parent();
    }
    anyhow::bail!(
        "no {} found in {} or any parent — run `lasius init` to bootstrap",
        CONFIG_FILE,
        cwd.display()
    )
}

/// `gh repo list <org>` decoded via [`lasius_core::gh::decode_repo_list`].
pub fn org_repos(ws: &Workspace) -> Result<Vec<Repo>> {
    let out = proc::capture_labeled(
        "gh repo list",
        "gh",
        &[
            "repo".into(),
            "list".into(),
            ws.config.org.clone(),
            "--limit".into(),
            "200".into(),
            "--json".into(),
            "name,sshUrl,isArchived,isPrivate".into(),
        ],
        None,
    )?;
    if !out.success() {
        let stderr = out.stderr.trim();
        let detail = if stderr.is_empty() {
            out.stdout.trim()
        } else {
            stderr
        };
        anyhow::bail!("gh repo list failed with exit code {}: {detail}", out.code);
    }
    lasius_core::gh::decode_repo_list(&out.stdout, &ws.config.exclude, ws.config.include_private)
        .map_err(|e| anyhow::anyhow!("could not decode gh repo list output: {e}"))
}

/// Names of git clones under `repos_dir`, sorted. Skips files/symlinks and
/// dirs without `.git`.
pub fn local_repos(ws: &Workspace) -> Result<Vec<String>> {
    let dir = ws.repos_path();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("cannot read {}", dir.display()))? {
        let entry = entry?;
        // file_type doesn't follow symlinks — symlinked dirs are skipped,
        // matching the reference implementation.
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if entry.path().join(".git").exists() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// Regenerate the workspace file from current clones; returns repo count.
pub fn refresh_workspace(ws: &Workspace) -> Result<usize> {
    let names = local_repos(ws)?;
    let content = lasius_core::build_workspace_file(&ws.config.org, &ws.config.repos_dir, &names);
    let path = ws.workspace_file_path();
    // atomic-ish: write temp then rename
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content).with_context(|| format!("cannot write {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("cannot rename to {}", path.display()))?;
    Ok(names.len())
}
