//! Task command handling.

use crate::discovery::{DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
use crate::output::OutputFormat;

/// List task descriptors from the provided source.
pub fn list_tasks<S>(source: &S, _output: OutputFormat) -> DiscoveryResult<Vec<TaskDescriptor>>
where
    S: TaskDescriptorSource + ?Sized,
{
    source.list_tasks()
}

#[cfg(test)]
mod tests {
    use crate::discovery::DiscoveryError;

    use super::*;
    use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};

    #[derive(Debug)]
    struct StaticTaskDescriptorSource {
        result: DiscoveryResult<Vec<TaskDescriptor>>,
    }

    impl TaskDescriptorSource for StaticTaskDescriptorSource {
        fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
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

        assert_eq!(
            list_tasks(&source, OutputFormat::Table).expect("descriptors should list"),
            descriptors
        );
    }

    #[test]
    fn list_tasks_returns_discovery_errors_from_source() {
        let source = StaticTaskDescriptorSource {
            result: Err(DiscoveryError::SourceFailed {
                message: "registry unavailable".to_string(),
            }),
        };

        assert_eq!(
            list_tasks(&source, OutputFormat::Json),
            Err(DiscoveryError::SourceFailed {
                message: "registry unavailable".to_string(),
            })
        );
    }
}
