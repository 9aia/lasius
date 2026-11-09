use thiserror::Error;

/// Errors produced by pure domain logic.
#[derive(Debug, Error)]
pub enum CoreError {
    /// `lasius.toml` failed to parse or validate.
    #[error("invalid lasius.toml: {0}")]
    Config(String),

    /// `gh repo list --json` output was not valid JSON or not the expected shape.
    #[error("invalid gh repo list JSON: {0}")]
    GhJson(#[from] serde_json::Error),

    /// A repo name from `gh` would escape `repos_dir` if used as a path segment.
    #[error("unsafe repo name in gh output: {0:?}")]
    UnsafeName(String),
}
