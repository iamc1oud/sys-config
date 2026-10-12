use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "no SysConfig repository found; run `sys-config init <path>`, `cd` into one or pass --repo"
    )]
    RepoNotFound,

    #[error("path does not exist: {0}")]
    MissingTarget(PathBuf),

    #[error("{0} is already tracked")]
    AlreadyTracked(String),

    #[error("{0} is not tracked")]
    NotTracked(String),

    #[error("{0} overlaps with already tracked path {1}")]
    Overlap(String, String),

    #[error("repository path `{0}` collides with the one used by {1}; choose another with --as")]
    RepoPathTaken(String, String),

    #[error("invalid repository path `{0}`")]
    InvalidRepoPath(String),

    #[error("{0} is a file on one side and a directory on the other")]
    TypeMismatch(String),

    #[error("{0} unresolved conflict(s); run interactively or pass --on-conflict")]
    Conflicts(usize),

    #[error("unsupported manifest version {0}")]
    ManifestVersion(u32),

    #[error("git failed: {0}")]
    Git(String),

    #[error("aborted")]
    Aborted,
}
