use std::time::SystemTimeError;
use thiserror::Error;
use totp_rs::{SecretParseError, TotpUrlError};

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum TotpError {
    #[error("Empty secret")]
    Empty,

    #[error("Secret parse error: {0}")]
    SecretParseError(#[from] SecretParseError),

    #[error("TOTP URL error: {0}")]
    TotpUrlError(#[from] TotpUrlError),

    #[error("System time error: {0}")]
    SystemTimeError(#[from] SystemTimeError),

    #[error("Invalid Key: {0}")]
    InvalidKey(String),
}

impl TotpError {
    pub fn error(&self) -> String {
        format!("{}", self)
    }
}
