//! Repo-provider seam — forge I/O lives here; `lasius-core` stays pure.
//!
//! [`RepoProvider`] is the extension point for non-GitHub forges.
//! `WorkspaceConfig::provider` selects the implementation via [`for_kind`];
//! today only [`Github`] exists.

use anyhow::Result;
use lasius_core::{ProviderKind, Repo};

use crate::proc;
use crate::support::{self, Workspace};

/// A git forge that can list an org's repos and describe its auth probe.
pub trait RepoProvider {
    /// Config value, e.g. `"github"` — scopes error messages.
    fn name(&self) -> &'static str;

    /// Doctor check label for the auth probe, e.g. `"gh auth"`.
    fn auth_check(&self) -> &'static str;

    /// Auth probe argv for the doctor check, e.g. `gh auth status`.
    fn auth_probe(&self) -> (&'static str, Vec<String>);

    /// Human label of the org-listing step, e.g. `"gh repo list"` — used in
    /// doctor output and error messages.
    fn list_label(&self) -> &'static str;

    /// Org repos with archived/private/excluded names already filtered.
    /// Implementations should also drop the workspace's own repo when it
    /// can be detected (see [`support::own_repo_name`] — currently
    /// github-remote-specific).
    fn list_repos(&self, ws: &Workspace) -> Result<Vec<Repo>>;
}

/// Provider for a `lasius.toml` `provider` value. Unknown kinds can't reach
/// here — [`lasius_core::WorkspaceConfig::from_toml`] rejects them.
pub fn for_kind(kind: ProviderKind) -> Box<dyn RepoProvider> {
    match kind {
        ProviderKind::Github => Box::new(Github),
    }
}

/// GitHub discovery via `gh repo list`.
pub struct Github;

impl RepoProvider for Github {
    fn name(&self) -> &'static str {
        "github"
    }

    fn auth_check(&self) -> &'static str {
        "gh auth"
    }

    fn auth_probe(&self) -> (&'static str, Vec<String>) {
        ("gh", vec!["auth".into(), "status".into()])
    }

    fn list_label(&self) -> &'static str {
        "gh repo list"
    }

    fn list_repos(&self, ws: &Workspace) -> Result<Vec<Repo>> {
        let out = proc::capture_labeled(
            self.list_label(),
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
            anyhow::bail!(
                "{}: {} failed with exit code {}: {detail}",
                self.name(),
                self.list_label(),
                out.code
            );
        }
        let mut repos = lasius_core::gh::decode_repo_list(
            &out.stdout,
            &ws.config.exclude,
            ws.config.include_private,
        )
        .map_err(|e| anyhow::anyhow!("could not decode {} output: {e}", self.list_label()))?;
        // The workspace repo itself is never a member — detected from the
        // root's own `origin` remote (owner must match `org`).
        if let Some(own) = support::own_repo_name(ws) {
            repos.retain(|r| r.name != own);
        }
        Ok(repos)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn dispatch_returns_github() {
        let p = for_kind(ProviderKind::Github);
        assert_eq!(p.name(), "github");
        assert_eq!(p.list_label(), "gh repo list");
        assert_eq!(p.auth_check(), "gh auth");
        let (prog, args) = p.auth_probe();
        assert_eq!(prog, "gh");
        assert_eq!(args, vec!["auth".to_string(), "status".to_string()]);
    }
}
