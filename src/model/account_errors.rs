use std::time::SystemTimeError;
use thiserror::Error;
use totp_rs::{SecretParseError, TotpError as TotpRsError};

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum TotpError {
    #[error("")]
    Empty,
    #[error("{0:?}")]
    SecretParseError(SecretParseError),
    #[error("{0}")]
    SystemTimeError(SystemTimeError),
    #[error("Invalid Key: {0}")]
    InvalidKey(String),
    #[error("{0:?}")]
    TotpRsError(TotpRsError),
}

impl TotpError {
    pub fn error(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<TotpRsError> for TotpError {
    fn from(e: TotpRsError) -> Self {
        TotpError::TotpRsError(e)
    }
}

impl From<SecretParseError> for TotpError {
    fn from(e: SecretParseError) -> Self {
        TotpError::SecretParseError(e)
    }
}

impl From<SystemTimeError> for TotpError {
    fn from(e: SystemTimeError) -> Self {
        TotpError::SystemTimeError(e)
    }
}
