//! Task command handling.

use crate::discovery::{DiscoveryError, TaskDescriptorSource};
use crate::output::{OutputError, OutputFormat, render_task_descriptor, render_task_list};
use std::error::Error;
use std::fmt;

/// Errors returned while describing one task descriptor.
#[derive(Debug)]
pub enum TaskDescribeError {
    /// Task descriptor discovery failed.
    Discovery(DiscoveryError),
    /// Task descriptor output rendering failed.
    Output(OutputError),
}

impl fmt::Display for TaskDescribeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(error) => write!(f, "{error}"),
            Self::Output(error) => write!(f, "{error}"),
        }
    }
}

impl Error for TaskDescribeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Discovery(error) => Some(error),
            Self::Output(error) => Some(error),
        }
    }
}

impl From<DiscoveryError> for TaskDescribeError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<OutputError> for TaskDescribeError {
    fn from(error: OutputError) -> Self {
        Self::Output(error)
    }
}

/// Errors returned while listing task descriptors.
#[derive(Debug)]
pub enum TaskListError {
    /// Task descriptor discovery failed.
    Discovery(DiscoveryError),
    /// Task descriptor output rendering failed.
    Output(OutputError),
}

impl fmt::Display for TaskListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(error) => write!(f, "{error}"),
            Self::Output(error) => write!(f, "{error}"),
        }
    }
}

impl Error for TaskListError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Discovery(error) => Some(error),
            Self::Output(error) => Some(error),
        }
    }
}

impl From<DiscoveryError> for TaskListError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<OutputError> for TaskListError {
    fn from(error: OutputError) -> Self {
        Self::Output(error)
    }
}

/// List task descriptors from the provided source and render them.
pub fn list_tasks<S>(source: &S, output: OutputFormat) -> Result<String, TaskListError>
where
    S: TaskDescriptorSource + ?Sized,
{
    let descriptors = source.list_tasks()?;
    render_task_list(&descriptors, output).map_err(TaskListError::from)
}

/// Describe one task descriptor from the provided source.
pub fn describe_task<S>(
    source: &S,
    identity: &str,
    output: OutputFormat,
) -> Result<String, TaskDescribeError>
where
    S: TaskDescriptorSource + ?Sized,
{
    let descriptor = source
        .describe_task(identity)
        .map_err(TaskDescribeError::from)?;
    render_task_descriptor(&descriptor, output).map_err(TaskDescribeError::from)
}

#[cfg(test)]
mod tests {
    use crate::discovery::{DiscoveryError, TaskDescriptor};

    use super::*;
    use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};

    #[derive(Debug)]
    struct StaticTaskDescriptorSource {
        result: Result<Vec<TaskDescriptor>, DiscoveryError>,
    }

    impl TaskDescriptorSource for StaticTaskDescriptorSource {
        fn list_tasks(&self) -> Result<Vec<TaskDescriptor>, DiscoveryError> {
            self.result.clone()
        }
    }

    fn descriptor(id: &str, version: &str) -> TaskDescriptor {
        TaskDescriptor::explicit(
            id,
            version,
            TaskDescriptorMetadata {
                name: id.replace('.', "_"),
                description: None,
                execution_mode: TaskExecutionMode::Blocking,
                connection_plugin_name: None,
                processor_names: Vec::new(),
                retry: None,
            },
            None,
            true,
        )
    }

    #[test]
    fn list_tasks_returns_descriptors_from_source() {
        let descriptors = vec![descriptor("acme.tests.cli.task_list", "1.0.0")];
        let source = StaticTaskDescriptorSource {
            result: Ok(descriptors.clone()),
        };
        let output = list_tasks(&source, OutputFormat::Json).expect("descriptors should list");
        let rendered: Vec<TaskDescriptor> =
            serde_json::from_str(&output).expect("JSON output should parse");

        assert_eq!(rendered, descriptors);
    }

    #[test]
    fn list_tasks_returns_discovery_errors_from_source() {
        let source = StaticTaskDescriptorSource {
            result: Err(DiscoveryError::SourceFailed {
                message: "registry unavailable".to_string(),
            }),
        };

        assert!(matches!(
            list_tasks(&source, OutputFormat::Json),
            Err(TaskListError::Discovery(DiscoveryError::SourceFailed { message }))
                if message == "registry unavailable"
        ));
    }

    #[test]
    fn describe_task_returns_descriptor_from_source() {
        let descriptors = vec![descriptor("acme.tests.cli.describe", "1.0.0")];
        let source = StaticTaskDescriptorSource {
            result: Ok(descriptors.clone()),
        };

        let output = describe_task(&source, "acme.tests.cli.describe@1.0.0", OutputFormat::Json)
            .expect("descriptor should be described");
        let rendered: TaskDescriptor =
            serde_json::from_str(&output).expect("JSON output should parse");

        assert_eq!(rendered, descriptors[0]);
    }

    #[test]
    fn describe_task_renders_table_output() {
        let source = StaticTaskDescriptorSource {
            result: Ok(vec![descriptor("acme.tests.cli.describe", "1.0.0")]),
        };

        let output = describe_task(
            &source,
            "acme.tests.cli.describe@1.0.0",
            OutputFormat::Table,
        )
        .expect("descriptor should render as table");

        assert!(output.contains("Task: acme.tests.cli.describe@1.0.0"));
        assert!(output.contains("Field"));
        assert!(output.contains("Value"));
        assert!(output.contains("ID source"));
        assert!(output.contains("explicit"));
    }

    #[test]
    fn describe_task_renders_yaml_output() {
        let source = StaticTaskDescriptorSource {
            result: Ok(vec![descriptor("acme.tests.cli.describe", "1.0.0")]),
        };

        let output = describe_task(&source, "acme.tests.cli.describe@1.0.0", OutputFormat::Yaml)
            .expect("descriptor should render as YAML");
        let rendered: yaml_serde::Value =
            yaml_serde::from_str(&output).expect("YAML output should parse");

        assert_eq!(rendered["id"], "acme.tests.cli.describe");
        assert_eq!(rendered["version"], "1.0.0");
        assert_eq!(rendered["id_source"], "explicit");
    }

    #[test]
    fn describe_task_renders_markdown_output() {
        let source = StaticTaskDescriptorSource {
            result: Ok(vec![descriptor("acme.tests.cli.describe", "1.0.0")]),
        };

        let output = describe_task(
            &source,
            "acme.tests.cli.describe@1.0.0",
            OutputFormat::Markdown,
        )
        .expect("descriptor should render as Markdown");

        assert!(output.starts_with("# acme.tests.cli.describe@1.0.0"));
        assert!(output.contains("| Field"));
        assert!(output.contains("| ID source"));
        assert!(output.contains("| `explicit`"));
    }

    #[test]
    fn describe_task_returns_invalid_identity_errors_from_source() {
        let source = StaticTaskDescriptorSource {
            result: Ok(Vec::new()),
        };

        assert!(matches!(
            describe_task(&source, "acme.tests.cli.describe", OutputFormat::Json),
            Err(TaskDescribeError::Discovery(DiscoveryError::InvalidIdentity {
                identity,
                reason
            })) if identity == "acme.tests.cli.describe"
                && reason == "identity must contain exactly one `@` separator"
        ));
    }

    #[test]
    fn describe_task_returns_not_found_errors_from_source() {
        let source = StaticTaskDescriptorSource {
            result: Ok(Vec::new()),
        };

        assert!(matches!(
            describe_task(
                &source,
                "acme.tests.cli.describe_missing@1.0.0",
                OutputFormat::Yaml
            ),
            Err(TaskDescribeError::Discovery(DiscoveryError::NotFound {
                id,
                version: Some(version),
            })) if id == "acme.tests.cli.describe_missing" && version == "1.0.0"
        ));
    }
}
