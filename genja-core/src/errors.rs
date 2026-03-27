//! Core error types for Genja.
//!
//! This module currently defines `InventoryLoadError` and `GenjaError`, used by
//! core APIs to report failures in a consistent way.

use std::fmt;

/// Error returned when inventory loading fails.
#[derive(Debug, Clone)]
pub enum InventoryLoadError {
    /// A human-readable error message.
    Message(String),
}

impl fmt::Display for InventoryLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InventoryLoadError::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for InventoryLoadError {}

impl From<String> for InventoryLoadError {
    fn from(value: String) -> Self {
        InventoryLoadError::Message(value)
    }
}

impl From<&str> for InventoryLoadError {
    fn from(value: &str) -> Self {
        InventoryLoadError::Message(value.to_string())
    }
}

/// Generic error type for core Genja operations.
#[derive(Debug, Clone)]
pub enum GenjaError {
    /// A human-readable error message.
    Message(String),
    /// Functionality is not implemented yet.
    NotImplemented(&'static str),
}

impl fmt::Display for GenjaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenjaError::Message(msg) => write!(f, "{msg}"),
            GenjaError::NotImplemented(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for GenjaError {}

impl From<String> for GenjaError {
    fn from(value: String) -> Self {
        GenjaError::Message(value)
    }
}

impl From<&str> for GenjaError {
    fn from(value: &str) -> Self {
        GenjaError::Message(value.to_string())
    }
}
