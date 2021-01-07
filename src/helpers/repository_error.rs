use secret_service::SsError;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum RepositoryError {
    SqlError(#[from] rusqlite::Error),
    IoError(#[from] io::Error),
    SerialisationError(#[from] serde_yaml::Error),
    KeyringError(#[from] SsError),
    KeyringDecodingError(#[from] std::string::FromUtf8Error),
}
