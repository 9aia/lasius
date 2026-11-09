//! Pure domain logic for lasius — no I/O, fully unit-testable.

pub mod config;
pub mod error;
pub mod gh;
pub mod names;
pub mod status;
pub mod workspace_file;

pub use config::WorkspaceConfig;
pub use error::CoreError;
pub use gh::Repo;
pub use status::{format_status_table, parse_git_status, GitStatus, StatusRow};
pub use workspace_file::build_workspace_file;
