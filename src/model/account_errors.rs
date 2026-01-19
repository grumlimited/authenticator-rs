use std::time::SystemTimeError;
use thiserror::Error;
use totp_rs::SecretParseError;
use totp_rs::TotpUrlError;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum TotpError {
    #[error("")]
    Empty,
    #[error("{0:?}")]
    SecretParseError(SecretParseError),
    #[error("")]
    TotpUrlError(TotpUrlError),
    #[error("{0}")]
    SystemTimeError(SystemTimeError),
    #[error("Invalid Key: {0}")]
    InvalidKey(String),
}

impl TotpError {
    pub fn error(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<TotpUrlError> for TotpError {
    fn from(e: TotpUrlError) -> Self {
        TotpError::TotpUrlError(e)
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
