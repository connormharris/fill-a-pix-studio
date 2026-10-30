use thiserror::Error;

/// Errors produced by `fap-core`.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML syntax error: {0}")]
    Toml(String),
    #[error("{0}")]
    Format(String),
    #[error("unsupported file: {0}")]
    Unsupported(String),
    #[error("PBM error: {0}")]
    Pbm(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
