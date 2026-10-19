//! Observations: every reported value says where it came from, or why it is missing.

use std::io;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Obs<T> {
    Value { value: T, source: String },
    Unavailable { source: String, detail: String },
    PermissionDenied { source: String, detail: String },
    Unsupported { source: String, detail: String },
}

impl<T> Obs<T> {
    pub fn value(value: T, source: impl Into<String>) -> Self {
        Obs::Value {
            value,
            source: source.into(),
        }
    }

    pub fn unavailable(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Obs::Unavailable {
            source: source.into(),
            detail: detail.into(),
        }
    }

    pub fn unsupported(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Obs::Unsupported {
            source: source.into(),
            detail: detail.into(),
        }
    }

    pub fn permission_denied(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Obs::PermissionDenied {
            source: source.into(),
            detail: detail.into(),
        }
    }

    pub fn from_io(source: impl Into<String>, e: &io::Error) -> Self {
        if e.kind() == io::ErrorKind::PermissionDenied {
            Obs::permission_denied(source, e.to_string())
        } else {
            Obs::unavailable(source, e.to_string())
        }
    }

    pub fn get(&self) -> Option<&T> {
        match self {
            Obs::Value { value, .. } => Some(value),
            _ => None,
        }
    }
}
