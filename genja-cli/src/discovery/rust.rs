//! Compiled Rust task descriptor discovery source.

use genja_core::task::list_compiled_tasks;

use super::{DiscoveryError, DiscoveryResult, TaskDescriptor, TaskDescriptorSource};

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
}
