//! Multi-threaded task execution plugin for concurrent host processing.
//!
//! This module provides a threaded runner plugin that executes tasks across multiple hosts
//! concurrently using a configurable thread pool. It's designed for I/O-bound operations
//! where parallel execution can significantly improve overall task completion time.
//!
//! # Overview
//!
//! The threaded runner distributes work across a pool of worker threads, with each thread
//! pulling hosts from a shared job queue and executing tasks against them. Results are
//! collected via message passing and merged into a single result set.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     ThreadedRunnerPlugin                        │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//!                    ┌──────────────────┐
//!                    │   Job Queue      │
//!                    │  (Arc<Mutex<>>)  │
//!                    └────────┬─────────┘
//!                             │
//!          ┌──────────────────┼──────────────────┐
//!          │                  │                  │
//!          ▼                  ▼                  ▼
//!     ┌─────────┐        ┌─────────┐       ┌─────────┐
//!     │Worker 1 │        │Worker 2 │  ...  │Worker N │
//!     └────┬────┘        └────┬────┘       └────┬────┘
//!          │                  │                  │
//!          └──────────────────┼──────────────────┘
//!                             │
//!                             ▼
//!                    ┌──────────────────┐
//!                    │  Result Channel  │
//!                    │     (mpsc)       │
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
//! The number of worker threads is determined by the following priority:
//!
//! 1. **Explicit Configuration**: If `runner_config.worker_count` is set, use that value
//!    (clamped between 1 and the number of hosts)
//! 2. **System Parallelism**: Otherwise, use the system's available parallelism
//!    (typically the number of CPU cores)
//! 3. **Host Count Cap**: Never create more workers than hosts to process
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
//! The runner uses several thread-safe primitives:
//!
//! - **`Arc<Mutex<VecDeque>>`**: Shared job queue protected by a mutex
//! - **`mpsc::channel`**: Message passing for result collection
//! - **Thread spawning**: Each worker runs in its own OS thread
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
//! - **Memory overhead**: Each worker thread has its own stack (typically 2MB on Linux)
//! - **Context switching**: Too many threads can cause performance degradation
//! - **Lock contention**: High worker counts may contend on the job queue mutex
//!
//! ## Recommended Worker Counts
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
//! - **Worker panics**: Detected via `thread::join()` and converted to `GenjaError`
//! - **Lock poisoning**: Mutex poisoning is caught and reported
//! - **Channel errors**: Send/receive failures are converted to errors
//! - **Task failures**: Individual host failures are collected in results
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use genja_core::inventory::{Hosts, Host, BaseBuilderHost};
//! use genja_core::settings::RunnerConfig;
//! use genja_core::task::TaskDefinition;
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
//! // let results = runner.run(&task, &hosts, &config, 10)?;
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
//! use genja_core::task::Tasks;
//! use genja_plugin_manager::plugin_types::PluginRunner;
//! # use genja::plugins::ThreadedRunnerPlugin;
//! # use genja_core::inventory::Hosts;
//! # use genja_core::settings::RunnerConfig;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let runner = ThreadedRunnerPlugin;
//! // let tasks = Tasks::new(vec![task1, task2, task3]);
//! # let tasks = Tasks::new();
//! # let hosts = Hosts::new();
//! # let config = RunnerConfig::default();
//!
//! // Execute all tasks sequentially, each with parallel host execution
//! let all_results = runner.run_tasks(&tasks, &hosts, &config, 10)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Implementation Details
//!
//! ## Job Queue
//!
//! The job queue is a `VecDeque` wrapped in `Arc<Mutex<>>` for thread-safe access.
//! Workers pop jobs from the front of the queue in a FIFO manner:
//!
//! ```text
//! Queue: [host1, host2, host3, host4, host5]
//!         ▲                              ▲
//!         │                              │
//!      pop_front()                    push_back()
//! ```
//!
//! ## Result Collection
//!
//! Results are sent via an `mpsc::channel` from workers to the main thread:
//!
//! ```text
//! Worker 1 ──┐
//! Worker 2 ──┼──> Channel ──> Main Thread ──> Merged Results
//! Worker 3 ──┘
//! ```
//!
//! ## Timing
//!
//! The runner tracks timing at multiple levels:
//!
//! - **Overall execution**: Start to finish of all workers
//! - **Per-host timing**: Captured by `TaskExecutor`
//! - **Sub-task timing**: Nested task execution times
//!
//! # See Also
//!
//! - [`SerialRunnerPlugin`](../serial/struct.SerialRunnerPlugin.html) - Sequential execution
//! - [`TaskExecutor`](../executor/struct.TaskExecutor.html) - Per-host task execution
//! - [`RunnerConfig`](../../../genja_core/settings/struct.RunnerConfig.html) - Configuration options

use super::executor::TaskExecutor;
use genja_core::GenjaError;
use genja_core::NatString;
use genja_core::inventory::{Host, Hosts};
use genja_core::settings::RunnerConfig;
use genja_core::task::{TaskDefinition, TaskInfo, TaskResults, Tasks};
use genja_plugin_manager::plugin_types::{Plugin, PluginRunner};
use log::error;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::SystemTime;

/// A multi-threaded task runner plugin that executes tasks concurrently across multiple hosts.
///
/// This runner distributes host execution across a configurable number of worker threads,
/// allowing parallel task execution. The number of workers is determined by either the
/// configured worker count or the system's available parallelism, capped by the number of hosts.
///
/// # Thread Safety
///
/// This runner uses thread-safe primitives (Arc, Mutex, mpsc channels) to coordinate work
/// distribution and result collection across worker threads.
///
/// # Performance
///
/// The threaded runner is suitable for I/O-bound tasks where parallelism can improve
/// overall execution time. Worker threads pull jobs from a shared queue until all hosts
/// have been processed.
pub struct ThreadedRunnerPlugin;

impl Plugin for ThreadedRunnerPlugin {
    fn name(&self) -> String {
        "threaded".to_string()
    }
}

impl PluginRunner for ThreadedRunnerPlugin {
    /// Executes a task across multiple hosts using a thread pool.
    ///
    /// This method distributes task execution across the provided hosts using a configurable
    /// number of worker threads. Each worker thread pulls hosts from a shared queue and executes
    /// the task against them. Results from all hosts are collected and merged into a single
    /// `TaskResults` object.
    ///
    /// If the host list is empty, the method returns immediately with an empty result set.
    ///
    /// # Parameters
    ///
    /// * `task` - The task definition to execute on each host.
    /// * `hosts` - A collection of hosts on which to execute the task.
    /// * `runner_config` - Configuration for the runner, including the desired worker thread count.
    /// * `max_depth` - The maximum recursion depth for nested task execution.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TaskResults)` containing the aggregated results from all hosts, including
    /// timing information and execution status. Returns `Err(GenjaError)` if any worker thread
    /// fails, panics, or if internal synchronization primitives become poisoned.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - A worker thread panics during execution
    /// - A worker thread encounters an error while executing tasks
    /// - The shared job queue lock becomes poisoned
    /// - Communication between worker threads and the main thread fails
    fn run(
        &self,
        task: &TaskDefinition,
        hosts: &Hosts,
        runner_config: &RunnerConfig,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        if hosts.is_empty() {
            let started_at = SystemTime::now();
            return Ok(TaskResults::new(task.name())
                .with_started_at(started_at)
                .with_finished_at(started_at)
                .with_duration_ns(0));
        }

        let started_at = SystemTime::now();
        let worker_count = worker_count_for(hosts.len(), runner_config.worker_count());
        let jobs = Arc::new(Mutex::new(collect_jobs(hosts)));
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let jobs = Arc::clone(&jobs);
            let tx = tx.clone();
            let task = task.clone();

            handles.push(thread::spawn(move || -> Result<(), GenjaError> {
                loop {
                    let next_job = {
                        let mut guard = jobs.lock().map_err(|_| {
                            error!(
                                "threaded runner queue lock poisoned for task '{}'",
                                task.name()
                            );
                            GenjaError::Message("threaded runner queue lock poisoned".to_string())
                        })?;
                        guard.pop_front()
                    };

                    let Some((host_id, host)) = next_job else {
                        break;
                    };

                    let host_results = TaskExecutor::run_host(&task, &host_id, &host, max_depth)?;

                    tx.send(host_results).map_err(|err| {
                        error!(
                            "threaded runner failed to send host result for task '{}': {}",
                            task.name(),
                            err
                        );
                        GenjaError::Message(format!(
                            "threaded runner failed to send host result: {}",
                            err
                        ))
                    })?;
                }

                Ok(())
            }));
        }

        drop(tx);

        let mut results = TaskResults::new(task.name()).with_started_at(started_at);
        for host_results in rx {
            results.merge(host_results);
        }

        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    error!("threaded runner worker failed for task '{}': {}", task.name(), err);
                    return Err(err);
                }
                Err(_) => {
                    error!("threaded runner worker panicked for task '{}'", task.name());
                    return Err(GenjaError::Message(
                        "threaded runner worker panicked".to_string(),
                    ));
                }
            }
        }

        let finished_at = SystemTime::now();
        let duration_ns = finished_at
            .duration_since(started_at)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        Ok(results
            .with_finished_at(finished_at)
            .with_duration_ns(duration_ns))
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

/// Determines the optimal number of worker threads for task execution.
///
/// This function calculates the number of worker threads to use based on the configured
/// worker count and the number of hosts. If a worker count is explicitly configured, it
/// will be used (clamped between 1 and the host count). Otherwise, the function uses the
/// system's available parallelism as the basis, also clamped to ensure at least one worker
/// and no more workers than hosts.
///
/// # Parameters
///
/// * `host_count` - The total number of hosts that need to be processed. This serves as
///   an upper bound for the worker count, as having more workers than hosts would be wasteful.
/// * `configured_worker_count` - An optional explicit worker count from the runner configuration.
///   If `Some`, this value takes precedence over system parallelism detection. If `None`, the
///   function falls back to detecting available system parallelism.
///
/// # Returns
///
/// Returns the number of worker threads to spawn, guaranteed to be at least 1 and at most
/// equal to the host count. The returned value represents the optimal thread pool size for
/// distributing work across the available hosts.
fn worker_count_for(host_count: usize, configured_worker_count: Option<usize>) -> usize {
    if let Some(worker_count) = configured_worker_count {
        return worker_count.max(1).min(host_count.max(1));
    }

    let available = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1);
    available.max(1).min(host_count.max(1))
}

/// Converts a collection of hosts into a queue of jobs for worker thread processing.
///
/// This function transforms the provided `Hosts` collection into a `VecDeque` containing
/// tuples of host identifiers and their corresponding host objects. The resulting queue
/// is used by worker threads to pull jobs in a thread-safe manner during parallel task
/// execution.
///
/// Each job in the queue represents a single host that needs to have tasks executed against it.
/// The use of `VecDeque` allows efficient FIFO (first-in-first-out) job distribution, where
/// worker threads can quickly pop jobs from the front of the queue.
///
/// # Parameters
///
/// * `hosts` - A reference to the `Hosts` collection containing all hosts that need to be
///   processed. Each host in this collection will be cloned and added to the job queue.
///
/// # Returns
///
/// Returns a `VecDeque` containing tuples of `(NatString, Host)`, where each tuple represents
/// a single job consisting of a host identifier and its corresponding host object. The queue
/// maintains the iteration order of the input hosts collection.
fn collect_jobs(hosts: &Hosts) -> VecDeque<(NatString, Host)> {
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
