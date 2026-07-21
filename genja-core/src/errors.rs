//! Core error types for Genja.
//!
//! This module currently defines configuration, inventory, and runtime error types used by
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
            } => write!(
                f,
                "failed to parse {kind} inventory JSON file {path}: {message}"
            ),
            InventoryLoadError::ParseYaml {
                kind,
                path,
                message,
            } => write!(
                f,
                "failed to parse {kind} inventory YAML file {path}: {message}"
            ),
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

/// Error returned when SSH configuration validation or parsing fails.
#[derive(Debug, Clone)]
pub enum SshConfigError {
    /// The SSH config file path does not exist.
    NotFound { path: String },
    /// The SSH config file exists but access was denied.
    PermissionDenied { path: String, message: String },
    /// Checking whether the SSH config file exists failed.
    CheckFailed { path: String, message: String },
    /// Opening the SSH config file failed.
    OpenFailed { path: String, message: String },
    /// Parsing the SSH config file failed.
    ParseFailed { path: String, message: String },
}

impl fmt::Display for SshConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SshConfigError::NotFound { path } => write!(f, "SSH config file not found: {path}"),
            SshConfigError::PermissionDenied { path, message } => {
                write!(
                    f,
                    "SSH config file exists but permission denied: {path}: {message}"
                )
            }
            SshConfigError::CheckFailed { path, message } => {
                write!(f, "Failed to check SSH config file {path}: {message}")
            }
            SshConfigError::OpenFailed { path, message } => {
                write!(f, "Failed to open SSH config file {path}: {message}")
            }
            SshConfigError::ParseFailed { path, message } => {
                write!(f, "Failed to parse SSH config file {path}: {message}")
            }
        }
    }
}

impl std::error::Error for SshConfigError {}

/// Error returned when loading the top-level settings file fails.
#[derive(Clone)]
pub enum ConfigLoadError {
    /// The settings file extension is not supported.
    UnsupportedFormat { path: String },
    /// Building the config source from disk failed.
    Read { path: String, message: String },
    /// Deserializing settings from the config source failed.
    Deserialize { path: String, message: String },
    /// SSH configuration referenced by settings failed validation.
    SshConfig(SshConfigError),
}

impl fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigLoadError::UnsupportedFormat { path } => {
                write!(
                    f,
                    "unsupported settings file format for {path}. Use .json, .yaml, or .yml"
                )
            }
            ConfigLoadError::Read { path, message } => {
                write!(f, "failed to read settings from {path}: {message}")
            }
            ConfigLoadError::Deserialize { path, message } => {
                write!(
                    f,
                    "failed to deserialize settings from {path}: {}",
                    format_settings_deserialize_message(message)
                )
            }
            ConfigLoadError::SshConfig(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ConfigLoadError {}

impl fmt::Debug for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

fn format_settings_deserialize_message(message: &str) -> String {
    let Some(error) = UnknownFieldError::parse(message) else {
        return message.to_string();
    };

    let mut formatted = String::from("unknown settings field");
    if let Some(section) = &error.section {
        formatted.push_str(&format!("\n  section: `{section}`"));
    }
    formatted.push_str(&format!("\n  field: `{}`", error.field));
    if !error.expected_fields.is_empty() {
        formatted.push_str("\n  expected fields: ");
        formatted.push_str(
            &error
                .expected_fields
                .iter()
                .map(|field| format!("`{field}`"))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    match error.closest_expected_field() {
        Some(suggestion) => {
            formatted.push_str(&format!("\n  suggestion: did you mean `{suggestion}`?"));
        }
        None => formatted.push_str(
            "\n  suggestion: remove this field, or move plugin-specific values into an explicit \
             options map such as `runner.options` or `inventory.transform_function_options`",
        ),
    }
    formatted
}

struct UnknownFieldError {
    field: String,
    expected_fields: Vec<String>,
    section: Option<String>,
}

impl UnknownFieldError {
    fn parse(message: &str) -> Option<Self> {
        let field = between(message, "unknown field `", "`")?.to_string();
        let section = message
            .split_once(" for key `")
            .and_then(|(_, rest)| rest.split_once('`').map(|(section, _)| section.to_string()));

        let expected = message
            .split_once(", expected ")
            .map(|(_, rest)| {
                rest.split_once(" for key `")
                    .map(|(expected, _)| expected)
                    .unwrap_or(rest)
            })
            .unwrap_or_default();

        let expected_fields = expected
            .split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect();

        Some(Self {
            field,
            expected_fields,
            section,
        })
    }

    fn closest_expected_field(&self) -> Option<&str> {
        self.expected_fields
            .iter()
            .map(|expected| {
                (
                    expected.as_str(),
                    levenshtein_distance(&self.field, expected),
                    common_prefix_len(&self.field, expected),
                )
            })
            .filter(|(expected, distance, prefix_len)| {
                let max_len = self.field.len().max(expected.len());
                *distance <= 3 || (*prefix_len >= 4 && *prefix_len * 2 >= max_len)
            })
            .min_by_key(|(_, distance, _)| *distance)
            .map(|(expected, _, _)| expected)
    }
}

fn between<'a>(value: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let (_, rest) = value.split_once(prefix)?;
    let (matched, _) = rest.split_once(suffix)?;
    Some(matched)
}

fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let right_len = right.chars().count();
    let mut previous: Vec<usize> = (0..=right_len).collect();
    let mut current = vec![0; right_len + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right.chars().enumerate() {
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            let substitution = previous[right_index] + usize::from(left_char != right_char);
            current[right_index + 1] = insertion.min(deletion).min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_len]
}

/// Generic error type for core Genja operations.
#[derive(Clone)]
pub enum GenjaError {
    /// Plugins have not been loaded for the runtime.
    PluginsNotLoaded,
    /// Inventory has not been loaded for the runtime.
    InventoryNotLoaded,
    /// A requested plugin name could not be found.
    PluginNotFound(String),
    /// The named plugin is not an inventory plugin.
    NotInventoryPlugin(String),
    /// The named plugin is an async-only inventory plugin and requires async construction.
    AsyncInventoryPluginRequiresAsyncConstruction(String),
    /// The named plugin is a sync-only inventory plugin and requires sync construction.
    SyncInventoryPluginRequiresSyncConstruction(String),
    /// The named plugin is not a runner plugin.
    NotRunnerPlugin(String),
    /// A plugin failed to load.
    PluginLoad(String),
    /// The configuration file could not be read, parsed, or validated.
    ConfigLoad(ConfigLoadError),
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
            GenjaError::AsyncInventoryPluginRequiresAsyncConstruction(name) => {
                write!(
                    f,
                    "async inventory plugin '{name}' requires async runtime construction.\n\nUse one of:\n- `Genja::from_settings_async(...)`\n- `Genja::from_settings_file_async(...)`"
                )
            }
            GenjaError::SyncInventoryPluginRequiresSyncConstruction(name) => {
                write!(
                    f,
                    "sync inventory plugin '{name}' requires sync runtime construction.\n\nUse one of:\n- `Genja::from_settings(...)`\n- `Genja::from_settings_file(...)`\n\nOr change the inventory plugin to an async implementation before using an async constructor."
                )
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

impl fmt::Debug for GenjaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

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

impl From<ConfigLoadError> for GenjaError {
    fn from(value: ConfigLoadError) -> Self {
        GenjaError::ConfigLoad(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigLoadError, GenjaError};

    #[test]
    fn debug_output_for_settings_unknown_field_uses_structured_display() {
        let error = GenjaError::from(ConfigLoadError::Deserialize {
            path: "/tmp/config.yml".to_string(),
            message: "unknown field `named`, expected one of `plugin`, `options`, `transform_function`, `transform_function_options` for key `inventory`".to_string(),
        });

        let output = format!("{error:?}");

        assert!(output.contains("failed to load settings"));
        assert!(output.contains("unknown settings field"));
        assert!(output.contains("section: `inventory`"));
        assert!(output.contains("field: `named`"));
        assert!(output.contains("expected fields:"));
        assert!(output.contains("suggestion: remove this field"));
        assert!(!output.contains("ConfigLoad(Deserialize"));
    }
}
