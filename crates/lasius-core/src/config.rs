use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::CoreError;

/// Contents of `lasius.toml` — the whole workspace declaration.
///
/// ```toml
/// org = "ktbhealth"
/// repos_dir = "repos"                          # optional, default "repos"
/// workspace_file = "ktbhealth.code-workspace"  # optional, default "{org}.code-workspace"
/// exclude = ["secret-*"]                       # optional name globs
/// include_private = true                       # optional, default false
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    /// GitHub org (or user) whose non-archived repos are managed.
    pub org: String,

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
    fn to_toml_roundtrips_org() {
        let cfg = WorkspaceConfig::from_toml("org = \"ktbhealth\"").unwrap();
        let parsed = WorkspaceConfig::from_toml(&cfg.to_toml()).unwrap();
        assert_eq!(parsed.org, "ktbhealth");
    }
}
