//! Output rendering helpers for CLI commands.

use clap::ValueEnum;

/// Supported output formats for task discovery commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    /// Render a compact human-readable table.
    Table,
    /// Render the serialized task descriptor list as JSON.
    Json,
    /// Render the serialized task descriptor list as YAML.
    Yaml,
    /// Render a documentation-friendly Markdown table.
    Markdown,
}
