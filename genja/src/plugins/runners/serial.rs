use super::executor::TaskExecutor;
use genja_core::settings::RunnerConfig;
use genja_core::GenjaError;
use genja_core::inventory::Hosts;
use genja_core::task::{TaskDefinition, TaskResults, Tasks};
use genja_plugin_manager::plugin_types::{Plugin, PluginRunner};

/// Built-in serial task runner.
///
/// This runner executes the provided task sequentially across the selected
/// hosts.
pub struct SerialRunnerPlugin;

impl Plugin for SerialRunnerPlugin {
    fn name(&self) -> String {
        "serial".to_string()
    }
}

impl PluginRunner for SerialRunnerPlugin {
    fn run(
        &self,
        task: &TaskDefinition,
        hosts: &Hosts,
        _runner_config: &RunnerConfig,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        TaskExecutor::new(hosts, max_depth).run_definition(task)
    }

    fn run_tasks(
        &self,
        tasks: &Tasks,
        hosts: &Hosts,
        runner_config: &RunnerConfig,
        max_depth: usize,
    ) -> Result<Vec<TaskResults>, GenjaError> {
        tasks
            .iter()
            .map(|task| self.run(task, hosts, runner_config, max_depth))
            .collect()
    }
}
