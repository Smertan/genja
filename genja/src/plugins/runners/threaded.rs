//! Multi-threaded task execution plugin for concurrent host processing.
//!
//! This module provides a threaded runner plugin that executes tasks across multiple hosts
//! concurrently using a bounded set of async tasks. It's designed for I/O-bound operations
//! where parallel execution can significantly improve overall task completion time.
//!
//! # Overview
//!
//! The threaded runner keeps up to `worker_count` host executions in flight at a time.
//! It clones the host list into a job vector, spawns async tasks with Tokio's `JoinSet`,
//! and merges each completed host result into a single result set before spawning the next
//! pending host.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     ThreadedRunnerPlugin                        │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//!                    ┌──────────────────┐
//!                    │   Jobs Vec       │
//!                    │  into_iter()     │
//!                    └────────┬─────────┘
//!                             │
//!          ┌──────────────────┼──────────────────┐
//!          │                  │                  │
//!          ▼                  ▼                  ▼
//!     ┌─────────┐        ┌─────────┐       ┌─────────┐
//!     │Task 1   │        │Task 2   │  ...  │Task N   │
//!     └────┬────┘        └────┬────┘       └────┬────┘
//!          │                  │                  │
//!          └──────────────────┼──────────────────┘
//!                             │
//!                             ▼
//!                    ┌──────────────────┐
//!                    │   JoinSet        │
//!                    │ join_next()      │
//!                    └────────┬─────────┘
//!                             │
//!                             ▼
//!                    ┌──────────────────┐
//!                    │  Merged Results  │
//!                    └──────────────────┘
//! ```
//!
//! # Worker Count Determination
//!
//! The concurrency limit is determined by the following priority:
//!
//! 1. **Explicit Configuration**: If `runner_config.worker_count` is set, use that value
//!    (clamped between 1 and the number of hosts)
//! 2. **System Parallelism**: Otherwise, use the system's available parallelism
//!    (typically the number of CPU cores)
//! 3. **Host Count Cap**: Never allow more in-flight host executions than hosts to process
//!
//! # Configuration
//!
//! The threaded runner can be configured via the `RunnerConfig`:
//!
//! ```json
//! {
//!   "runner": {
//!     "plugin": "threaded",
//!     "worker_count": 10,
//!     "options": {}
//!   }
//! }
//! ```
//!
//! Or via environment variables:
//! ```bash
//! export GENJA_RUNNER_PLUGIN=threaded
//! export GENJA_RUNNER_WORKER_COUNT=10
//! ```
//!
//! # Thread Safety
//!
//! The runner uses several concurrency-safe primitives:
//!
//! - **`tokio::task::JoinSet`**: Bounded in-flight host execution
//! - **Cloned host jobs**: Each spawned task owns its host input
//! - **Optional `Arc<TaskConnectionResolver>`**: Shared connection resolution across tasks
//!
//! # Performance Characteristics
//!
//! ## Best Use Cases
//!
//! - **I/O-bound tasks**: Network operations, file I/O, database queries
//! - **Independent hosts**: Tasks that don't require coordination between hosts
//! - **Variable execution times**: Some hosts may take longer than others
//!
//! ## Considerations
//!
//! - **Task overhead**: More in-flight tasks still increase scheduling and memory overhead
//! - **Context switching**: Excessive concurrency can still degrade throughput
//! - **Host cloning cost**: Hosts are cloned into a job vector before execution begins
//!
//! ## Recommended Concurrency Levels
//!
//! | Scenario | Recommended Workers | Rationale |
//! |----------|-------------------|-----------|
//! | Few hosts (< 10) | Match host count | Avoid idle threads |
//! | Many hosts (> 100) | 2-4x CPU cores | Balance parallelism and overhead |
//! | Network-heavy tasks | 10-20 workers | I/O-bound, can handle more |
//! | CPU-heavy tasks | Match CPU cores | Avoid context switching |
//!
//! # Error Handling
//!
//! The runner handles several error conditions:
//!
//! - **Join failures**: Tokio task failures are logged and converted to `GenjaError`
//! - **Task failures**: Individual host failures are collected in results
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use genja_core::inventory::{Hosts, Host, BaseBuilderHost};
//! use genja_core::settings::RunnerConfig;
//! use genja_core::task::{TaskDefinition, TaskRunOptions};
//! use genja_plugin_manager::plugin_types::PluginRunner;
//! # use genja::plugins::ThreadedRunnerPlugin;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let runner = ThreadedRunnerPlugin;
//! let mut hosts = Hosts::new();
//! hosts.insert("host1".to_string(), Host::builder().hostname("10.0.0.1").build());
//! hosts.insert("host2".to_string(), Host::builder().hostname("10.0.0.2").build());
//!
//! // let task = TaskDefinition::new(my_task);
//! let config = RunnerConfig::default();
//!
//! // let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
//! // let results = runtime.block_on(runner.run_task(
//! //     &task,
//! //     &hosts,
//! //     None,
//! //     &config,
//! //     TaskRunOptions::new(10),
//! // ))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Worker Count
//!
//! ```rust
//! use genja_core::settings::RunnerConfig;
//!
//! // Explicitly set worker count
//! let config = RunnerConfig::builder()
//!     .plugin("threaded")
//!     .worker_count(4)
//!     .build();
//! ```
//!
//! ## Running Multiple Tasks
//!
//! ```rust
//! use genja_core::task::{TaskRunOptions, Tasks};
//! use genja_plugin_manager::plugin_types::PluginRunner;
//! use tokio::runtime::Builder;
//! # use genja::plugins::ThreadedRunnerPlugin;
//! # use genja_core::inventory::Hosts;
//! # use genja_core::settings::RunnerConfig;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let runner = ThreadedRunnerPlugin;
//! let mut tasks = Tasks::new();
//! // tasks.add_task(task1);
//! // tasks.add_task(task2);
//! // tasks.add_task(task3);
//! # let hosts = Hosts::new();
//! # let config = RunnerConfig::default();
//!
//! // Execute root tasks in order, each with parallel host execution
//! let runtime = Builder::new_current_thread().enable_all().build().unwrap();
//! let all_results = runtime.block_on(runner.run_tasks(
//!     &tasks,
//!     &hosts,
//!     None,
//!     &config,
//!     TaskRunOptions::new(10),
//! ))?;
//! # Ok(())
//! # }
//! ```
//!
//! # Implementation Details
//!
//! ## Job Collection
//!
//! The host list is cloned into a `Vec<(NatString, Host)>`, then consumed with an iterator.
//! The runner first fills the `JoinSet` up to the worker limit, then starts one new host
//! execution each time an in-flight task completes:
//!
//! ```text
//! Initial fill: host1, host2, host3
//! Completed:    host2
//! Refilled:     host4
//! ```
//!
//! ## Result Collection
//!
//! Results are merged on the main async task as `join_next()` yields completed host executions:
//!
//! ```text
//! Task 1 ──┐
//! Task 2 ──┼──> JoinSet::join_next() ──> Main Task ──> Merged Results
//! Task 3 ──┘
//! ```
//!
//! ## Timing
//!
//! The runner tracks timing at multiple levels:
//!
//! - **Overall execution**: Start to finish of the full runner invocation
//! - **Per-host timing**: Captured by `TaskExecutor`
//! - **Sub-task timing**: Nested task execution times
//!
//! # See Also
//!
//! - [`SerialRunnerPlugin`](../serial/struct.SerialRunnerPlugin.html) - Sequential execution
//! - [`TaskExecutor`](../executor/struct.TaskExecutor.html) - Per-host task execution
//! - [`RunnerConfig`](../../../genja_core/settings/struct.RunnerConfig.html) - Configuration options

use super::executor::TaskExecutor;
use async_trait::async_trait;
use genja_core::GenjaError;
use genja_core::NatString;
use genja_core::inventory::{Host, Hosts};
use genja_core::settings::RunnerConfig;
use genja_core::task::{TaskDefinition, TaskInfo, TaskResults, TaskRunOptions, Tasks};
use genja_plugin_manager::plugin_types::{Plugin, PluginRunner};
use log::error;
use std::time::SystemTime;
use tokio::task::JoinSet;

/// A multi-threaded task runner plugin that executes tasks concurrently across multiple hosts.
///
/// This runner distributes host execution across a configurable number of concurrent async
/// tasks. The number of in-flight host executions is determined by either the configured
/// worker count or the system's available parallelism, capped by the number of hosts.
///
/// # Thread Safety
///
/// This runner shares only cloned host inputs and an optional `Arc`-wrapped connection
/// resolver across spawned tasks.
///
/// # Performance
///
/// The threaded runner is suitable for I/O-bound tasks where parallelism can improve
/// overall execution time. Spawned tasks are kept in flight until all hosts have been
/// processed.
pub struct ThreadedRunnerPlugin;

impl Plugin for ThreadedRunnerPlugin {
    fn name(&self) -> String {
        "threaded".to_string()
    }
}

#[async_trait]
impl PluginRunner for ThreadedRunnerPlugin {
    /// Executes a task across multiple hosts using bounded async concurrency.
    ///
    /// This method distributes task execution across the provided hosts using a configurable
    /// number of concurrent host executions. The runner fills a `JoinSet` up to the worker
    /// limit, merges each completed host result, and then schedules the next pending host
    /// until all hosts have been processed into a single `TaskResults` object.
    ///
    /// If the host list is empty, the method returns immediately with an empty result set.
    ///
    /// # Parameters
    ///
    /// * `task` - The task definition to execute on each host.
    /// * `hosts` - A collection of hosts on which to execute the task.
    /// * `connection_resolver` - Optional shared resolver used for per-host connection selection.
    /// * `runner_config` - Configuration for the runner, including the desired concurrency limit.
    /// * `options` - Runtime options for task execution.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TaskResults)` containing the aggregated results from all hosts, including
    /// timing information and execution status. Returns `Err(GenjaError)` if any spawned task
    /// fails or returns an execution error.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - A spawned Tokio task fails during execution
    /// - A host execution returns an error
    async fn run_task(
        &self,
        task: &TaskDefinition,
        hosts: &Hosts,
        connection_resolver: Option<std::sync::Arc<dyn genja_core::task::TaskConnectionResolver>>,
        runner_config: &RunnerConfig,
        options: TaskRunOptions,
    ) -> Result<TaskResults, GenjaError> {
        if hosts.is_empty() {
            let started_at = SystemTime::now();
            let mut results = TaskResults::new(task.name())
                .with_started_at(started_at)
                .with_finished_at(started_at)
                .with_duration_ns(0);
            task.process_task_start(&mut results)?;
            task.process_task_finish(&mut results)?;
            return Ok(results);
        }

        let started_at = SystemTime::now();
        let worker_count = worker_count_for(hosts.len(), runner_config.worker_count());
        let jobs = collect_jobs(hosts);
        let mut join_set = JoinSet::new();
        let mut jobs_iter = jobs.into_iter();
        let mut results = TaskResults::new(task.name()).with_started_at(started_at);
        task.process_task_start(&mut results)?;

        while join_set.len() < worker_count {
            let Some((host_id, host)) = jobs_iter.next() else {
                break;
            };
            let task = task.clone();
            let connection_resolver = connection_resolver.clone();
            let runner_config = runner_config.clone();
            join_set.spawn(async move {
                TaskExecutor::run_host(
                    &task,
                    &host_id,
                    &host,
                    connection_resolver,
                    &runner_config,
                    options,
                )
                .await
            });
        }

        while let Some(joined) = join_set.join_next().await {
            match joined {
                Ok(Ok(host_results)) => results.merge(host_results),
                Ok(Err(err)) => {
                    error!(
                        "threaded runner worker failed for task '{}': {}",
                        task.name(),
                        err
                    );
                    return Err(err);
                }
                Err(err) => {
                    error!(
                        "threaded runner worker task failed for task '{}': {}",
                        task.name(),
                        err
                    );
                    return Err(GenjaError::Message(format!(
                        "threaded runner worker task failed: {err}"
                    )));
                }
            }

            if let Some((host_id, host)) = jobs_iter.next() {
                let task = task.clone();
                let connection_resolver = connection_resolver.clone();
                let runner_config = runner_config.clone();
                join_set.spawn(async move {
                    TaskExecutor::run_host(
                        &task,
                        &host_id,
                        &host,
                        connection_resolver,
                        &runner_config,
                        options,
                    )
                    .await
                });
            }
        }

        let finished_at = SystemTime::now();
        let duration_ns = finished_at
            .duration_since(started_at)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        results = results
            .with_finished_at(finished_at)
            .with_duration_ns(duration_ns);
        task.process_task_finish(&mut results)?;
        Ok(results)
    }

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

/// Determines the optimal concurrency limit for task execution.
///
/// This function calculates the number of concurrent host executions to use based on the
/// configured worker count and the number of hosts. If a worker count is explicitly
/// configured, it will be used (clamped between 1 and the host count). Otherwise, the
/// function uses the system's available parallelism as the basis, also clamped to ensure
/// at least one in-flight execution and no more concurrent executions than hosts.
///
/// # Parameters
///
/// * `host_count` - The total number of hosts that need to be processed. This serves as
///   an upper bound for the concurrency limit, as having more in-flight executions than
///   hosts would be wasteful.
/// * `configured_worker_count` - An optional explicit worker count from the runner configuration.
///   If `Some`, this value takes precedence over system parallelism detection. If `None`, the
///   function falls back to detecting available system parallelism.
///
/// # Returns
///
/// Returns the number of concurrent host executions to allow, guaranteed to be at least 1
/// and at most equal to the host count. The returned value represents the effective
/// in-flight work limit for distributing execution across the available hosts.
fn worker_count_for(host_count: usize, configured_worker_count: Option<usize>) -> usize {
    if let Some(worker_count) = configured_worker_count {
        return worker_count.max(1).min(host_count.max(1));
    }

    let available = std::thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1);
    available.max(1).min(host_count.max(1))
}

/// Converts a collection of hosts into an owned job list for concurrent task processing.
///
/// This function transforms the provided `Hosts` collection into a `Vec` containing
/// tuples of host identifiers and their corresponding host objects. The resulting list
/// is consumed by the runner as it schedules host executions during parallel task
/// execution.
///
/// Each job in the list represents a single host that needs to have tasks executed against it.
/// Each tuple is cloned up front so spawned tasks can own their inputs independently.
///
/// # Parameters
///
/// * `hosts` - A reference to the `Hosts` collection containing all hosts that need to be
///   processed. Each host in this collection will be cloned and added to the job list.
///
/// # Returns
///
/// Returns a `Vec` containing tuples of `(NatString, Host)`, where each tuple represents
/// a single job consisting of a host identifier and its corresponding host object. The list
/// maintains the iteration order of the input hosts collection.
fn collect_jobs(hosts: &Hosts) -> Vec<(NatString, Host)> {
    hosts
        .iter()
        .map(|(host_id, host)| (host_id.clone(), host.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::worker_count_for;

    #[test]
    fn worker_count_uses_configured_value_when_present() {
        assert_eq!(worker_count_for(10, Some(4)), 4);
    }

    #[test]
    fn worker_count_caps_configured_value_to_host_count() {
        assert_eq!(worker_count_for(2, Some(10)), 2);
    }

    #[test]
    fn worker_count_clamps_configured_zero_to_one() {
        assert_eq!(worker_count_for(5, Some(0)), 1);
    }
}
