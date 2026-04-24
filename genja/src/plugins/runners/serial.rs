//! Serial task runner plugin implementation.
//!
//! This module provides a serial execution strategy for running tasks across hosts.
//! Tasks are executed sequentially, with each task completing on all hosts before
//! the next task begins. This ensures predictable execution order and simplifies
//! debugging at the cost of parallelism.
//!
//! # Execution Model
//!
//! The serial runner follows this execution pattern:
//!
//! 1. For each task in the task list:
//!    - Execute the task on all hosts sequentially
//!    - Wait for completion on all hosts
//!    - Collect results before proceeding to the next task
//!
//! # Use Cases
//!
//! The serial runner is ideal for:
//!
//! - Debugging task execution and troubleshooting issues
//! - Tasks with strict ordering requirements across hosts
//! - Environments with limited resources where parallel execution might cause issues
//! - Scenarios where task output needs to be easily correlated with specific hosts
//!
//! # Example
//!
//! ```rust,no_run
//! use genja_core::settings::RunnerConfig;
//! use genja_core::inventory::Hosts;
//! use genja_core::task::TaskDefinition;
//! use genja_plugin_manager::plugin_types::PluginRunner;
//! # use genja::plugins::SerialRunnerPlugin;
//!
//! let runner = SerialRunnerPlugin;
//! let task = TaskDefinition::default();
//! let hosts = Hosts::default();
//! let config = RunnerConfig::default();
//!
//! let results = runner.run(&task, &hosts, &config, 10)?;
//! # Ok::<(), genja_core::GenjaError>(())
//! ```
//!
//! # Performance Considerations
//!
//! The serial runner provides no parallelism, making it slower than threaded
//! alternatives for independent tasks. However, it offers:
//!
//! - Minimal resource overhead
//! - Predictable execution order
//! - Simplified error tracking and debugging
//!
//! For production workloads with many independent tasks, consider using the
//! `threaded` runner plugin instead.

use super::executor::TaskExecutor;
use genja_core::settings::RunnerConfig;
use genja_core::GenjaError;
use genja_core::inventory::Hosts;
use genja_core::task::{TaskDefinition, TaskResults, Tasks};
use genja_plugin_manager::plugin_types::{Plugin, PluginRunner};

/// Built-in serial task runner plugin.
///
/// This plugin provides a serial execution strategy for tasks, where tasks are
/// executed sequentially across all selected hosts. Each task completes on all
/// hosts before the next task begins.
pub struct SerialRunnerPlugin;

impl Plugin for SerialRunnerPlugin {
    fn name(&self) -> String {
        "serial".to_string()
    }
}

impl PluginRunner for SerialRunnerPlugin {
    /// Executes a single task definition serially across all hosts.
    ///
    /// This method runs the provided task on all hosts in the inventory sequentially,
    /// waiting for each task to complete on all hosts before proceeding.
    ///
    /// # Parameters
    ///
    /// * `task` - The task definition to execute, containing the task configuration and actions.
    /// * `hosts` - The inventory of hosts on which to execute the task.
    /// * `_runner_config` - The runner configuration (currently unused in serial execution).
    /// * `max_depth` - The maximum depth for nested task execution, used to prevent infinite recursion.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TaskResults)` containing the results of the task execution across all hosts,
    /// or `Err(GenjaError)` if the task execution fails.
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
