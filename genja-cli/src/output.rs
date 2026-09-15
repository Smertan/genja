//! Output rendering helpers for CLI commands.

use clap::ValueEnum;
use comfy_table::{Table, presets};
use genja_core::task::{TaskDescriptor, TaskExecutionMode};
use std::error::Error;
use std::fmt;

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

/// Errors returned while rendering CLI output.
#[derive(Debug)]
pub enum OutputError {
    /// JSON serialization failed.
    Json(serde_json::Error),
    /// YAML serialization failed.
    Yaml(yaml_serde::Error),
}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "failed to render JSON output: {error}"),
            Self::Yaml(error) => write!(f, "failed to render YAML output: {error}"),
        }
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Yaml(error) => Some(error),
        }
    }
}

/// Render task descriptors in the requested output format.
pub fn render_task_list(
    descriptors: &[TaskDescriptor],
    format: OutputFormat,
) -> Result<String, OutputError> {
    match format {
        OutputFormat::Table => Ok(render_task_table(descriptors)),
        OutputFormat::Json => serde_json::to_string_pretty(descriptors).map_err(OutputError::Json),
        OutputFormat::Yaml => yaml_serde::to_string(descriptors).map_err(OutputError::Yaml),
        OutputFormat::Markdown => Ok(render_task_markdown(descriptors)),
    }
}

fn render_task_table(descriptors: &[TaskDescriptor]) -> String {
    if descriptors.is_empty() {
        return "No registered tasks found.".to_string();
    }

    let mut table = Table::new();
    table.load_preset(presets::ASCII_NO_BORDERS).set_header([
        "ID",
        "VERSION",
        "NAME",
        "MODE",
        "CONSTRUCTIBLE",
    ]);

    for descriptor in descriptors {
        table.add_row([
            descriptor.id.as_str(),
            descriptor.version.as_str(),
            descriptor.name.as_str(),
            execution_mode_label(descriptor.execution_mode),
            constructible_label(descriptor.constructible),
        ]);
    }

    table.trim_fmt()
}

fn render_task_markdown(descriptors: &[TaskDescriptor]) -> String {
    let mut table = Table::new();
    table.load_preset(presets::ASCII_MARKDOWN).set_header([
        "ID",
        "Version",
        "Name",
        "Mode",
        "Constructible",
        "Description",
    ]);

    for descriptor in descriptors {
        table.add_row([
            markdown_code(&descriptor.id),
            markdown_code(&descriptor.version),
            markdown_code(&descriptor.name),
            markdown_code(execution_mode_label(descriptor.execution_mode)),
            constructible_label(descriptor.constructible).to_string(),
            markdown_text(descriptor.description.as_deref().unwrap_or("")),
        ]);
    }

    table.trim_fmt()
}

fn execution_mode_label(mode: TaskExecutionMode) -> &'static str {
    match mode {
        TaskExecutionMode::Blocking => "blocking",
        TaskExecutionMode::Async => "async",
    }
}

fn constructible_label(constructible: bool) -> &'static str {
    if constructible { "yes" } else { "no" }
}

fn markdown_code(value: &str) -> String {
    format!("`{}`", value.replace('`', "\\`"))
}

fn markdown_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace("\r\n", "<br>")
        .replace(['\n', '\r'], "<br>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use genja_core::task::TaskDescriptorMetadata;
    use serde_json::Value;

    fn descriptor(
        id: &str,
        version: &str,
        name: &str,
        description: Option<&str>,
        execution_mode: TaskExecutionMode,
        constructible: bool,
    ) -> TaskDescriptor {
        TaskDescriptor::explicit(
            id,
            version,
            TaskDescriptorMetadata {
                name: name.to_string(),
                description: description.map(str::to_string),
                execution_mode,
                connection_plugin_name: Some("ssh".to_string()),
                processor_names: Vec::new(),
                retry: None,
            },
            None,
            constructible,
        )
    }

    fn descriptors() -> Vec<TaskDescriptor> {
        vec![
            descriptor(
                "acme.examples.backup_config",
                "1.0.0",
                "backup_config",
                Some("Backs up selected paths from a network device"),
                TaskExecutionMode::Blocking,
                true,
            ),
            descriptor(
                "acme.examples.collect_facts",
                "1.0.0",
                "collect_facts",
                Some("Collects basic host facts"),
                TaskExecutionMode::Async,
                false,
            ),
        ]
    }

    #[test]
    fn table_output_renders_compact_task_rows() {
        let output =
            render_task_list(&descriptors(), OutputFormat::Table).expect("table should render");

        assert!(output.contains("ID"));
        assert!(output.contains("VERSION"));
        assert!(output.contains("NAME"));
        assert!(output.contains("MODE"));
        assert!(output.contains("CONSTRUCTIBLE"));
        assert!(output.contains("acme.examples.backup_config"));
        assert!(output.contains("1.0.0"));
        assert!(output.contains("backup_config"));
        assert!(output.contains("blocking"));
        assert!(output.contains("yes"));
        assert!(output.contains("async"));
        assert!(output.contains("no"));
    }

    #[test]
    fn table_output_reports_empty_task_lists() {
        assert_eq!(
            render_task_list(&[], OutputFormat::Table).expect("empty table should render"),
            "No registered tasks found."
        );
    }

    #[test]
    fn json_output_uses_task_descriptor_contract() {
        let output =
            render_task_list(&descriptors(), OutputFormat::Json).expect("JSON should render");
        let value: Value = serde_json::from_str(&output).expect("JSON should parse");

        assert_eq!(value[0]["id"], "acme.examples.backup_config");
        assert_eq!(value[0]["id_source"], "explicit");
        assert_eq!(value[0]["execution_mode"], "blocking");
        assert_eq!(value[0]["constructible"], true);
        assert_eq!(value[1]["execution_mode"], "async");
        assert_eq!(value[1]["constructible"], false);
    }

    #[test]
    fn json_output_renders_empty_array() {
        assert_eq!(
            render_task_list(&[], OutputFormat::Json).expect("empty JSON should render"),
            "[]"
        );
    }

    #[test]
    fn yaml_output_uses_task_descriptor_contract() {
        let output =
            render_task_list(&descriptors(), OutputFormat::Yaml).expect("YAML should render");
        let value: yaml_serde::Value = yaml_serde::from_str(&output).expect("YAML should parse");

        assert_eq!(value[0]["id"], "acme.examples.backup_config");
        assert_eq!(value[0]["id_source"], "explicit");
        assert_eq!(value[0]["execution_mode"], "blocking");
        assert_eq!(value[0]["constructible"], true);
        assert_eq!(value[1]["execution_mode"], "async");
        assert_eq!(value[1]["constructible"], false);
    }

    #[test]
    fn yaml_output_renders_empty_sequence() {
        let output = render_task_list(&[], OutputFormat::Yaml).expect("empty YAML should render");
        let value: yaml_serde::Value =
            yaml_serde::from_str(&output).expect("empty YAML should parse");

        assert_eq!(value, yaml_serde::Value::Sequence(Vec::new()));
    }

    #[test]
    fn markdown_output_renders_documentation_table() {
        let output = render_task_list(&descriptors(), OutputFormat::Markdown)
            .expect("Markdown should render");

        assert!(output.contains("| ID"));
        assert!(output.contains("|-------"));
        assert!(output.contains("| `acme.examples.backup_config`"));
        assert!(output.contains("| `1.0.0`"));
        assert!(output.contains("| `backup_config`"));
        assert!(output.contains("| `blocking`"));
        assert!(output.contains("| yes"));
        assert!(output.contains("Backs up selected paths from a network device"));
    }

    #[test]
    fn markdown_output_escapes_description_delimiters() {
        let descriptors = vec![descriptor(
            "acme.examples.pipe",
            "1.0.0",
            "pipe_task",
            Some("uses | pipes\nacross lines"),
            TaskExecutionMode::Blocking,
            true,
        )];

        let output =
            render_task_list(&descriptors, OutputFormat::Markdown).expect("Markdown should render");

        assert!(output.contains("uses \\| pipes<br>across lines"));
    }
}
