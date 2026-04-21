use super::executor::TaskExecutor;
use genja_core::GenjaError;
use genja_core::inventory::Hosts;
use genja_core::task::{TaskDefinition, TaskResults, Tasks};
use genja_plugin_manager::plugin_types::{Plugin, PluginRunner};

/// Built-in threaded runner placeholder.
///
/// This currently reuses the shared executor path until a concurrent executor
/// is implemented, but it preserves the configured default runner name.
pub struct ThreadedRunnerPlugin;

impl Plugin for ThreadedRunnerPlugin {
    fn name(&self) -> String {
        "threaded".to_string()
    }
}

impl PluginRunner for ThreadedRunnerPlugin {
    fn run(
        &self,
        task: &TaskDefinition,
        hosts: &Hosts,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        TaskExecutor::new(hosts, max_depth).run_definition(task)
    }

    fn run_tasks(
        &self,
        tasks: &Tasks,
        hosts: &Hosts,
        max_depth: usize,
    ) -> Result<Vec<TaskResults>, GenjaError> {
        tasks
            .iter()
            .map(|task| self.run(task, hosts, max_depth))
            .collect()
    }
}
