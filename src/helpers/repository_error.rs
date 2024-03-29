use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
#[allow(clippy::enum_variant_names)]
pub enum RepositoryError {
    GAuthQrCodeError(String),
    SqlError(#[from] rusqlite::Error),
    IoError(#[from] io::Error),
    SerialisationError(#[from] serde_yaml::Error),
    KeyringError(#[from] secret_service::Error),
    KeyringDecodingError(#[from] std::string::FromUtf8Error),
}
