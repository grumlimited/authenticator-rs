use std::num::ParseIntError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("invalid field {0}")]
    FieldError(String),
    #[error("field {0} is blank")]
    FieldErrorBlank(String),
    #[error("{0}")]
    FieldErrorNaN(String),
}

impl From<ParseIntError> for ValidationError {
    fn from(value: ParseIntError) -> Self {
        ValidationError::FieldErrorNaN(value.to_string())
    }
}
