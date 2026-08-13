//! Shared error type for Elin backend operations.
//!
//! Command handlers convert this into a string so the frontend can show a
//! human-readable message without leaking internal structure.

use serde::Serialize;

/// Application-wide fallible result.
pub type AppResult<T> = Result<T, AppError>;

/// Recoverable failure that can be shown in the UI.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("install error: {0}")]
    Install(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    /// Convenience constructor for ad-hoc messages.
    pub fn msg(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        Self::Network(value.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::Install(format!("archive error: {value}"))
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
