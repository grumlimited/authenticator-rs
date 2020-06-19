use std::error::Error;
use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    FieldError,
}

impl Error for ValidationError {
    fn description(&self) -> &str {
        match *self {
            ValidationError::FieldError => "invalid",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ValidationError::FieldError => write!(f, "invalid"),
        }
    }
}
