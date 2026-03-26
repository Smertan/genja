//! Core error types for Genja.
//!
//! This module currently defines `InventoryLoadError`, used by inventory plugins
//! to report load failures in a consistent way.

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
