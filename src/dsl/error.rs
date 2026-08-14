use thiserror::Error;

use super::Location;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{source_name}:{line}:{column}: {message}")]
pub struct DslError {
    pub message: String,
    pub source_name: String,
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

impl DslError {
    pub(crate) fn new(message: impl Into<String>, location: &Location) -> Self {
        Self {
            message: message.into(),
            source_name: location.source_name.clone(),
            line: location.line,
            column: location.column,
            index: location.index,
        }
    }
}
