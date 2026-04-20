use genja_core::GenjaError;
use genja_core::NatString;
use genja_core::inventory::Inventory;
use genja_core::task::{Task, TaskDefinition, TaskInfo, TaskResults};
use std::time::SystemTime;

/// Executes a task against the selected hosts for a `Genja` runtime.
///
/// This centralizes host iteration and task dispatch so future execution
/// policies like retries or failure handling can be added without inflating
/// `Genja::run`.
#[derive(Debug)]
pub(crate) struct TaskExecutor<'a> {
    inventory: &'a Inventory,
    host_ids: &'a [NatString],
    max_depth: usize,
}

impl<'a> TaskExecutor<'a> {
    pub(crate) fn new(
        inventory: &'a Inventory,
        host_ids: &'a [NatString],
        max_depth: usize,
    ) -> Self {
        Self {
            inventory,
            host_ids,
            max_depth,
        }
    }

    pub(crate) fn run<T: Task + 'static>(&self, task: T) -> Result<TaskResults, GenjaError> {
        let task_definition = TaskDefinition::new(task);
        self.run_definition(&task_definition)
    }

    fn run_definition(&self, task_definition: &TaskDefinition) -> Result<TaskResults, GenjaError> {
        let started_at = SystemTime::now();
        let mut results = TaskResults::new(task_definition.name()).with_started_at(started_at);

        for host_id in self.host_ids {
            self.run_host(task_definition, host_id, &mut results)?;
        }

        let finished_at = SystemTime::now();
        let duration_ms = finished_at
            .duration_since(started_at)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);

        Ok(results
            .with_finished_at(finished_at)
            .with_duration_ms(duration_ms))
    }

    fn run_host(
        &self,
        task_definition: &TaskDefinition,
        host_id: &NatString,
        results: &mut TaskResults,
    ) -> Result<(), GenjaError> {
        let host = self
            .inventory
            .hosts()
            .get(host_id)
            .ok_or_else(|| GenjaError::Message(format!("host '{}' not found", host_id)))?;

        task_definition.start(host_id.as_str(), &host, results, self.max_depth)
    }
}
