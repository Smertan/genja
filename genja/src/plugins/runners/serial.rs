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
//! use genja::genja_task;
//! use genja_core::settings::RunnerConfig;
//! use genja_core::inventory::{Host, Hosts};
//! use genja_core::task::{
//!     HostTaskResult, TaskDefinition, TaskError, TaskRuntimeContext, TaskRunOptions, TaskSuccess,
//! };
//! use genja_plugin_manager::plugin_types::PluginRunner;
//! use tokio::runtime::Builder;
//! # use genja::plugins::SerialRunnerPlugin;
//!
//! struct ExampleTask;
//!
//! #[genja_task(name = "example", connection_plugin_name = "ssh")]
//! impl ExampleTask {
//!     async fn start_async(
//!         &self,
//!         _host: &Host,
//!         _context: &TaskRuntimeContext,
//!     ) -> Result<HostTaskResult, TaskError> {
//!         Ok(HostTaskResult::passed(TaskSuccess::new()))
//!     }
//! }
//!
//! let runner = SerialRunnerPlugin;
//! let task = TaskDefinition::new(ExampleTask);
//! let hosts = Hosts::default();
//! let config = RunnerConfig::default();
//!
//! let runtime = Builder::new_current_thread().enable_all().build().unwrap();
//! let results = runtime.block_on(runner.run_task(
//!     &task,
//!     &hosts,
//!     None,
//!     &config,
//!     TaskRunOptions::new(10),
//! ))?;
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
use async_trait::async_trait;
use genja_core::GenjaError;
use genja_core::inventory::Hosts;
use genja_core::settings::RunnerConfig;
use genja_core::task::{TaskDefinition, TaskResults, TaskRunOptions, Tasks};
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

#[async_trait]
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
    /// * `connection_resolver` - Optional shared resolver used for per-host connection selection.
    /// * `_runner_config` - The runner configuration (currently unused in serial execution).
    /// * `options` - Runtime options for task execution.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TaskResults)` containing the results of the task execution across all hosts,
    /// or `Err(GenjaError)` if the task execution fails.
    async fn run_task(
        &self,
        task: &TaskDefinition,
        hosts: &Hosts,
        connection_resolver: Option<std::sync::Arc<dyn genja_core::task::TaskConnectionResolver>>,
        runner_config: &RunnerConfig,
        options: TaskRunOptions,
    ) -> Result<TaskResults, GenjaError> {
        TaskExecutor::new(hosts, connection_resolver, runner_config, options)
            .run_definition(task)
            .await
    }

    /// Executes all task definitions sequentially.
    ///
    /// This method runs each task in `tasks` one after another. For each task, host
    /// execution is also serial because it delegates to [`Self::run_task`].
    ///
    /// # Parameters
    ///
    /// * `tasks` - The ordered list of task definitions to execute.
    /// * `hosts` - The inventory of hosts on which to execute each task.
    /// * `connection_resolver` - Optional shared resolver used for per-host connection selection.
    /// * `runner_config` - The runner configuration forwarded to [`Self::run_task`].
    /// * `options` - Runtime options for task execution.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<TaskResults>)` containing one aggregated result set per task,
    /// or `Err(GenjaError)` if any task execution fails.
    async fn run_tasks(
        &self,
        tasks: &Tasks,
        hosts: &Hosts,
        connection_resolver: Option<std::sync::Arc<dyn genja_core::task::TaskConnectionResolver>>,
        runner_config: &RunnerConfig,
        options: TaskRunOptions,
    ) -> Result<Vec<TaskResults>, GenjaError> {
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks.iter() {
            results.push(
                self.run_task(
                    task,
                    hosts,
                    connection_resolver.clone(),
                    runner_config,
                    options,
                )
                .await?,
            );
        }
        Ok(results)
    }
}
