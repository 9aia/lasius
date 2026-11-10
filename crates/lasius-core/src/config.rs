use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::CoreError;

/// Contents of `lasius.toml` — the whole workspace declaration.
///
/// ```toml
/// org = "ktbhealth"
/// provider = "github"                        # optional, default "github"
/// repos_dir = "repos"                          # optional, default "repos"
/// workspace_file = "ktbhealth.code-workspace"  # optional, default "{org}.code-workspace"
/// exclude = ["secret-*"]                       # optional name globs
/// include_private = true                       # optional, default false
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    /// Org (or user) whose non-archived repos are managed.
    pub org: String,

    /// Git forge that owns repo discovery. Unknown values are a config
    /// error — there is no silent fallback.
    #[serde(default)]
    pub provider: ProviderKind,

    /// Directory under the workspace root that holds clones.
    #[serde(default = "default_repos_dir")]
    pub repos_dir: String,

    /// VS Code workspace file name; defaults to `{org}.code-workspace`.
    #[serde(default)]
    pub workspace_file: Option<String>,

    /// Glob patterns (`*`, `?`) of repo names to skip during discovery.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Include private repos in discovery (requires `gh` auth with access).
    #[serde(default)]
    pub include_private: bool,

    /// `[checks]` — conformance engine toggles.
    #[serde(default)]
    pub checks: CheckConfig,

    /// `[policy]` — org conventions enforced by `lasius check`.
    #[serde(default)]
    pub policy: Policy,
}

/// Git forge provider that owns org repo discovery.
///
/// Serialized lowercase (`provider = "github"`). New forges add a variant;
/// `lasius-cli` dispatches on this to pick a provider implementation.
/// Unknown strings fail deserialization — no silent default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    /// GitHub, discovered via the `gh` CLI. The default and only provider.
    #[default]
    Github,
}

impl ProviderKind {
    /// Config-file spelling, e.g. `"github"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
        }
    }
}

/// `[checks]` table — enable/disable and severity overrides.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CheckConfig {
    /// Check ids (kebab-case) to skip entirely.
    #[serde(default)]
    pub disabled: Vec<String>,

    /// Per-check severity overrides, e.g. `{ "worktree-health" = "warn" }`.
    #[serde(default)]
    pub severity: std::collections::BTreeMap<String, String>,
}

impl CheckConfig {
    /// Whether `id` is enabled.
    pub fn is_enabled(&self, id: &str) -> bool {
        !self.disabled.iter().any(|d| d == id)
    }
}

/// `[policy]` table — data-driven org conventions.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Policy {
    /// Worktree names every clone must have (e.g. `["project", "docs"]`).
    #[serde(default)]
    pub require_worktrees: Vec<String>,

    /// Files every clone must contain (e.g. `["AGENTS.md"]`).
    #[serde(default)]
    pub require_files: Vec<String>,

    /// Treat a clone whose `origin` doesn't match `org/<name>` as an error
    /// (default: warn).
    #[serde(default)]
    pub remote_must_match_org: bool,
}

fn default_repos_dir() -> String {
    "repos".to_string()
}

impl WorkspaceConfig {
    /// Parse and validate `lasius.toml` contents.
    pub fn from_toml(source: &str) -> Result<Self, CoreError> {
        let config: Self = toml::from_str(source).map_err(|e| CoreError::Config(e.to_string()))?;
        if config.org.trim().is_empty() {
            return Err(CoreError::Config("org must not be empty".to_string()));
        }
        Ok(config)
    }

    /// Serialize back to `lasius.toml` form (used by `init --org`).
    pub fn to_toml(&self) -> String {
        let mut out = format!("org = {:?}\n", self.org);
        if self.provider != ProviderKind::default() {
            out.push_str(&format!("provider = {:?}\n", self.provider.as_str()));
        }
        if self.repos_dir != default_repos_dir() {
            out.push_str(&format!("repos_dir = {:?}\n", self.repos_dir));
        }
        if let Some(file) = &self.workspace_file {
            out.push_str(&format!("workspace_file = {:?}\n", file));
        }
        if !self.exclude.is_empty() {
            let items: Vec<String> = self.exclude.iter().map(|e| format!("{e:?}")).collect();
            out.push_str(&format!("exclude = [{}]\n", items.join(", ")));
        }
        if self.include_private {
            out.push_str("include_private = true\n");
        }
        if !self.checks.disabled.is_empty() {
            let items: Vec<String> = self
                .checks
                .disabled
                .iter()
                .map(|d| format!("{d:?}"))
                .collect();
            out.push_str(&format!("[checks]\ndisabled = [{}]\n", items.join(", ")));
        }
        if !self.checks.severity.is_empty() {
            if self.checks.disabled.is_empty() {
                out.push_str("[checks]\n");
            }
            for (k, v) in &self.checks.severity {
                out.push_str(&format!("severity.{k} = {v:?}\n"));
            }
        }
        let p = &self.policy;
        if !p.require_worktrees.is_empty() || !p.require_files.is_empty() || p.remote_must_match_org
        {
            out.push_str("[policy]\n");
            if !p.require_worktrees.is_empty() {
                let items: Vec<String> = p
                    .require_worktrees
                    .iter()
                    .map(|d| format!("{d:?}"))
                    .collect();
                out.push_str(&format!("require_worktrees = [{}]\n", items.join(", ")));
            }
            if !p.require_files.is_empty() {
                let items: Vec<String> = p.require_files.iter().map(|d| format!("{d:?}")).collect();
                out.push_str(&format!("require_files = [{}]\n", items.join(", ")));
            }
            if p.remote_must_match_org {
                out.push_str("remote_must_match_org = true\n");
            }
        }
        out
    }

    /// Workspace file name — explicit or `{org}.code-workspace`.
    pub fn workspace_file_name(&self) -> String {
        self.workspace_file
            .clone()
            .unwrap_or_else(|| format!("{}.code-workspace", self.org))
    }

    /// `<root>/<repos_dir>`.
    pub fn repos_path(&self, root: &Path) -> PathBuf {
        root.join(&self.repos_dir)
    }

    /// `<root>/<repos_dir>/<name>`.
    pub fn repo_path(&self, root: &Path, name: &str) -> PathBuf {
        self.repos_path(root).join(name)
    }

    /// `<root>/<workspace_file_name>`.
    pub fn workspace_file_path(&self, root: &Path) -> PathBuf {
        root.join(self.workspace_file_name())
    }

    /// Whether `name` matches any `exclude` glob.
    pub fn is_excluded(&self, name: &str) -> bool {
        self.exclude
            .iter()
            .any(|pat| crate::names::wildcard_match(pat, name))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let cfg = WorkspaceConfig::from_toml("org = \"ktbhealth\"").unwrap();
        assert_eq!(cfg.org, "ktbhealth");
        assert_eq!(cfg.repos_dir, "repos");
        assert_eq!(cfg.workspace_file_name(), "ktbhealth.code-workspace");
        assert!(cfg.exclude.is_empty());
    }

    #[test]
    fn parses_full_config() {
        let cfg = WorkspaceConfig::from_toml(
            "org = \"9aia\"\nrepos_dir = \"checkouts\"\nworkspace_file = \"9aia.ws\"\nexclude = [\"secret-*\"]\n",
        )
        .unwrap();
        assert_eq!(cfg.repos_dir, "checkouts");
        assert_eq!(cfg.workspace_file_name(), "9aia.ws");
        assert!(cfg.is_excluded("secret-plans"));
        assert!(!cfg.is_excluded("product"));
    }

    #[test]
    fn rejects_empty_org() {
        assert!(WorkspaceConfig::from_toml("org = \"  \"").is_err());
    }

    #[test]
    fn rejects_invalid_toml() {
        assert!(WorkspaceConfig::from_toml("org = ").is_err());
    }

    #[test]
    fn paths_join_under_root() {
        let cfg = WorkspaceConfig::from_toml("org = \"o\"\nrepos_dir = \"r\"").unwrap();
        let root = Path::new("/ws");
        assert_eq!(cfg.repos_path(root), PathBuf::from("/ws/r"));
        assert_eq!(cfg.repo_path(root, "ui"), PathBuf::from("/ws/r/ui"));
        assert_eq!(
            cfg.workspace_file_path(root),
            PathBuf::from("/ws/o.code-workspace")
        );
    }

    #[test]
    fn provider_defaults_to_github() {
        let cfg = WorkspaceConfig::from_toml("org = \"o\"").unwrap();
        assert_eq!(cfg.provider, ProviderKind::Github);
    }

    #[test]
    fn parses_explicit_provider() {
        let cfg = WorkspaceConfig::from_toml("org = \"o\"\nprovider = \"github\"\n").unwrap();
        assert_eq!(cfg.provider, ProviderKind::Github);
    }

    #[test]
    fn rejects_unknown_provider() {
        let err = WorkspaceConfig::from_toml("org = \"o\"\nprovider = \"gitlab\"\n").unwrap_err();
        assert!(matches!(err, CoreError::Config(_)));
        assert!(err.to_string().contains("gitlab"));
    }

    #[test]
    fn to_toml_omits_default_provider() {
        let cfg = WorkspaceConfig::from_toml("org = \"o\"\nprovider = \"github\"\n").unwrap();
        assert!(!cfg.to_toml().contains("provider"));
    }

    #[test]
    fn to_toml_roundtrips_org() {
        let cfg = WorkspaceConfig::from_toml("org = \"ktbhealth\"").unwrap();
        let parsed = WorkspaceConfig::from_toml(&cfg.to_toml()).unwrap();
        assert_eq!(parsed.org, "ktbhealth");
    }
}
