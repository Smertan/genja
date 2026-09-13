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

    fn describe_task(&self, identity: &str) -> DiscoveryResult<TaskDescriptor> {
        get_compiled_task_descriptor_by_identity(identity).map_err(DiscoveryError::from)
    }

    fn describe_task_by_key(&self, key: &TaskRegistrationKey) -> DiscoveryResult<TaskDescriptor> {
        self.describe_task(&key.to_string())
    }
}
