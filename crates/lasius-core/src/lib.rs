//! Pure domain logic for lasius — no I/O, fully unit-testable.

pub mod checks;
pub mod config;
pub mod error;
pub mod failure;
pub mod gh;
pub mod names;
pub mod status;
pub mod workspace_file;

pub use checks::{run as run_checks, Facts, Finding, RepoFacts, Severity};
pub use config::{ProviderKind, WorkspaceConfig};
pub use error::CoreError;
pub use failure::{classify as classify_failure, tail as tail_text, FailureKind};
pub use gh::Repo;
pub use status::{format_status_table, parse_git_status, GitStatus, StatusRow};
pub use workspace_file::build_workspace_file;
