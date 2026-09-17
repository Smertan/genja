//! Output rendering helpers for CLI commands.

use clap::ValueEnum;
use comfy_table::{Table, presets};
use genja_core::task::{RetryConfig, TaskDescriptor, TaskExecutionMode, TaskIdSource};
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
    /// Render Markdown output.
    Markdown,
}

/// Supported output formats for task documentation generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum TaskDocsOutputFormat {
    /// Render a Markdown task catalogue.
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

/// Render one task descriptor in the requested output format.
pub fn render_task_descriptor(
    descriptor: &TaskDescriptor,
    format: OutputFormat,
) -> Result<String, OutputError> {
    match format {
        OutputFormat::Table => render_task_descriptor_table(descriptor),
        OutputFormat::Json => serde_json::to_string_pretty(descriptor).map_err(OutputError::Json),
        OutputFormat::Yaml => yaml_serde::to_string(descriptor).map_err(OutputError::Yaml),
        OutputFormat::Markdown => render_task_descriptor_markdown(descriptor),
    }
}

/// Render task descriptors as a documentation catalogue.
pub fn render_task_docs(
    descriptors: &[TaskDescriptor],
    format: TaskDocsOutputFormat,
) -> Result<String, OutputError> {
    match format {
        TaskDocsOutputFormat::Markdown => render_task_docs_markdown(descriptors),
    }
}

fn render_task_docs_markdown(descriptors: &[TaskDescriptor]) -> Result<String, OutputError> {
    if descriptors.is_empty() {
        return Ok("# Task Catalogue\n\nNo registered tasks found.".to_string());
    }

    let mut output = format!(
        "# Task Catalogue\n\n## Index\n\n{}\n\n## Summary\n\n{}",
        render_task_docs_index_markdown(descriptors),
        render_task_docs_summary_markdown(descriptors)
    );

    output.push_str("\n\n## Tasks");

    for descriptor in descriptors {
        output.push_str("\n\n");
        output.push_str(&format!(
            "<a id=\"{}\"></a>\n\n",
            task_docs_anchor_id(descriptor)
        ));
        output.push_str(&render_task_descriptor_markdown_section(
            descriptor, "###", "####",
        )?);
    }

    Ok(output)
}

fn render_task_docs_index_markdown(descriptors: &[TaskDescriptor]) -> String {
    let task_links = descriptors
        .iter()
        .map(|descriptor| {
            format!(
                "  - [{}](#{})",
                markdown_code(&descriptor_identity(descriptor)),
                task_docs_anchor_id(descriptor)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("- [Summary](#summary)\n- [Tasks](#tasks)\n{task_links}")
}

fn task_docs_anchor_id(descriptor: &TaskDescriptor) -> String {
    let identity = descriptor_identity(descriptor);
    let mut anchor = String::from("task-");

    for character in identity.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            anchor.push(character);
        } else if !anchor.ends_with('-') {
            anchor.push('-');
        }
    }

    anchor.trim_end_matches('-').to_string()
}

fn render_task_docs_summary_markdown(descriptors: &[TaskDescriptor]) -> String {
    let mut table = Table::new();
    table.load_preset(presets::ASCII_MARKDOWN).set_header([
        "ID",
        "Version",
        "Name",
        "Source",
        "Mode",
        "Constructible",
        "Description",
    ]);

    for descriptor in descriptors {
        table.add_row([
            markdown_code(&descriptor.id),
            markdown_code(&descriptor.version),
            markdown_code(&descriptor.name),
            markdown_code(id_source_label(descriptor.id_source)),
            markdown_code(execution_mode_label(descriptor.execution_mode)),
            constructible_label(descriptor.constructible).to_string(),
            markdown_text(optional_label(descriptor.description.as_deref())),
        ]);
    }

    table.trim_fmt()
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
        "SOURCE",
        "MODE",
        "CONSTRUCTIBLE",
    ]);

    for descriptor in descriptors {
        table.add_row([
            descriptor.id.as_str(),
            descriptor.version.as_str(),
            descriptor.name.as_str(),
            id_source_label(descriptor.id_source),
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
        "Source",
        "Mode",
        "Constructible",
    ]);

    for descriptor in descriptors {
        table.add_row([
            markdown_code(&descriptor.id),
            markdown_code(&descriptor.version),
            markdown_code(&descriptor.name),
            markdown_code(id_source_label(descriptor.id_source)),
            markdown_code(execution_mode_label(descriptor.execution_mode)),
            constructible_label(descriptor.constructible).to_string(),
        ]);
    }

    table.trim_fmt()
}

fn render_task_descriptor_table(descriptor: &TaskDescriptor) -> Result<String, OutputError> {
    let mut table = Table::new();
    table
        .load_preset(presets::ASCII_NO_BORDERS)
        .set_header(["Field", "Value"]);

    table.add_row(["ID".to_string(), descriptor.id.clone()]);
    table.add_row(["Version".to_string(), descriptor.version.clone()]);
    table.add_row(["Name".to_string(), descriptor.name.clone()]);
    table.add_row([
        "Description".to_string(),
        optional_label(descriptor.description.as_deref()).to_string(),
    ]);
    table.add_row([
        "ID source".to_string(),
        id_source_label(descriptor.id_source).to_string(),
    ]);
    table.add_row([
        "Execution mode".to_string(),
        execution_mode_label(descriptor.execution_mode).to_string(),
    ]);
    table.add_row([
        "Connection plugin".to_string(),
        optional_label(descriptor.connection_plugin_name.as_deref()).to_string(),
    ]);
    table.add_row([
        "Processors".to_string(),
        processors_label(&descriptor.processor_names),
    ]);
    table.add_row([
        "Constructible".to_string(),
        constructible_label(descriptor.constructible).to_string(),
    ]);
    table.add_row(["Retry".to_string(), retry_label(descriptor.retry.as_ref())?]);
    table.add_row([
        "Input schema".to_string(),
        input_schema_label(descriptor.input_schema.as_ref()).to_string(),
    ]);

    let mut output = format!(
        "Task: {}\n\n{}",
        descriptor_identity(descriptor),
        table.trim_fmt()
    );

    if let Some(schema) = descriptor.input_schema.as_ref() {
        output.push_str("\n\nInput schema:\n");
        output.push_str(&serde_json::to_string_pretty(schema).map_err(OutputError::Json)?);
    }

    Ok(output)
}

fn render_task_descriptor_markdown(descriptor: &TaskDescriptor) -> Result<String, OutputError> {
    render_task_descriptor_markdown_section(descriptor, "#", "##")
}

fn render_task_descriptor_markdown_section(
    descriptor: &TaskDescriptor,
    descriptor_heading: &str,
    input_schema_heading: &str,
) -> Result<String, OutputError> {
    let mut table = Table::new();
    table
        .load_preset(presets::ASCII_MARKDOWN)
        .set_header(["Field", "Value"]);

    table.add_row(["ID".to_string(), markdown_code(&descriptor.id)]);
    table.add_row(["Version".to_string(), markdown_code(&descriptor.version)]);
    table.add_row(["Name".to_string(), markdown_code(&descriptor.name)]);
    table.add_row([
        "Description".to_string(),
        markdown_text(optional_label(descriptor.description.as_deref())),
    ]);
    table.add_row([
        "ID source".to_string(),
        markdown_code(id_source_label(descriptor.id_source)),
    ]);
    table.add_row([
        "Execution mode".to_string(),
        markdown_code(execution_mode_label(descriptor.execution_mode)),
    ]);
    table.add_row([
        "Connection plugin".to_string(),
        optional_markdown_code(descriptor.connection_plugin_name.as_deref()),
    ]);
    table.add_row([
        "Processors".to_string(),
        processors_markdown_label(&descriptor.processor_names),
    ]);
    table.add_row([
        "Constructible".to_string(),
        constructible_label(descriptor.constructible).to_string(),
    ]);
    table.add_row([
        "Retry".to_string(),
        retry_markdown_label(descriptor.retry.as_ref())?,
    ]);
    table.add_row([
        "Input schema".to_string(),
        input_schema_label(descriptor.input_schema.as_ref()).to_string(),
    ]);

    let mut output = format!(
        "{} {}\n\n{}",
        descriptor_heading,
        descriptor_identity(descriptor),
        table.trim_fmt()
    );

    if let Some(schema) = descriptor.input_schema.as_ref() {
        output.push_str(&format!(
            "\n\n{input_schema_heading} Input Schema\n\n```json\n"
        ));
        output.push_str(&serde_json::to_string_pretty(schema).map_err(OutputError::Json)?);
        output.push_str("\n```");
    }

    Ok(output)
}

fn descriptor_identity(descriptor: &TaskDescriptor) -> String {
    format!("{}@{}", descriptor.id, descriptor.version)
}

fn id_source_label(source: TaskIdSource) -> &'static str {
    match source {
        TaskIdSource::Generated => "generated",
        TaskIdSource::Explicit => "explicit",
    }
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

fn optional_label(value: Option<&str>) -> &str {
    value.filter(|value| !value.is_empty()).unwrap_or("-")
}

fn processors_label(processors: &[String]) -> String {
    if processors.is_empty() {
        "-".to_string()
    } else {
        processors.join(", ")
    }
}

fn processors_markdown_label(processors: &[String]) -> String {
    if processors.is_empty() {
        "-".to_string()
    } else {
        processors
            .iter()
            .map(|processor| markdown_code(processor))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn retry_label(retry: Option<&RetryConfig>) -> Result<String, OutputError> {
    retry.map_or_else(
        || Ok("-".to_string()),
        |retry| serde_json::to_string(retry).map_err(OutputError::Json),
    )
}

fn retry_markdown_label(retry: Option<&RetryConfig>) -> Result<String, OutputError> {
    retry_label(retry).map(|label| {
        if label == "-" {
            label
        } else {
            markdown_code(&label)
        }
    })
}

fn input_schema_label(input_schema: Option<&serde_json::Value>) -> &'static str {
    if input_schema.is_some() {
        "available"
    } else {
        "-"
    }
}

fn optional_markdown_code(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(markdown_code)
        .unwrap_or_else(|| "-".to_string())
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
    use serde_json::{Value, json};

    fn descriptor(
        id: &str,
        version: &str,
        name: &str,
        description: Option<&str>,
        execution_mode: TaskExecutionMode,
        constructible: bool,
    ) -> TaskDescriptor {
        detailed_descriptor(
            id,
            version,
            name,
            description,
            execution_mode,
            constructible,
            Vec::new(),
            None,
            None,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "tests need explicit descriptor fields"
    )]
    fn detailed_descriptor(
        id: &str,
        version: &str,
        name: &str,
        description: Option<&str>,
        execution_mode: TaskExecutionMode,
        constructible: bool,
        processor_names: Vec<String>,
        retry: Option<RetryConfig>,
        input_schema: Option<Value>,
    ) -> TaskDescriptor {
        TaskDescriptor::explicit(
            id,
            version,
            TaskDescriptorMetadata {
                name: name.to_string(),
                description: description.map(str::to_string),
                execution_mode,
                connection_plugin_name: Some("ssh".to_string()),
                processor_names,
                retry,
            },
            input_schema,
            constructible,
        )
    }

    fn schema_descriptor() -> TaskDescriptor {
        detailed_descriptor(
            "acme.examples.backup_config",
            "1.0.0",
            "backup_config",
            Some("Backs up selected paths from a network device"),
            TaskExecutionMode::Blocking,
            true,
            vec!["audit".to_string(), "notify".to_string()],
            Some(RetryConfig::new(Some(true), Some(3), Some(250))),
            Some(json!({
                "type": "object",
                "required": ["backup_path", "compress"],
                "properties": {
                    "backup_path": {
                        "type": "string"
                    },
                    "compress": {
                        "type": "boolean"
                    }
                }
            })),
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
        assert!(output.contains("SOURCE"));
        assert!(output.contains("MODE"));
        assert!(output.contains("CONSTRUCTIBLE"));
        assert!(output.contains("acme.examples.backup_config"));
        assert!(output.contains("1.0.0"));
        assert!(output.contains("backup_config"));
        assert!(output.contains("explicit"));
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
        assert!(output.contains("| `explicit`"));
        assert!(output.contains("| `blocking`"));
        assert!(output.contains("| yes"));
        assert!(!output.contains("Backs up selected paths from a network device"));
    }

    #[test]
    fn markdown_output_omits_description_for_compact_catalog() {
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

        assert!(!output.contains("uses | pipes"));
        assert!(!output.contains("uses \\| pipes"));
    }

    #[test]
    fn docs_output_reports_empty_task_lists() {
        let output = render_task_docs(&[], TaskDocsOutputFormat::Markdown)
            .expect("empty docs should render");

        assert_eq!(output, "# Task Catalogue\n\nNo registered tasks found.");
    }

    #[test]
    fn docs_output_renders_summary_and_detailed_sections() {
        let output = render_task_docs(&descriptors(), TaskDocsOutputFormat::Markdown)
            .expect("docs should render");

        assert!(output.starts_with("# Task Catalogue"));
        assert!(output.contains("## Index"));
        assert!(output.contains("- [Summary](#summary)"));
        assert!(output.contains("- [Tasks](#tasks)"));
        assert!(output.contains(
            "  - [`acme.examples.backup_config@1.0.0`](#task-acme-examples-backup-config-1-0-0)"
        ));
        assert!(output.contains(
            "  - [`acme.examples.collect_facts@1.0.0`](#task-acme-examples-collect-facts-1-0-0)"
        ));
        assert!(output.contains("## Summary"));
        assert!(output.contains("| ID"));
        assert!(output.contains("| Description"));
        assert!(output.contains("| `acme.examples.backup_config`"));
        assert!(output.contains("| `1.0.0`"));
        assert!(output.contains("| `backup_config`"));
        assert!(output.contains("| `explicit`"));
        assert!(output.contains("| `blocking`"));
        assert!(output.contains("| yes"));
        assert!(output.contains("Backs up selected paths from a network device"));
        assert!(output.contains("## Tasks"));
        assert!(output.contains("<a id=\"task-acme-examples-backup-config-1-0-0\"></a>"));
        assert!(output.contains("<a id=\"task-acme-examples-collect-facts-1-0-0\"></a>"));
        assert!(output.contains("### acme.examples.backup_config@1.0.0"));
        assert!(output.contains("### acme.examples.collect_facts@1.0.0"));
        assert!(output.contains("| Field"));
        assert!(output.contains("| ID source"));
        assert!(output.contains("| Execution mode"));
        assert!(output.contains("| Connection plugin"));
        assert!(output.contains("| Processors"));
        assert!(output.contains("| Constructible"));
        assert!(output.contains("| Retry"));
        assert!(output.contains("| Input schema"));
    }

    #[test]
    fn docs_output_renders_schema_blocks() {
        let output = render_task_docs(&[schema_descriptor()], TaskDocsOutputFormat::Markdown)
            .expect("docs should render");

        assert!(output.contains(
            "  - [`acme.examples.backup_config@1.0.0`](#task-acme-examples-backup-config-1-0-0)"
        ));
        assert!(output.contains("<a id=\"task-acme-examples-backup-config-1-0-0\"></a>"));
        assert!(output.contains("### acme.examples.backup_config@1.0.0"));
        assert!(output.contains("| Input schema"));
        assert!(output.contains("| available"));
        assert!(output.contains("#### Input Schema"));
        assert!(output.contains("```json"));
        assert!(output.contains("\"backup_path\""));
        assert!(output.contains("\"compress\""));
    }

    #[test]
    fn descriptor_table_output_renders_summary_and_schema() {
        let output = render_task_descriptor(&schema_descriptor(), OutputFormat::Table)
            .expect("descriptor table should render");

        assert!(output.contains("Task: acme.examples.backup_config@1.0.0"));
        assert!(output.contains("Field"));
        assert!(output.contains("Value"));
        assert!(output.contains("ID"));
        assert!(output.contains("acme.examples.backup_config"));
        assert!(output.contains("Version"));
        assert!(output.contains("1.0.0"));
        assert!(output.contains("Name"));
        assert!(output.contains("backup_config"));
        assert!(output.contains("Description"));
        assert!(output.contains("Backs up selected paths from a network device"));
        assert!(output.contains("ID source"));
        assert!(output.contains("explicit"));
        assert!(output.contains("Execution mode"));
        assert!(output.contains("blocking"));
        assert!(output.contains("Connection plugin"));
        assert!(output.contains("ssh"));
        assert!(output.contains("Processors"));
        assert!(output.contains("audit, notify"));
        assert!(output.contains("Constructible"));
        assert!(output.contains("yes"));
        assert!(output.contains("Retry"));
        assert!(output.contains("\"max_attempts\":3"));
        assert!(output.contains("Input schema"));
        assert!(output.contains("available"));
        assert!(output.contains("\"backup_path\""));
    }

    #[test]
    fn descriptor_table_output_marks_missing_optional_metadata() {
        let output = render_task_descriptor(
            &descriptor(
                "acme.examples.collect_facts",
                "1.0.0",
                "collect_facts",
                None,
                TaskExecutionMode::Async,
                false,
            ),
            OutputFormat::Table,
        )
        .expect("descriptor table should render");

        assert!(output.contains("Description"));
        assert!(output.contains("Connection plugin"));
        assert!(output.contains("Processors"));
        assert!(output.contains("Retry"));
        assert!(output.contains("Input schema"));
        assert!(output.contains("no"));
        assert!(!output.contains("Input schema:\n"));
    }

    #[test]
    fn descriptor_json_output_uses_task_descriptor_contract() {
        let output = render_task_descriptor(&schema_descriptor(), OutputFormat::Json)
            .expect("JSON should render");
        let value: Value = serde_json::from_str(&output).expect("JSON should parse");

        assert_eq!(value["id"], "acme.examples.backup_config");
        assert_eq!(value["id_source"], "explicit");
        assert_eq!(value["execution_mode"], "blocking");
        assert_eq!(value["processor_names"], json!(["audit", "notify"]));
        assert_eq!(value["retry"]["allow"], true);
        assert_eq!(value["input_schema"]["type"], "object");
        assert_eq!(value["constructible"], true);
    }

    #[test]
    fn descriptor_yaml_output_uses_task_descriptor_contract() {
        let output = render_task_descriptor(&schema_descriptor(), OutputFormat::Yaml)
            .expect("YAML should render");
        let value: yaml_serde::Value = yaml_serde::from_str(&output).expect("YAML should parse");

        assert_eq!(value["id"], "acme.examples.backup_config");
        assert_eq!(value["id_source"], "explicit");
        assert_eq!(value["execution_mode"], "blocking");
        assert_eq!(value["processor_names"][0], "audit");
        assert_eq!(value["retry"]["allow"], true);
        assert_eq!(value["input_schema"]["type"], "object");
        assert_eq!(value["constructible"], true);
    }

    #[test]
    fn descriptor_markdown_output_renders_summary_and_schema() {
        let output = render_task_descriptor(&schema_descriptor(), OutputFormat::Markdown)
            .expect("Markdown should render");

        assert!(output.starts_with("# acme.examples.backup_config@1.0.0"));
        assert!(output.contains("| Field"));
        assert!(output.contains("| ID"));
        assert!(output.contains("| `acme.examples.backup_config`"));
        assert!(output.contains("| Version"));
        assert!(output.contains("| `1.0.0`"));
        assert!(output.contains("| Name"));
        assert!(output.contains("| `backup_config`"));
        assert!(output.contains("| Description"));
        assert!(output.contains("Backs up selected paths from a network device"));
        assert!(output.contains("| ID source"));
        assert!(output.contains("| `explicit`"));
        assert!(output.contains("| Execution mode"));
        assert!(output.contains("| `blocking`"));
        assert!(output.contains("| Connection plugin"));
        assert!(output.contains("| `ssh`"));
        assert!(output.contains("| Processors"));
        assert!(output.contains("`audit`, `notify`"));
        assert!(output.contains("| Constructible"));
        assert!(output.contains("| yes"));
        assert!(output.contains("| Retry"));
        assert!(output.contains("`{\"allow\":true"));
        assert!(output.contains("| Input schema"));
        assert!(output.contains("| available"));
        assert!(output.contains("## Input Schema"));
        assert!(output.contains("```json"));
        assert!(output.contains("\"backup_path\""));
    }
}
