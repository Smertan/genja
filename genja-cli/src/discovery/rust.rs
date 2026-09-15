//! Compiled Rust task descriptor discovery source.

use genja_core::task::{get_compiled_task_descriptor_by_identity, list_compiled_tasks};

use super::{
    DiscoveryError, DiscoveryResult, TaskDescriptor, TaskDescriptorSource, TaskRegistrationKey,
};

/// Task descriptor source backed by Rust tasks linked into the current process.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompiledTaskDescriptorSource;

impl CompiledTaskDescriptorSource {
    /// Create a compiled Rust task descriptor source.
    pub fn new() -> Self {
        Self
    }
}

impl TaskDescriptorSource for CompiledTaskDescriptorSource {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        let mut descriptors = list_compiled_tasks().map_err(DiscoveryError::from)?;
        descriptors.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.version.cmp(&right.version))
        });
        Ok(descriptors)
    }

    fn describe_task_by_key(&self, key: &TaskRegistrationKey) -> DiscoveryResult<TaskDescriptor> {
        get_compiled_task_descriptor_by_identity(&key.to_string()).map_err(DiscoveryError::from)
    }
}

#[cfg(test)]
mod tests {
    use genja_core::genja_task;
    use genja_core::inventory::Host;
    use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};

    use super::*;

    const ALPHA_ID: &str = "acme.tests.cli.discovery.compiled_alpha";
    const BETA_ID: &str = "acme.tests.cli.discovery.compiled_beta";
    const GENERATED_NAME: &str = "generated_compiled";
    const ALPHA_IDENTITY: &str = "acme.tests.cli.discovery.compiled_alpha@1.0.0";
    const BETA_IDENTITY: &str = "acme.tests.cli.discovery.compiled_beta@2.0.0";

    #[derive(Default)]
    struct GeneratedCompiledTask;

    #[genja_task(name = "generated_compiled")]
    impl GeneratedCompiledTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, genja_core::task::TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    #[derive(Default)]
    struct CompiledBetaTask;

    #[genja_task(
        name = "compiled_beta",
        registration(
            id = "acme.tests.cli.discovery.compiled_beta",
            version = "2.0.0",
            description = "Beta compiled task descriptor",
            factory = "default"
        )
    )]
    impl CompiledBetaTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, genja_core::task::TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    #[derive(Default)]
    struct CompiledAlphaTask;

    #[genja_task(
        name = "compiled_alpha",
        registration(
            id = "acme.tests.cli.discovery.compiled_alpha",
            version = "1.0.0",
            description = "Alpha compiled task descriptor",
            factory = "default"
        )
    )]
    impl CompiledAlphaTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, genja_core::task::TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    fn test_descriptors(source: &CompiledTaskDescriptorSource) -> Vec<TaskDescriptor> {
        source
            .list_tasks()
            .expect("compiled descriptors should list")
            .into_iter()
            .filter(|descriptor| descriptor.id == ALPHA_ID || descriptor.id == BETA_ID)
            .collect()
    }

    #[test]
    fn new_creates_compiled_descriptor_source() {
        let source = CompiledTaskDescriptorSource::new();

        assert_eq!(test_descriptors(&source).len(), 2);
    }

    #[test]
    fn list_tasks_includes_compiled_task_descriptors_in_deterministic_order() {
        let source = CompiledTaskDescriptorSource::new();
        let identities = test_descriptors(&source)
            .into_iter()
            .map(|descriptor| format!("{}@{}", descriptor.id, descriptor.version))
            .collect::<Vec<_>>();

        assert_eq!(identities, vec![ALPHA_IDENTITY, BETA_IDENTITY]);
    }

    #[test]
    fn describe_task_returns_compiled_descriptor_by_identity() {
        let source = CompiledTaskDescriptorSource::new();
        let descriptor = source
            .describe_task(ALPHA_IDENTITY)
            .expect("compiled descriptor should be described");

        assert_eq!(descriptor.id, ALPHA_ID);
        assert_eq!(descriptor.version, "1.0.0");
        assert_eq!(descriptor.name, "compiled_alpha");
        assert_eq!(
            descriptor.description.as_deref(),
            Some("Alpha compiled task descriptor")
        );
        assert!(descriptor.constructible);
    }

    #[test]
    fn describe_task_by_key_returns_compiled_descriptor() {
        let source = CompiledTaskDescriptorSource::new();
        let key = TaskRegistrationKey::parse(BETA_IDENTITY).expect("key should parse");
        let descriptor = source
            .describe_task_by_key(&key)
            .expect("compiled descriptor should be described");

        assert_eq!(descriptor.id, BETA_ID);
        assert_eq!(descriptor.version, "2.0.0");
        assert_eq!(descriptor.name, "compiled_beta");
    }

    #[test]
    fn describe_task_maps_invalid_identity_errors() {
        let source = CompiledTaskDescriptorSource::new();

        assert!(matches!(
            source.describe_task("acme.tests.cli.discovery.compiled_alpha"),
            Err(DiscoveryError::InvalidIdentity { identity, reason })
                if identity == "acme.tests.cli.discovery.compiled_alpha"
                    && reason == "identity must contain exactly one `@` separator"
        ));
    }

    #[test]
    fn describe_task_maps_missing_identity_errors() {
        let source = CompiledTaskDescriptorSource::new();

        assert_eq!(
            source.describe_task("acme.tests.cli.discovery.missing@1.0.0"),
            Err(DiscoveryError::NotFound {
                id: "acme.tests.cli.discovery.missing".to_string(),
                version: Some("1.0.0".to_string()),
            })
        );
    }

    #[test]
    fn describe_task_accepts_generated_descriptor_identities() {
        let source = CompiledTaskDescriptorSource::new();
        let generated = source
            .list_tasks()
            .expect("compiled descriptors should list")
            .into_iter()
            .find(|descriptor| descriptor.name == GENERATED_NAME)
            .expect("generated descriptor should be registered");
        let identity = format!("{}@{}", generated.id, generated.version);

        let descriptor = source
            .describe_task(&identity)
            .expect("generated descriptor should be described");

        assert_eq!(descriptor.id, generated.id);
        assert_eq!(descriptor.version, generated.version);
        assert_eq!(descriptor.name, GENERATED_NAME);
    }
}
