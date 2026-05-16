//! Core error types for Genja.
//!
//! This module currently defines `InventoryLoadError` and `GenjaError`, used by
//! core APIs to report failures in a consistent way.

use std::fmt;

/// Logical inventory section associated with an inventory load error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryFileKind {
    /// The hosts inventory file.
    Hosts,
    /// The groups inventory file.
    Groups,
    /// The defaults inventory file.
    Defaults,
}

impl fmt::Display for InventoryFileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InventoryFileKind::Hosts => write!(f, "hosts"),
            InventoryFileKind::Groups => write!(f, "groups"),
            InventoryFileKind::Defaults => write!(f, "defaults"),
        }
    }
}

/// Error returned when inventory loading fails.
#[derive(Debug, Clone)]
pub enum InventoryLoadError {
    /// Reading an inventory file failed.
    Read {
        /// Which logical inventory file failed.
        kind: InventoryFileKind,
        /// Filesystem path that was being read.
        path: String,
        /// Underlying read failure rendered as text.
        message: String,
    },
    /// Parsing a JSON inventory file failed.
    ParseJson {
        /// Which logical inventory file failed.
        kind: InventoryFileKind,
        /// Filesystem path that was being parsed.
        path: String,
        /// Underlying parse failure rendered as text.
        message: String,
    },
    /// Parsing a YAML inventory file failed.
    ParseYaml {
        /// Which logical inventory file failed.
        kind: InventoryFileKind,
        /// Filesystem path that was being parsed.
        path: String,
        /// Underlying parse failure rendered as text.
        message: String,
    },
    /// The inventory file extension is not supported.
    UnsupportedFormat {
        /// Which logical inventory file failed.
        kind: InventoryFileKind,
        /// Filesystem path with the unsupported extension.
        path: String,
    },
    /// A configured transform plugin was not found.
    TransformPluginNotFound {
        /// Missing plugin name.
        name: String,
    },
    /// A configured plugin exists but is not a transform-function plugin.
    NotTransformPlugin {
        /// Plugin name with the wrong type.
        name: String,
    },
    /// A human-readable fallback error message.
    Message(String),
}

impl fmt::Display for InventoryLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InventoryLoadError::Read {
                kind,
                path,
                message,
            } => write!(f, "failed to read {kind} inventory file {path}: {message}"),
            InventoryLoadError::ParseJson {
                kind,
                path,
                message,
            } => write!(f, "failed to parse {kind} inventory JSON file {path}: {message}"),
            InventoryLoadError::ParseYaml {
                kind,
                path,
                message,
            } => write!(f, "failed to parse {kind} inventory YAML file {path}: {message}"),
            InventoryLoadError::UnsupportedFormat { kind, path } => write!(
                f,
                "unsupported {kind} inventory file format for {path}. Use .json, .yaml, or .yml"
            ),
            InventoryLoadError::TransformPluginNotFound { name } => {
                write!(f, "transform plugin '{name}' not found")
            }
            InventoryLoadError::NotTransformPlugin { name } => {
                write!(f, "plugin '{name}' is not a transform function plugin")
            }
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
    /// Plugins have not been loaded for the runtime.
    PluginsNotLoaded,
    /// Inventory has not been loaded for the runtime.
    InventoryNotLoaded,
    /// A requested plugin name could not be found.
    PluginNotFound(String),
    /// The named plugin is not an inventory plugin.
    NotInventoryPlugin(String),
    /// The named plugin is not a runner plugin.
    NotRunnerPlugin(String),
    /// A plugin failed to load.
    PluginLoad(String),
    /// The configuration file could not be read or parsed.
    ConfigLoad(String),
    /// Inventory loading failed.
    InventoryLoad(InventoryLoadError),
    /// A human-readable error message.
    Message(String),
    /// Functionality is not implemented yet.
    NotImplemented(&'static str),
}

impl fmt::Display for GenjaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenjaError::PluginsNotLoaded => write!(f, "plugins have not been loaded"),
            GenjaError::InventoryNotLoaded => write!(f, "inventory has not been loaded"),
            GenjaError::PluginNotFound(name) => write!(f, "plugin '{name}' not found"),
            GenjaError::NotInventoryPlugin(name) => {
                write!(f, "plugin '{name}' is not an inventory plugin")
            }
            GenjaError::NotRunnerPlugin(name) => {
                write!(f, "plugin '{name}' is not a runner plugin")
            }
            GenjaError::PluginLoad(err) => write!(f, "failed to load plugins: {err}"),
            GenjaError::ConfigLoad(err) => write!(f, "failed to load settings: {err}"),
            GenjaError::InventoryLoad(err) => write!(f, "failed to load inventory: {err}"),
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

impl From<InventoryLoadError> for GenjaError {
    fn from(value: InventoryLoadError) -> Self {
        GenjaError::InventoryLoad(value)
    }
}
