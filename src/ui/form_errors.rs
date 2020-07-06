use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("invalid field {0}")]
    FieldError(String),
}
