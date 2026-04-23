//! Task execution framework for Genja.
//!
//! This module provides the core task execution infrastructure for Genja, enabling
//! structured task definition, execution, and result tracking across multiple hosts.
//! It defines traits, types, and utilities for building task-based automation workflows
//! with support for nested sub-tasks, rich result metadata, and flexible error handling.
//!
//! # Overview
//!
//! The task system is built around several key concepts:
//!
//! - **Task Definition**: Tasks implement the [`Task`] trait, which combines metadata
//!   ([`TaskInfo`]) with execution logic and optional sub-tasks ([`SubTasks`]).
//! - **Task Execution**: Tasks execute against hosts and return [`HostTaskResult`]
//!   indicating success, failure, or skip status.
//! - **Result Tracking**: The [`TaskResults`] structure maintains a hierarchical tree
//!   of execution results for tasks and their sub-tasks across all hosts.
//! - **Rich Metadata**: Tasks can attach detailed metadata including timing information,
//!   warnings, messages, diffs, and custom data to their results.
//!
//! # Task Lifecycle
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Task Definition                          │
//! │                    (implements Task trait)                      │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      TaskDefinition Wrapper                     │
//! │                  (provides execution control)                   │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     Task Execution (start)                      │
//! │              - Execute task.start() for each host               │
//! │              - Store results in TaskResults tree                │
//! │              - Recursively execute sub-tasks                    │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        HostTaskResult                           │
//! │         - Passed (TaskSuccess with metadata)                    │
//! │         - Failed (TaskFailure with error details)               │
//! │         - Skipped (TaskSkip with reason)                        │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Core Traits
//!
//! ## [`Task`]
//!
//! The primary trait that all tasks must implement. It combines [`TaskInfo`] for
//! metadata, [`SubTasks`] for hierarchical task structures, and a `start()` method
//! for execution logic.
//!
//! In the common derive-based workflow, `#[derive(Task)]` from `genja-core-derive`
//! generates [`TaskInfo`] and [`SubTasks`], while you still implement [`Task`]
//! manually to provide `start()`.
//!
//! ```rust
//! use genja_core::task::{Task, TaskInfo, SubTasks, HostTaskResult, TaskSuccess};
//! use genja_core::inventory::{Host, ConnectionKey};
//! use std::sync::Arc;
//! use serde_json::Value;
//!
//! struct DeployTask {
//!     name: String,
//!     config_file: String,
//! }
//!
//! impl TaskInfo for DeployTask {
//!     fn name(&self) -> &str {
//!         &self.name
//!     }
//!
//!     fn plugin_name(&self) -> &str {
//!         "ssh"
//!     }
//!
//!     fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
//!         ConnectionKey::new(hostname, "ssh")
//!     }
//!
//!     fn options(&self) -> Option<&Value> {
//!         None
//!     }
//! }
//!
//! impl SubTasks for DeployTask {
//!     fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
//!         Vec::new()
//!     }
//! }
//!
//! impl Task for DeployTask {
//!     fn start(&self, host: &Host) -> Result<HostTaskResult, genja_core::task::TaskError> {
//!         // Execute deployment logic
//!         Ok(HostTaskResult::passed(
//!             TaskSuccess::new()
//!                 .with_changed(true)
//!                 .with_summary("Configuration deployed successfully")
//!         ))
//!     }
//! }
//! ```
//!
//! ## [`TaskInfo`]
//!
//! Provides metadata about a task including its name, associated plugin, connection
//! requirements, and optional configuration. This trait is typically auto-implemented
//! when using the `#[derive(Task)]` macro from `genja-core-derive`.
//! That derive reads the task struct's `name`, optional `plugin_name`, and optional
//! `options` fields to generate the corresponding trait methods.
//!
//! ## [`SubTasks`]
//!
//! Enables hierarchical task structures by allowing tasks to define sub-tasks that
//! execute after the parent task completes. Sub-tasks inherit the execution context
//! and can be conditionally skipped based on parent task results.
//! With `#[derive(Task)]`, any field marked with `#[task(subtask)]` is included in
//! [`SubTasks::sub_tasks()`] in declaration order.
//!
//! # Behavioral Rules
//!
//! The execution model is intentionally simple and deterministic:
//!
//! - The parent task's `start()` method runs before any of its sub-tasks.
//! - The parent task's [`HostTaskResult`] is inserted into [`TaskResults`] before
//!   sub-task execution begins.
//! - Sub-tasks run in the order returned by [`SubTasks::sub_tasks()`]. For the
//!   derive macro, that means declaration order of fields marked with `#[task(subtask)]`.
//! - Each host is executed independently. When running through `Genja::run`, the
//!   full task tree is executed once per selected host.
//! - Sub-task results are grouped by sub-task name. The `TaskResults` node for a
//!   given sub-task contains per-host results accumulated across all hosts.
//! - The framework does not automatically skip sub-tasks when a parent fails or is
//!   skipped. If you want that behavior, return an explicit [`HostTaskResult::Skipped`]
//!   from the sub-task or encode the condition in the task itself.
//! - `max_depth` is checked using `depth > max_depth`. This means `max_depth = 0`
//!   still allows the root task at depth `0`, but rejects all sub-tasks at depth `1`.
//!
//! # Task Results
//!
//! ## [`HostTaskResult`]
//!
//! Represents the outcome of executing a task on a single host. It can be:
//!
//! - **Passed**: Task completed successfully with optional metadata in [`TaskSuccess`]
//! - **Failed**: Task encountered an error with details in [`TaskFailure`]
//! - **Skipped**: Task was not executed with reason in [`TaskSkip`]
//!
//! ```rust
//! use genja_core::task::{HostTaskResult, TaskSuccess, TaskFailure, TaskFailureKind};
//! use serde_json::json;
//!
//! // Success with metadata
//! let success = HostTaskResult::passed(
//!     TaskSuccess::new()
//!         .with_result(json!({"status": "deployed"}))
//!         .with_changed(true)
//!         .with_diff("+ new_config_line")
//! );
//!
//! // Failure with classification
//! let failure = HostTaskResult::failed(
//!     TaskFailure::new(std::io::Error::new(
//!         std::io::ErrorKind::ConnectionRefused,
//!         "connection refused"
//!     ))
//!     .with_kind(TaskFailureKind::Connection)
//!     .with_retryable(true)
//! );
//!
//! // Skipped with reason
//! let skipped = HostTaskResult::skipped_with_reason("parent_failed");
//! ```
//!
//! ## [`TaskResults`]
//!
//! A hierarchical structure that stores execution results for a task and all its
//! sub-tasks across multiple hosts. It provides methods for querying results,
//! tracking success/failure counts, and navigating the task tree.
//!
//! ```rust
//! use genja_core::task::{TaskResults, HostTaskResult, TaskSuccess};
//!
//! let mut results = TaskResults::new("deploy")
//!     .with_summary("Deployment completed");
//!
//! results.insert_host_result(
//!     "router1",
//!     HostTaskResult::passed(TaskSuccess::new().with_changed(true))
//! );
//!
//! results.insert_host_result(
//!     "router2",
//!     HostTaskResult::skipped_with_reason("maintenance_mode")
//! );
//!
//! // Query results
//! assert_eq!(results.passed_hosts().len(), 1);
//! assert!(results.host_result("router1").unwrap().is_passed());
//! assert!(results.host_result("router2").unwrap().is_skipped());
//! ```
//!
//! # Task Metadata
//!
//! ## [`TaskSuccess`]
//!
//! Contains rich metadata about successful task execution:
//!
//! - **result**: Structured data returned by the task (JSON)
//! - **changed**: Whether the task modified the target system
//! - **diff**: Text representation of changes made
//! - **summary**: Human-readable summary of execution
//! - **warnings**: Non-fatal issues encountered
//! - **messages**: Structured log messages with levels and codes
//! - **metadata**: Additional custom data
//! - **timing**: Start time, finish time, and duration
//!
//! ## [`TaskFailure`]
//!
//! Contains detailed error information for failed tasks:
//!
//! - **error**: The underlying error that caused the failure
//! - **kind**: Classification of the failure ([`TaskFailureKind`])
//! - **retryable**: Whether the operation can be retried
//! - **details**: Additional context about the failure (JSON)
//! - **warnings**: Non-fatal issues that preceded the failure
//! - **messages**: Structured log messages
//!
//! ## [`TaskSkip`]
//!
//! Contains information about why a task was skipped:
//!
//! - **reason**: Machine-readable skip reason
//! - **message**: Human-readable explanation
//!
//! # Failure Classification
//!
//! The [`TaskFailureKind`] enum categorizes failures to enable appropriate error
//! handling and retry logic:
//!
//! - **Connection**: Network or connectivity issues (often retryable)
//! - **Authentication**: Credential or permission problems
//! - **Validation**: Invalid input or configuration
//! - **Timeout**: Operation exceeded time limit (often retryable)
//! - **Command**: Remote command execution failed
//! - **Unsupported**: Operation not supported by target
//! - **Internal**: Genja/framework implementation error
//! - **External**: Error returned from a task, plugin, or external dependency
//!
//! # Message System
//!
//! Tasks can emit structured messages during execution using [`TaskMessage`]:
//!
//! ```rust
//! use genja_core::task::{TaskMessage, MessageLevel};
//! use std::time::SystemTime;
//!
//! let message = TaskMessage::new(MessageLevel::Warning, "High latency detected")
//!     .with_code("latency_warn")
//!     .with_timestamp(SystemTime::now());
//! ```
//!
//! Message levels include:
//! - **Info**: Informational messages
//! - **Warning**: Non-fatal issues
//! - **Error**: Error details
//! - **Debug**: Debugging information
//!
//! # Task Execution
//!
//! ## [`TaskDefinition`]
//!
//! A wrapper around task implementations that provides execution control and
//! enforces the task execution flow. It handles recursive sub-task execution
//! with depth limiting to prevent infinite recursion.
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use genja_core::inventory::{BaseBuilderHost, ConnectionKey, Host};
//! use genja_core::task::{
//!     HostTaskResult, Task, TaskDefinition, TaskInfo, TaskResults, TaskSuccess, SubTasks,
//! };
//! use serde_json::Value;
//!
//! struct DeployTask {
//!     name: String,
//! }
//!
//! impl TaskInfo for DeployTask {
//!     fn name(&self) -> &str {
//!         &self.name
//!     }
//!
//!     fn plugin_name(&self) -> &str {
//!         "ssh"
//!     }
//!
//!     fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
//!         ConnectionKey::new(hostname, self.plugin_name())
//!     }
//!
//!     fn options(&self) -> Option<&Value> {
//!         None
//!     }
//! }
//!
//! impl SubTasks for DeployTask {
//!     fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
//!         Vec::new()
//!     }
//! }
//!
//! impl Task for DeployTask {
//!     fn start(&self, _host: &Host) -> Result<HostTaskResult, genja_core::task::TaskError> {
//!         Ok(HostTaskResult::passed(
//!             TaskSuccess::new().with_summary("deploy complete"),
//!         ))
//!     }
//! }
//!
//! let task = TaskDefinition::new(DeployTask {
//!     name: "deploy".to_string(),
//! });
//! let host = Host::builder().hostname("router1").build();
//! let mut results = TaskResults::new("deploy");
//!
//! task.start("router1", &host, &mut results, 1)
//!     .expect("task execution should succeed");
//!
//! assert!(results.host_result("router1").unwrap().is_passed());
//! ```
//!
//! ## [`Tasks`]
//!
//! A collection type for managing multiple task definitions. It provides a
//! convenient way to build and execute task lists.
//!
//! ```rust
//! use genja_core::task::Tasks;
//!
//! let mut tasks = Tasks::new();
//! // tasks.add_task(deploy_task);
//! // tasks.add_task(validate_task);
//! // tasks.add_task(cleanup_task);
//! ```
//!
//! # Advanced Usage
//!
//! ## Hierarchical Task Execution
//!
//! Tasks can define sub-tasks that execute after the parent task completes. This
//!
use crate::inventory::Host;
use crate::types::{CustomTreeMap, NatString};
use log::{debug, info, warn};
use serde::Serialize;
use serde_json::Value;
use std::any::{type_name, Any};
use std::error::Error;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Clone)]
pub struct TaskError {
    error: Arc<dyn Error + Send + Sync + 'static>,
    error_type: String,
    source: Option<Arc<dyn Any + Send + Sync + 'static>>,
}

type TaskFailureSource = Arc<dyn Any + Send + Sync + 'static>;

impl TaskError {
    pub fn new<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        let error = Arc::new(error);
        Self {
            error_type: type_name::<E>().to_string(),
            source: Some(error.clone()),
            error,
        }
    }

    pub fn from_arc(error: Arc<dyn Error + Send + Sync + 'static>) -> Self {
        Self {
            error,
            error_type: "dyn core::error::Error".to_string(),
            source: None,
        }
    }

    pub fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.error.as_ref()
    }

    pub fn error_type(&self) -> &str {
        &self.error_type
    }

    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: 'static,
    {
        self.source
            .as_ref()
            .and_then(|source| source.downcast_ref::<E>())
    }
}

impl fmt::Debug for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskError")
            .field("error_type", &self.error_type)
            .field("message", &self.error.to_string())
            .finish()
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl Error for TaskError {}

#[derive(Debug)]
struct CapturedTaskFailure {
    message: String,
}

impl fmt::Display for CapturedTaskFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CapturedTaskFailure {}

fn format_timestamp_display(timestamp: SystemTime) -> String {
    humantime::format_rfc3339_seconds(timestamp).to_string()
}

fn format_duration_display(duration_ns: u128) -> String {
    if duration_ns < 1_000 {
        return format!("{duration_ns}ns");
    }

    if duration_ns < 1_000_000 {
        return format_decimal_unit(duration_ns as f64 / 1_000.0, "us");
    }

    if duration_ns < 1_000_000_000 {
        return format_decimal_unit(duration_ns as f64 / 1_000_000.0, "ms");
    }

    if duration_ns < 60_000_000_000 {
        return format_decimal_unit(duration_ns as f64 / 1_000_000_000.0, "s");
    }

    if duration_ns < 3_600_000_000_000 {
        return format_decimal_unit(duration_ns as f64 / 60_000_000_000.0, "m");
    }

    format_decimal_unit(duration_ns as f64 / 3_600_000_000_000.0, "h")
}

fn format_decimal_unit(value: f64, unit: &str) -> String {
    let precision = if value >= 100.0 {
        0
    } else if value >= 10.0 {
        1
    } else {
        2
    };

    let formatted = format!("{value:.precision$}");
    let trimmed = if let Some((whole, fractional)) = formatted.split_once('.') {
        let fractional = fractional.trim_end_matches('0');
        if fractional.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{fractional}")
        }
    } else {
        formatted
    };
    format!("{trimmed}{unit}")
}

/// Results of a task execution, including timing, host outcomes, and nested sub-task results.
///
/// `TaskResults` captures the complete execution state of a task across multiple hosts,
/// including timing information, a summary, per-host results, and any sub-tasks that were
/// executed as part of this task. This structure forms a tree where each task can contain
/// results for multiple hosts and multiple sub-tasks, allowing for hierarchical task execution
/// tracking.
///
/// # Fields
///
/// * `task_name` - The name of the task that was executed.
/// * `started_at` - The timestamp when the task execution started, if available.
/// * `finished_at` - The timestamp when the task execution finished, if available.
/// * `duration_ms` - The duration of the task execution in milliseconds, if available.
/// * `summary` - An optional summary message describing the overall task execution.
/// * `hosts` - A map of hostname to `HostTaskResult`, containing the execution result for each host.
/// * `sub_tasks` - A map of sub-task name to `TaskResults`, containing results for any nested tasks.
///
/// # Example
///
/// ```rust
/// use genja_core::task::{TaskResults, HostTaskResult, TaskSuccess};
/// use std::time::SystemTime;
///
/// // Create a new task results container
/// let mut results = TaskResults::new("deploy_config")
///     .with_started_at(SystemTime::now())
///     .with_summary("Deploying configuration to network devices");
///
/// // Add results for individual hosts
/// results.insert_host_result(
///     "router1",
///     HostTaskResult::passed(
///         TaskSuccess::new()
///             .with_changed(true)
///             .with_summary("Configuration deployed successfully")
///     )
/// );
///
/// results.insert_host_result(
///     "router2",
///     HostTaskResult::skipped_with_reason("Device in maintenance mode")
/// );
///
/// // Create and add sub-task results
/// let mut validation_results = TaskResults::new("validate_config");
/// validation_results.insert_host_result(
///     "router1",
///     HostTaskResult::passed(TaskSuccess::new())
/// );
///
/// results.insert_sub_task("validate_config", validation_results);
///
/// // Query results
/// assert_eq!(results.task_name(), "deploy_config");
/// assert_eq!(results.passed_hosts().len(), 1);
/// assert!(results.sub_task("validate_config").is_some());
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskResults {
    task_name: String,
    started_at: Option<SystemTime>,
    finished_at: Option<SystemTime>,
    duration_ns: Option<u128>,
    duration_ms: Option<u128>,
    summary: Option<String>,
    hosts: CustomTreeMap<HostTaskResult>,
    sub_tasks: CustomTreeMap<TaskResults>,
}

#[derive(Serialize)]
struct TaskResultsHumanJson<'a> {
    task_name: &'a str,
    started_at: Option<String>,
    finished_at: Option<String>,
    duration: Option<String>,
    summary: Option<&'a str>,
    hosts: CustomTreeMap<HostTaskResultHumanJson<'a>>,
    sub_tasks: CustomTreeMap<TaskResultsHumanJson<'a>>,
}

#[derive(Serialize)]
enum HostTaskResultHumanJson<'a> {
    Passed(TaskSuccessHumanJson<'a>),
    Failed(TaskFailureHumanJson<'a>),
    Skipped(TaskSkipHumanJson<'a>),
}

#[derive(Serialize)]
struct TaskSuccessHumanJson<'a> {
    result: Option<&'a Value>,
    changed: bool,
    diff: Option<&'a str>,
    summary: Option<&'a str>,
    warnings: &'a [String],
    messages: &'a [TaskMessage],
    metadata: Option<&'a Value>,
    started_at: Option<String>,
    finished_at: Option<String>,
    duration: Option<String>,
}

#[derive(Serialize)]
struct TaskFailureHumanJson<'a> {
    kind: &'a TaskFailureKind,
    error_type: &'a str,
    message: &'a str,
    retryable: bool,
    details: Option<&'a Value>,
    warnings: &'a [String],
    messages: &'a [TaskMessage],
    started_at: Option<String>,
    finished_at: Option<String>,
    duration: Option<String>,
}

#[derive(Serialize)]
struct TaskSkipHumanJson<'a> {
    reason: Option<&'a str>,
    message: Option<&'a str>,
}

/// Aggregate host outcome counts for a task result node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct TaskHostSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
}

/// Recursive summary of a task result tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskResultsSummary {
    task_name: String,
    hosts: TaskHostSummary,
    duration_ns: Option<u128>,
    sub_tasks: CustomTreeMap<TaskResultsSummary>,
}

impl TaskHostSummary {
    /// Creates a new aggregate host summary.
    pub fn new(passed: usize, failed: usize, skipped: usize) -> Self {
        Self {
            passed,
            failed,
            skipped,
        }
    }

    /// Returns the number of hosts that passed.
    pub fn passed(&self) -> usize {
        self.passed
    }

    /// Returns the number of hosts that failed.
    pub fn failed(&self) -> usize {
        self.failed
    }

    /// Returns the number of hosts that were skipped.
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// Returns the total number of hosts represented in this summary.
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped
    }
}

impl TaskResultsSummary {
    /// Returns the task name for this summary node.
    pub fn task_name(&self) -> &str {
        &self.task_name
    }

    /// Returns the host outcome counts for this summary node.
    pub fn hosts(&self) -> TaskHostSummary {
        self.hosts
    }

    /// Returns the duration in milliseconds for this summary node, if available.
    pub fn duration_ms(&self) -> Option<u128> {
        self.duration_ns.map(|duration_ns| duration_ns / 1_000_000)
    }

    /// Returns the duration in a human-readable format for this summary node, if available.
    pub fn duration_display(&self) -> Option<String> {
        self.duration_ns.map(format_duration_display)
    }

    /// Returns recursive sub-task summaries keyed by task name.
    pub fn sub_tasks(&self) -> &CustomTreeMap<TaskResultsSummary> {
        &self.sub_tasks
    }
}

impl<'a> From<&'a HostTaskResult> for HostTaskResultHumanJson<'a> {
    fn from(result: &'a HostTaskResult) -> Self {
        match result {
            HostTaskResult::Passed(success) => Self::Passed(TaskSuccessHumanJson::from(success)),
            HostTaskResult::Failed(failure) => Self::Failed(TaskFailureHumanJson::from(failure)),
            HostTaskResult::Skipped(skip) => Self::Skipped(TaskSkipHumanJson::from(skip)),
        }
    }
}

impl<'a> From<&'a TaskSuccess> for TaskSuccessHumanJson<'a> {
    fn from(success: &'a TaskSuccess) -> Self {
        Self {
            result: success.result(),
            changed: success.changed(),
            diff: success.diff(),
            summary: success.summary(),
            warnings: success.warnings(),
            messages: success.messages(),
            metadata: success.metadata(),
            started_at: success.started_at_display(),
            finished_at: success.finished_at_display(),
            duration: success.duration_display(),
        }
    }
}

impl<'a> From<&'a TaskFailure> for TaskFailureHumanJson<'a> {
    fn from(failure: &'a TaskFailure) -> Self {
        Self {
            kind: failure.kind(),
            error_type: failure.error_type(),
            message: failure.message(),
            retryable: failure.retryable(),
            details: failure.details(),
            warnings: failure.warnings(),
            messages: failure.messages(),
            started_at: failure.started_at_display(),
            finished_at: failure.finished_at_display(),
            duration: failure.duration_display(),
        }
    }
}

impl<'a> From<&'a TaskSkip> for TaskSkipHumanJson<'a> {
    fn from(skip: &'a TaskSkip) -> Self {
        Self {
            reason: skip.reason(),
            message: skip.message(),
        }
    }
}

impl<'a> From<&'a TaskResults> for TaskResultsHumanJson<'a> {
    fn from(results: &'a TaskResults) -> Self {
        let mut hosts = CustomTreeMap::new();
        for (hostname, host_result) in results.hosts().iter() {
            hosts.insert(hostname, HostTaskResultHumanJson::from(host_result));
        }

        let mut sub_tasks = CustomTreeMap::new();
        for (task_name, task_results) in results.sub_tasks().iter() {
            sub_tasks.insert(task_name, TaskResultsHumanJson::from(task_results));
        }

        Self {
            task_name: results.task_name(),
            started_at: results.started_at_display(),
            finished_at: results.finished_at_display(),
            duration: results.duration_display(),
            summary: results.summary(),
            hosts,
            sub_tasks,
        }
    }
}

impl TaskResults {
    /// Creates a new `TaskResults` instance with the specified task name.
    ///
    /// This constructor initializes a `TaskResults` with the given task name and empty
    /// collections for hosts and sub-tasks. All timing and summary fields are set to `None`.
    ///
    /// # Parameters
    ///
    /// * `task_name` - The name of the task. Can be any type that implements `Into<String>`,
    ///   such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// A new `TaskResults` instance with the specified task name and default values for all
    /// other fields.
    pub fn new(task_name: impl Into<String>) -> Self {
        Self {
            task_name: task_name.into(),
            started_at: None,
            finished_at: None,
            duration_ns: None,
            duration_ms: None,
            summary: None,
            hosts: CustomTreeMap::new(),
            sub_tasks: CustomTreeMap::new(),
        }
    }

    /// Returns the name of the task.
    ///
    /// # Returns
    ///
    /// A string slice containing the task name.
    pub fn task_name(&self) -> &str {
        &self.task_name
    }

    /// Sets the task execution start timestamp.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `started_at` - The timestamp when the task execution started.
    ///
    /// # Returns
    ///
    /// The modified `TaskResults` instance with the start timestamp set.
    pub fn with_started_at(mut self, started_at: SystemTime) -> Self {
        self.started_at = Some(started_at);
        self
    }

    /// Sets the task execution finish timestamp.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `finished_at` - The timestamp when the task execution finished.
    ///
    /// # Returns
    ///
    /// The modified `TaskResults` instance with the finish timestamp set.
    pub fn with_finished_at(mut self, finished_at: SystemTime) -> Self {
        self.finished_at = Some(finished_at);
        self
    }

    /// Sets the task execution duration in milliseconds.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `duration_ms` - The duration of the task execution in milliseconds.
    ///
    /// # Returns
    ///
    /// The modified `TaskResults` instance with the duration set.
    pub fn with_duration_ms(mut self, duration_ms: u128) -> Self {
        self.duration_ns = Some(duration_ms.saturating_mul(1_000_000));
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Sets the task execution duration in nanoseconds.
    pub fn with_duration_ns(mut self, duration_ns: u128) -> Self {
        self.duration_ns = Some(duration_ns);
        self.duration_ms = Some(duration_ns / 1_000_000);
        self
    }

    /// Sets a summary message describing the task execution.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `summary` - A human-readable summary message. Can be any type that implements
    ///   `Into<String>`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// The modified `TaskResults` instance with the summary set.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Merges another result tree for the same task into this one.
    ///
    /// Host results are inserted directly and sub-task trees are merged
    /// recursively. Aggregate timing is widened to cover the full execution
    /// window across both result trees.
    pub fn merge(&mut self, other: TaskResults) {
        let mut other = other;
        debug_assert_eq!(self.task_name, other.task_name);

        if let (Some(started_at), Some(finished_at)) = (other.started_at, other.finished_at) {
            self.record_execution_timing(started_at, finished_at);
        } else {
            if self.started_at.is_none() {
                self.started_at = other.started_at;
            }
            if self.finished_at.is_none() {
                self.finished_at = other.finished_at;
            }
            if self.duration_ns.is_none() {
                self.duration_ns = other.duration_ns;
            }
            if self.duration_ms.is_none() {
                self.duration_ms = other.duration_ms;
            }
        }

        if self.summary.is_none() {
            self.summary = other.summary;
        }

        for (hostname, result) in std::mem::take(&mut *other.hosts).into_iter() {
            self.insert_host_result(hostname, result);
        }

        for (task_name, sub_results) in std::mem::take(&mut *other.sub_tasks).into_iter() {
            if let Some(existing) = self.sub_task_mut(task_name.as_str()) {
                existing.merge(sub_results);
            } else {
                self.insert_sub_task(task_name, sub_results);
            }
        }
    }

    fn record_execution_timing(&mut self, started_at: SystemTime, finished_at: SystemTime) {
        if self.started_at.is_none_or(|current| started_at < current) {
            self.started_at = Some(started_at);
        }

        if self.finished_at.is_none_or(|current| finished_at > current) {
            self.finished_at = Some(finished_at);
        }

        if let (Some(started_at), Some(finished_at)) = (self.started_at, self.finished_at) {
            let duration_ns = finished_at
                .duration_since(started_at)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            self.duration_ns = Some(duration_ns);
            self.duration_ms = Some(duration_ns / 1_000_000);
        }
    }

    /// Returns the task execution start timestamp, if available.
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if the start timestamp was set, `None` otherwise.
    pub fn started_at(&self) -> Option<SystemTime> {
        self.started_at
    }

    /// Returns the task execution finish timestamp, if available.
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if the finish timestamp was set, `None` otherwise.
    pub fn finished_at(&self) -> Option<SystemTime> {
        self.finished_at
    }

    /// Returns the task execution duration in milliseconds, if available.
    ///
    /// # Returns
    ///
    /// `Some(u128)` if the duration was set, `None` otherwise.
    pub fn duration_ms(&self) -> Option<u128> {
        self.duration_ns
            .map(|duration_ns| duration_ns / 1_000_000)
            .or(self.duration_ms)
    }

    /// Returns the task execution duration in nanoseconds, if available.
    pub fn duration_ns(&self) -> Option<u128> {
        self.duration_ns.or_else(|| {
            self.duration_ms
                .map(|duration_ms| duration_ms.saturating_mul(1_000_000))
        })
    }

    /// Returns the task execution start timestamp in RFC 3339 format, if available.
    pub fn started_at_display(&self) -> Option<String> {
        self.started_at.map(format_timestamp_display)
    }

    /// Returns the task execution finish timestamp in RFC 3339 format, if available.
    pub fn finished_at_display(&self) -> Option<String> {
        self.finished_at.map(format_timestamp_display)
    }

    /// Returns the task execution duration in a human-readable format, if available.
    pub fn duration_display(&self) -> Option<String> {
        self.duration_ns().map(format_duration_display)
    }

    /// Serializes task results as compact human-readable JSON.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&TaskResultsHumanJson::from(self))
    }

    /// Serializes task results as pretty-printed human-readable JSON.
    pub fn to_pretty_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&TaskResultsHumanJson::from(self))
    }

    /// Serializes task results as compact raw JSON using the struct's default serde representation.
    pub fn to_raw_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serializes task results as pretty-printed raw JSON using the struct's default serde representation.
    pub fn to_raw_pretty_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Returns the task summary message, if available.
    ///
    /// # Returns
    ///
    /// `Some(&str)` if a summary was set, `None` otherwise.
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    /// Inserts or updates the execution result for a specific host.
    ///
    /// If a result already exists for the given hostname, it will be replaced with the new result.
    ///
    /// # Parameters
    ///
    /// * `hostname` - The hostname to associate with the result. Can be any type that implements
    ///   `Into<NatString>`, such as `&str`, `String`, or `NatString`.
    /// * `result` - The `HostTaskResult` containing the execution outcome for this host.
    pub fn insert_host_result<K>(&mut self, hostname: K, result: HostTaskResult)
    where
        K: Into<NatString>,
    {
        self.hosts.insert(hostname.into(), result);
    }

    /// Retrieves the execution result for a specific host.
    ///
    /// # Parameters
    ///
    /// * `hostname` - The hostname to look up.
    ///
    /// # Returns
    ///
    /// `Some(&HostTaskResult)` if a result exists for the hostname, `None` otherwise.
    pub fn host_result(&self, hostname: &str) -> Option<&HostTaskResult> {
        self.hosts.get(hostname)
    }

    /// Retrieves a mutable reference to the execution result for a specific host.
    ///
    /// # Parameters
    ///
    /// * `hostname` - The hostname to look up.
    ///
    /// # Returns
    ///
    /// `Some(&mut HostTaskResult)` if a result exists for the hostname, `None` otherwise.
    pub fn host_result_mut(&mut self, hostname: &str) -> Option<&mut HostTaskResult> {
        self.hosts.get_mut(hostname)
    }

    /// Returns a reference to the map of all host results.
    ///
    /// # Returns
    ///
    /// A reference to the `CustomTreeMap` containing hostname to `HostTaskResult` mappings.
    pub fn hosts(&self) -> &CustomTreeMap<HostTaskResult> {
        &self.hosts
    }

    /// Inserts or updates the results for a sub-task.
    ///
    /// If results already exist for the given sub-task name, they will be replaced with the new results.
    ///
    /// # Parameters
    ///
    /// * `task_name` - The name of the sub-task. Can be any type that implements `Into<NatString>`,
    ///   such as `&str`, `String`, or `NatString`.
    /// * `results` - The `TaskResults` containing the execution results for this sub-task.
    pub fn insert_sub_task<K>(&mut self, task_name: K, results: TaskResults)
    where
        K: Into<NatString>,
    {
        self.sub_tasks.insert(task_name.into(), results);
    }

    /// Retrieves the results for a specific sub-task.
    ///
    /// # Parameters
    ///
    /// * `task_name` - The name of the sub-task to look up.
    ///
    /// # Returns
    ///
    /// `Some(&TaskResults)` if results exist for the sub-task, `None` otherwise.
    pub fn sub_task(&self, task_name: &str) -> Option<&TaskResults> {
        self.sub_tasks.get(task_name)
    }

    /// Retrieves a mutable reference to the results for a specific sub-task.
    ///
    /// # Parameters
    ///
    /// * `task_name` - The name of the sub-task to look up.
    ///
    /// # Returns
    ///
    /// `Some(&mut TaskResults)` if results exist for the sub-task, `None` otherwise.
    pub fn sub_task_mut(&mut self, task_name: &str) -> Option<&mut TaskResults> {
        self.sub_tasks.get_mut(task_name)
    }

    /// Returns a reference to the map of all sub-task results.
    ///
    /// # Returns
    ///
    /// A reference to the `CustomTreeMap` containing sub-task name to `TaskResults` mappings.
    pub fn sub_tasks(&self) -> &CustomTreeMap<TaskResults> {
        &self.sub_tasks
    }

    /// Returns a list of hostnames for which the task execution passed.
    ///
    /// This method filters the host results and collects the hostnames where the
    /// `HostTaskResult` indicates a successful execution (passed state).
    ///
    /// # Returns
    ///
    /// A `Vec` containing references to the hostnames of all hosts where the task passed.
    pub fn passed_hosts(&self) -> Vec<&NatString> {
        self.hosts
            .iter()
            .filter_map(|(host, result)| result.is_passed().then_some(host))
            .collect()
    }

    /// Returns a list of hostnames for which the task execution failed.
    ///
    /// This method filters the host results and collects the hostnames where the
    /// `HostTaskResult` indicates a failed execution.
    ///
    /// # Returns
    ///
    /// A `Vec` containing references to the hostnames of all hosts where the task failed.
    pub fn failed_hosts(&self) -> Vec<&NatString> {
        self.hosts
            .iter()
            .filter_map(|(host, result)| result.is_failed().then_some(host))
            .collect()
    }

    /// Returns a list of hostnames for which the task execution was skipped.
    pub fn skipped_hosts(&self) -> Vec<&NatString> {
        self.hosts
            .iter()
            .filter_map(|(host, result)| result.is_skipped().then_some(host))
            .collect()
    }

    /// Returns aggregate host counts for this task only.
    pub fn host_summary(&self) -> TaskHostSummary {
        TaskHostSummary::new(
            self.passed_hosts().len(),
            self.failed_hosts().len(),
            self.skipped_hosts().len(),
        )
    }

    /// Returns a recursive summary of this task and all sub-tasks.
    pub fn task_summary(&self) -> TaskResultsSummary {
        let mut sub_tasks = CustomTreeMap::new();
        for (task_name, task_results) in self.sub_tasks().iter() {
            sub_tasks.insert(task_name, task_results.task_summary());
        }

        TaskResultsSummary {
            task_name: self.task_name.clone(),
            hosts: self.host_summary(),
            duration_ns: self.duration_ns(),
            sub_tasks,
        }
    }
}

/// Represents the execution outcome of a task on a single host.
///
/// `HostTaskResult` captures one of three possible states for a task execution:
/// - **Passed**: The task completed successfully, potentially with changes, warnings, or metadata.
/// - **Failed**: The task encountered an error and could not complete successfully.
/// - **Skipped**: The task was not executed, typically due to conditional logic or dependencies.
///
/// This enum provides a type-safe way to represent task outcomes and includes helper methods
/// to query the result state and extract the underlying success, failure, or skip details.
///
/// # Variants
///
/// * `Passed(TaskSuccess)` - The task executed successfully. Contains detailed information about
///   the execution including any results, changes made, warnings, and timing information.
///
/// * `Failed(TaskFailure)` - The task failed during execution. Contains error information,
///   failure classification, retry hints, and any warnings or messages collected before failure.
///
/// * `Skipped(TaskSkip)` - The task was skipped and not executed. Contains optional reason
///   and message explaining why the task was skipped.
///
/// # Example
///
/// ```rust
/// use genja_core::task::{HostTaskResult, TaskSuccess, TaskFailure};
///
/// // Create a successful result
/// let success = HostTaskResult::passed(
///     TaskSuccess::new()
///         .with_changed(true)
///         .with_summary("Configuration updated")
/// );
///
/// // Check the result state
/// assert!(success.is_passed());
/// assert!(!success.is_failed());
///
/// // Extract success details
/// if let Some(details) = success.success() {
///     assert!(details.changed());
/// }
///
/// // Create a skipped result
/// let skipped = HostTaskResult::skipped_with_reason("Host in maintenance mode");
/// assert!(skipped.is_skipped());
/// ```
#[derive(Debug, Clone, Serialize)]
pub enum HostTaskResult {
    Passed(TaskSuccess),
    Failed(TaskFailure),
    Skipped(TaskSkip),
}

impl HostTaskResult {
    /// Creates a new `HostTaskResult` representing a successful task execution.
    ///
    /// This constructor wraps a `TaskSuccess` instance in the `Passed` variant,
    /// indicating that the task completed successfully on the host.
    ///
    /// # Parameters
    ///
    /// * `result` - The `TaskSuccess` containing details about the successful execution,
    ///   including any results, changes made, warnings, and timing information.
    ///
    /// # Returns
    ///
    /// A `HostTaskResult::Passed` variant containing the provided success details.
    pub fn passed(result: TaskSuccess) -> Self {
        Self::Passed(result)
    }

    /// Creates a new `HostTaskResult` representing a failed task execution.
    ///
    /// This constructor wraps a `TaskFailure` instance in the `Failed` variant,
    /// indicating that the task encountered an error and could not complete successfully.
    ///
    /// # Parameters
    ///
    /// * `failure` - The `TaskFailure` containing error information, failure classification,
    ///   retry hints, and any warnings or messages collected before failure.
    ///
    /// # Returns
    ///
    /// A `HostTaskResult::Failed` variant containing the provided failure details.
    pub fn failed(failure: TaskFailure) -> Self {
        Self::Failed(failure)
    }

    /// Creates a new `HostTaskResult` representing a skipped task execution.
    ///
    /// This constructor creates a `Skipped` variant with default (empty) skip details,
    /// indicating that the task was not executed on the host.
    ///
    /// # Returns
    ///
    /// A `HostTaskResult::Skipped` variant with default skip information (no reason or message).
    pub fn skipped() -> Self {
        Self::Skipped(TaskSkip::default())
    }

    /// Creates a new `HostTaskResult` representing a skipped task execution with a reason.
    ///
    /// This constructor creates a `Skipped` variant with a specified reason explaining
    /// why the task was not executed on the host.
    ///
    /// # Parameters
    ///
    /// * `reason` - A machine-readable reason code or identifier explaining why the task
    ///   was skipped. Can be any type that implements `Into<String>`, such as `&str`,
    ///   `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// A `HostTaskResult::Skipped` variant with the specified reason set.
    pub fn skipped_with_reason(reason: impl Into<String>) -> Self {
        Self::Skipped(TaskSkip::new().with_reason(reason))
    }

    /// Checks if the task execution passed (completed successfully).
    ///
    /// # Returns
    ///
    /// `true` if this result represents a successful task execution (`Passed` variant),
    /// `false` otherwise.
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed(_))
    }

    /// Checks if the task execution failed.
    ///
    /// # Returns
    ///
    /// `true` if this result represents a failed task execution (`Failed` variant),
    /// `false` otherwise.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Checks if the task execution was skipped.
    ///
    /// # Returns
    ///
    /// `true` if this result represents a skipped task execution (`Skipped` variant),
    /// `false` otherwise.
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped(_))
    }

    /// Retrieves the success details if the task passed.
    ///
    /// This method extracts the `TaskSuccess` from a `Passed` variant, providing
    /// access to execution results, changes, warnings, and other success metadata.
    ///
    /// # Returns
    ///
    /// `Some(&TaskSuccess)` if this is a `Passed` result, `None` if the task failed
    /// or was skipped.
    pub fn success(&self) -> Option<&TaskSuccess> {
        match self {
            Self::Passed(success) => Some(success),
            Self::Failed(_) | Self::Skipped(_) => None,
        }
    }

    /// Retrieves the failure details if the task failed.
    ///
    /// This method extracts the `TaskFailure` from a `Failed` variant, providing
    /// access to error information, failure classification, and retry hints.
    ///
    /// # Returns
    ///
    /// `Some(&TaskFailure)` if this is a `Failed` result, `None` if the task passed
    /// or was skipped.
    pub fn failure(&self) -> Option<&TaskFailure> {
        match self {
            Self::Failed(failure) => Some(failure),
            Self::Passed(_) | Self::Skipped(_) => None,
        }
    }

    /// Retrieves the skip details if the task was skipped.
    ///
    /// This method extracts the `TaskSkip` from a `Skipped` variant, providing
    /// access to the reason and message explaining why the task was not executed.
    ///
    /// # Returns
    ///
    /// `Some(&TaskSkip)` if this is a `Skipped` result, `None` if the task passed
    /// or failed.
    pub fn skipped_detail(&self) -> Option<&TaskSkip> {
        match self {
            Self::Skipped(skip) => Some(skip),
            Self::Passed(_) | Self::Failed(_) => None,
        }
    }

    fn with_execution_timing(
        self,
        started_at: SystemTime,
        finished_at: SystemTime,
        duration_ns: u128,
    ) -> Self {
        match self {
            Self::Passed(success) => Self::Passed(
                success
                    .with_started_at(started_at)
                    .with_finished_at(finished_at)
                    .with_duration_ns(duration_ns),
            ),
            Self::Failed(failure) => Self::Failed(
                failure
                    .with_started_at(started_at)
                    .with_finished_at(finished_at)
                    .with_duration_ns(duration_ns),
            ),
            Self::Skipped(skip) => Self::Skipped(skip),
        }
    }
}

/// Represents the successful execution of a task on a host.
///
/// `TaskSuccess` captures detailed information about a task that completed successfully,
/// including the execution result, whether changes were made, timing information, and any
/// warnings or messages generated during execution. This structure provides a comprehensive
/// view of what happened during task execution, even when the task succeeded.
///
/// # Fields
///
/// * `result` - The structured result data produced by the task, if any. This can contain
///   arbitrary JSON data representing the task's output.
///
/// * `changed` - Indicates whether the task made any changes to the target system. This is
///   important for idempotency tracking and reporting.
///
/// * `diff` - A textual representation of changes made, useful for showing what was modified
///   before and after the task execution.
///
/// * `summary` - A human-readable summary message describing what the task accomplished.
///
/// * `warnings` - A list of warning messages generated during execution. Warnings indicate
///   potential issues that didn't prevent success but may require attention.
///
/// * `messages` - Structured messages with levels (Info, Warning, Error, Debug) that provide
///   detailed execution information beyond simple warnings.
///
/// * `metadata` - Additional structured metadata about the execution, such as version information,
///   configuration details, or other contextual data.
///
/// * `started_at` - The timestamp when the task execution started, if available.
///
/// * `finished_at` - The timestamp when the task execution finished, if available.
///
/// * `duration_ms` - The duration of the task execution in milliseconds, if available.
///
/// # Example
///
/// ```rust
/// use genja_core::task::TaskSuccess;
/// use serde_json::json;
///
/// let success = TaskSuccess::new()
///     .with_result(json!({"status": "deployed"}))
///     .with_changed(true)
///     .with_summary("Configuration deployed successfully")
///     .with_warning("Using deprecated configuration format")
///     .with_diff("- old_value\n+ new_value");
///
/// assert!(success.changed());
/// assert_eq!(success.warnings().len(), 1);
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskSuccess {
    result: Option<Value>,
    changed: bool,
    diff: Option<String>,
    summary: Option<String>,
    warnings: Vec<String>,
    messages: Vec<TaskMessage>,
    metadata: Option<Value>,
    started_at: Option<SystemTime>,
    finished_at: Option<SystemTime>,
    duration_ns: Option<u128>,
    duration_ms: Option<u128>,
}

impl TaskSuccess {
    /// Creates a new `TaskSuccess` instance with default values.
    ///
    /// This constructor initializes a `TaskSuccess` with all fields set to their default values:
    /// no result data, no changes made, no diff, no summary, empty warnings and messages lists,
    /// no metadata, and no timing information.
    ///
    /// # Returns
    ///
    /// A new `TaskSuccess` instance with default values for all fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the structured result data produced by the task.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The result can contain arbitrary JSON data representing
    /// the task's output, such as configuration details, status information, or any other
    /// structured data relevant to the task execution.
    ///
    /// # Parameters
    ///
    /// * `result` - A `serde_json::Value` containing the structured result data.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the result data set.
    pub fn with_result(mut self, result: Value) -> Self {
        self.result = Some(result);
        self
    }

    /// Sets whether the task made changes to the target system.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The changed flag is important for idempotency tracking
    /// and reporting, indicating whether the task modified the system state or found it
    /// already in the desired state.
    ///
    /// # Parameters
    ///
    /// * `changed` - `true` if the task made changes to the system, `false` if no changes
    ///   were necessary or made.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the changed flag set.
    pub fn with_changed(mut self, changed: bool) -> Self {
        self.changed = changed;
        self
    }

    /// Sets a textual representation of changes made by the task.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The diff typically shows what was modified, often
    /// in a before/after format, making it easy to understand what changed during execution.
    ///
    /// # Parameters
    ///
    /// * `diff` - A textual representation of the changes. Can be any type that implements
    ///   `Into<String>`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the diff set.
    pub fn with_diff(mut self, diff: impl Into<String>) -> Self {
        self.diff = Some(diff.into());
        self
    }

    /// Sets a human-readable summary message describing what the task accomplished.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The summary provides a concise description of the
    /// task's outcome, useful for logging and reporting.
    ///
    /// # Parameters
    ///
    /// * `summary` - A human-readable summary message. Can be any type that implements
    ///   `Into<String>`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the summary set.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Adds a warning message to the list of warnings generated during execution.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. Warnings indicate potential issues that didn't prevent
    /// success but may require attention. Multiple warnings can be added by calling this
    /// method multiple times.
    ///
    /// # Parameters
    ///
    /// * `warning` - A warning message. Can be any type that implements `Into<String>`,
    ///   such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the warning added to the warnings list.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Adds a structured message to the list of messages generated during execution.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. Messages provide detailed execution information with
    /// associated severity levels. Multiple messages can be added by calling this method
    /// multiple times.
    ///
    /// # Parameters
    ///
    /// * `message` - A `TaskMessage` containing the message text, severity level, and
    ///   optional code and timestamp.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the message added to the messages list.
    pub fn with_message(mut self, message: TaskMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Sets additional structured metadata about the execution.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. Metadata can contain arbitrary JSON data such as
    /// version information, configuration details, or other contextual data relevant to
    /// the task execution.
    ///
    /// # Parameters
    ///
    /// * `metadata` - A `serde_json::Value` containing the structured metadata.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the metadata set.
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the task execution start timestamp.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `started_at` - The timestamp when the task execution started.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the start timestamp set.
    pub fn with_started_at(mut self, started_at: SystemTime) -> Self {
        self.started_at = Some(started_at);
        self
    }

    /// Sets the task execution finish timestamp.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `finished_at` - The timestamp when the task execution finished.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the finish timestamp set.
    pub fn with_finished_at(mut self, finished_at: SystemTime) -> Self {
        self.finished_at = Some(finished_at);
        self
    }

    /// Sets the task execution duration in milliseconds.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `duration_ms` - The duration of the task execution in milliseconds.
    ///
    /// # Returns
    ///
    /// The modified `TaskSuccess` instance with the duration set.
    pub fn with_duration_ms(mut self, duration_ms: u128) -> Self {
        self.duration_ns = Some(duration_ms.saturating_mul(1_000_000));
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Sets the task execution duration in nanoseconds.
    pub fn with_duration_ns(mut self, duration_ns: u128) -> Self {
        self.duration_ns = Some(duration_ns);
        self.duration_ms = Some(duration_ns / 1_000_000);
        self
    }

    /// Returns the structured result data produced by the task, if available.
    ///
    /// # Returns
    ///
    /// `Some(&Value)` if result data was set, `None` otherwise.
    pub fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    /// Returns whether the task made changes to the target system.
    ///
    /// # Returns
    ///
    /// `true` if the task made changes, `false` if no changes were made.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Returns the textual representation of changes made, if available.
    ///
    /// # Returns
    ///
    /// `Some(&str)` if a diff was set, `None` otherwise.
    pub fn diff(&self) -> Option<&str> {
        self.diff.as_deref()
    }

    /// Returns the task summary message, if available.
    ///
    /// # Returns
    ///
    /// `Some(&str)` if a summary was set, `None` otherwise.
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    /// Returns a slice of all warning messages generated during execution.
    ///
    /// # Returns
    ///
    /// A slice containing all warning messages. Returns an empty slice if no warnings
    /// were generated.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Returns a slice of all structured messages generated during execution.
    ///
    /// # Returns
    ///
    /// A slice containing all `TaskMessage` instances. Returns an empty slice if no
    /// messages were generated.
    pub fn messages(&self) -> &[TaskMessage] {
        &self.messages
    }

    /// Returns the additional structured metadata, if available.
    ///
    /// # Returns
    ///
    /// `Some(&Value)` if metadata was set, `None` otherwise.
    pub fn metadata(&self) -> Option<&Value> {
        self.metadata.as_ref()
    }

    /// Returns the task execution start timestamp, if available.
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if the start timestamp was set, `None` otherwise.
    pub fn started_at(&self) -> Option<SystemTime> {
        self.started_at
    }

    /// Returns the task execution finish timestamp, if available.
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if the finish timestamp was set, `None` otherwise.
    pub fn finished_at(&self) -> Option<SystemTime> {
        self.finished_at
    }

    /// Returns the task execution duration in milliseconds, if available.
    ///
    /// # Returns
    ///
    /// `Some(u128)` if the duration was set, `None` otherwise.
    pub fn duration_ms(&self) -> Option<u128> {
        self.duration_ns
            .map(|duration_ns| duration_ns / 1_000_000)
            .or(self.duration_ms)
    }

    /// Returns the task execution duration in nanoseconds, if available.
    pub fn duration_ns(&self) -> Option<u128> {
        self.duration_ns.or_else(|| {
            self.duration_ms
                .map(|duration_ms| duration_ms.saturating_mul(1_000_000))
        })
    }

    /// Returns the task execution start timestamp in RFC 3339 format, if available.
    pub fn started_at_display(&self) -> Option<String> {
        self.started_at.map(format_timestamp_display)
    }

    /// Returns the task execution finish timestamp in RFC 3339 format, if available.
    pub fn finished_at_display(&self) -> Option<String> {
        self.finished_at.map(format_timestamp_display)
    }

    /// Returns the task execution duration in a human-readable format, if available.
    pub fn duration_display(&self) -> Option<String> {
        self.duration_ns().map(format_duration_display)
    }
}

/// Represents a failed task execution with comprehensive error information and context.
///
/// `TaskFailure` captures detailed information about why a task failed, including the underlying
/// error, failure classification, retry hints, and any warnings or messages collected during
/// execution before the failure occurred. This structure provides rich context for error handling,
/// logging, and determining whether a failed task should be retried.
///
/// The failure information includes timing data, structured details about the error, and the
/// ability to downcast to specific error types for specialized error handling.
///
/// # Fields
///
/// * `error` - The underlying error that caused the task to fail. This is a thread-safe,
///   reference-counted error that can be downcast to specific error types. Not serialized.
///
/// * `kind` - The classification of the failure (e.g., Connection, Authentication, Timeout).
///   This helps categorize errors for reporting and handling purposes.
///
/// * `error_type` - A string representation of the error's type name, useful for debugging
///   and logging when the actual error type information is needed.
///
/// * `message` - A human-readable error message describing what went wrong. This is typically
///   derived from the error's `Display` implementation.
///
/// * `retryable` - Indicates whether this failure is potentially transient and the task could
///   succeed if retried. This helps automation systems decide retry strategies.
///
/// * `details` - Optional structured data providing additional context about the failure,
///   such as error codes, affected resources, or diagnostic information.
///
/// * `warnings` - A list of warning messages that were generated during execution before the
///   failure occurred. These can provide context about what led to the failure.
///
/// * `messages` - Structured messages with levels (Info, Warning, Error, Debug) that were
///   collected during execution, providing a detailed execution trace up to the point of failure.
///
/// * `started_at` - The timestamp when the task execution started, if available.
///
/// * `finished_at` - The timestamp when the task execution failed, if available.
///
/// * `duration_ms` - The duration of the task execution in milliseconds before it failed,
///   if available.
///
/// # Example
///
/// ```rust
/// use genja_core::task::{TaskFailure, TaskFailureKind, TaskMessage, MessageLevel};
/// use serde_json::json;
/// use std::io;
///
/// let failure = TaskFailure::new(io::Error::new(io::ErrorKind::TimedOut, "connection timeout"))
///     .with_kind(TaskFailureKind::Timeout)
///     .with_retryable(true)
///     .with_details(json!({"timeout_seconds": 30}))
///     .with_warning("Slow network detected")
///     .with_message(TaskMessage::new(MessageLevel::Error, "Failed to connect to host"));
///
/// assert!(failure.retryable());
/// assert!(matches!(failure.kind(), TaskFailureKind::Timeout));
/// assert_eq!(failure.warnings().len(), 1);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct TaskFailure {
    #[serde(skip)]
    error: TaskError,
    #[serde(skip)]
    source: Option<TaskFailureSource>,
    kind: TaskFailureKind,
    error_type: String,
    message: String,
    retryable: bool,
    details: Option<Value>,
    warnings: Vec<String>,
    messages: Vec<TaskMessage>,
    started_at: Option<SystemTime>,
    finished_at: Option<SystemTime>,
    duration_ns: Option<u128>,
    duration_ms: Option<u128>,
}

impl TaskFailure {
    /// Creates a new `TaskFailure` instance from an error.
    ///
    /// This constructor wraps any error type that implements the standard `Error` trait
    /// in a `TaskFailure`, capturing the error message and type information. The failure
    /// is initialized with default values: classified as `Internal`, not retryable, with
    /// no additional details, warnings, or messages, and no timing information.
    ///
    /// The error is stored as a thread-safe, reference-counted pointer (`Arc<dyn Error>`),
    /// allowing it to be cloned and shared across threads while preserving the ability
    /// to downcast to the original error type for specialized error handling.
    ///
    /// # Parameters
    ///
    /// * `error` - The error that caused the task to fail. Must implement `Error + Send + Sync + 'static`,
    ///   ensuring it can be safely shared across threads and stored for the lifetime of the program.
    ///
    /// # Returns
    ///
    /// A new `TaskFailure` instance with the error wrapped and default values for all other fields.
    /// The failure kind is set to `Internal`, retryable is `false`, and all optional fields are `None`.
    pub fn new<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        let source = Arc::new(error);
        let message = source.to_string();
        let error_type = type_name::<E>().to_string();

        Self {
            kind: TaskFailureKind::Internal,
            error_type: error_type.clone(),
            message,
            error: TaskError {
                error: source.clone(),
                error_type,
                source: Some(source.clone()),
            },
            source: Some(source),
            retryable: false,
            details: None,
            warnings: Vec::new(),
            messages: Vec::new(),
            started_at: None,
            finished_at: None,
            duration_ns: None,
            duration_ms: None,
        }
    }

    /// Creates a new `TaskFailure` from any thread-safe `'static` payload.
    ///
    /// This is useful when the failure value does not implement [`Error`] but
    /// still carries meaningful type and display information that should be
    /// stored with the task result.
    pub fn capture<E>(error: E) -> Self
    where
        E: fmt::Debug + fmt::Display + Send + Sync + 'static,
    {
        let source = Arc::new(error);
        let message = source.to_string();
        let error_type = type_name::<E>().to_string();
        let wrapped_error = Arc::new(CapturedTaskFailure {
            message: message.clone(),
        });

        Self {
            kind: TaskFailureKind::Internal,
            error_type: error_type.clone(),
            message,
            error: TaskError {
                error: wrapped_error,
                error_type,
                source: Some(source.clone()),
            },
            source: Some(source),
            retryable: false,
            details: None,
            warnings: Vec::new(),
            messages: Vec::new(),
            started_at: None,
            finished_at: None,
            duration_ns: None,
            duration_ms: None,
        }
    }

    /// Creates a new `TaskFailure` from an already-erased task error.
    ///
    /// Failures captured through the task execution boundary default to
    /// [`TaskFailureKind::External`], because the error originated outside the
    /// Genja framework itself.
    pub fn from_task_error(error: TaskError) -> Self {
        Self {
            kind: TaskFailureKind::External,
            error_type: error.error_type().to_string(),
            message: error.to_string(),
            error: error.clone(),
            source: error.source,
            retryable: false,
            details: None,
            warnings: Vec::new(),
            messages: Vec::new(),
            started_at: None,
            finished_at: None,
            duration_ns: None,
            duration_ms: None,
        }
    }

    /// Sets the failure classification kind.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The kind categorizes the failure type (e.g., Connection,
    /// Authentication, Timeout) to help with error handling and reporting.
    ///
    /// # Parameters
    ///
    /// * `kind` - The `TaskFailureKind` classification for this failure.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the failure kind set.
    pub fn with_kind(mut self, kind: TaskFailureKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets whether this failure is retryable.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The retryable flag indicates whether the failure
    /// is potentially transient and the task could succeed if retried.
    ///
    /// # Parameters
    ///
    /// * `retryable` - `true` if the task should be retried after this failure, `false` if
    ///   the failure is permanent and retrying would not help.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the retryable flag set.
    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    /// Sets additional structured details about the failure.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The details can contain arbitrary JSON data providing
    /// additional context about the failure, such as error codes, affected resources, or
    /// diagnostic information.
    ///
    /// # Parameters
    ///
    /// * `details` - A `serde_json::Value` containing the structured failure details.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the details set.
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Adds a warning message to the list of warnings generated before the failure.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. Warnings provide context about what occurred during
    /// execution before the failure happened. Multiple warnings can be added by calling
    /// this method multiple times.
    ///
    /// # Parameters
    ///
    /// * `warning` - A warning message. Can be any type that implements `Into<String>`,
    ///   such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the warning added to the warnings list.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Adds a structured message to the list of messages generated before the failure.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. Messages provide detailed execution information with
    /// associated severity levels that were collected before the failure occurred. Multiple
    /// messages can be added by calling this method multiple times.
    ///
    /// # Parameters
    ///
    /// * `message` - A `TaskMessage` containing the message text, severity level, and
    ///   optional code and timestamp.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the message added to the messages list.
    pub fn with_message(mut self, message: TaskMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Sets the task execution start timestamp.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `started_at` - The timestamp when the task execution started.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the start timestamp set.
    pub fn with_started_at(mut self, started_at: SystemTime) -> Self {
        self.started_at = Some(started_at);
        self
    }

    /// Sets the task execution finish timestamp.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `finished_at` - The timestamp when the task execution failed.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the finish timestamp set.
    pub fn with_finished_at(mut self, finished_at: SystemTime) -> Self {
        self.finished_at = Some(finished_at);
        self
    }

    /// Sets the task execution duration in milliseconds.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining.
    ///
    /// # Parameters
    ///
    /// * `duration_ms` - The duration of the task execution in milliseconds before it failed.
    ///
    /// # Returns
    ///
    /// The modified `TaskFailure` instance with the duration set.
    pub fn with_duration_ms(mut self, duration_ms: u128) -> Self {
        self.duration_ns = Some(duration_ms.saturating_mul(1_000_000));
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Sets the task execution duration in nanoseconds.
    pub fn with_duration_ns(mut self, duration_ns: u128) -> Self {
        self.duration_ns = Some(duration_ns);
        self.duration_ms = Some(duration_ns / 1_000_000);
        self
    }

    /// Attempts to downcast the underlying error to a specific error type.
    ///
    /// This method provides type-safe access to the original error type, allowing
    /// specialized error handling based on the concrete error type. If the underlying
    /// error is of type `E`, this returns a reference to it; otherwise, it returns `None`.
    ///
    /// # Type Parameters
    ///
    /// * `E` - The concrete error type to downcast to. Must implement `Error + 'static`.
    ///
    /// # Returns
    ///
    /// `Some(&E)` if the underlying error is of type `E`, `None` otherwise.
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: 'static,
    {
        self.source
            .as_ref()
            .and_then(|source| source.downcast_ref::<E>())
    }

    /// Returns a reference to the underlying error as a trait object.
    ///
    /// This method provides access to the error through the `Error` trait interface,
    /// allowing generic error handling without knowing the concrete error type.
    ///
    /// # Returns
    ///
    /// A reference to the underlying error as a trait object implementing
    /// `Error + Send + Sync + 'static`.
    pub fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.error.error()
    }

    /// Returns the type name of the underlying error.
    ///
    /// This method provides the fully qualified type name of the error as a string,
    /// which is useful for debugging and logging when you need to know the concrete
    /// error type without downcasting.
    ///
    /// # Returns
    ///
    /// A string slice containing the fully qualified type name of the error.
    pub fn error_type(&self) -> &str {
        &self.error_type
    }

    /// Returns the human-readable error message.
    ///
    /// This message is derived from the error's `Display` implementation and provides
    /// a description of what went wrong during task execution.
    ///
    /// # Returns
    ///
    /// A string slice containing the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the failure classification kind.
    ///
    /// # Returns
    ///
    /// A reference to the `TaskFailureKind` indicating the category of this failure.
    pub fn kind(&self) -> &TaskFailureKind {
        &self.kind
    }

    /// Returns whether this failure is retryable.
    ///
    /// # Returns
    ///
    /// `true` if the task should be retried after this failure, `false` if the failure
    /// is permanent and retrying would not help.
    pub fn retryable(&self) -> bool {
        self.retryable
    }

    /// Returns the additional structured details about the failure, if available.
    ///
    /// # Returns
    ///
    /// `Some(&Value)` if failure details were set, `None` otherwise.
    pub fn details(&self) -> Option<&Value> {
        self.details.as_ref()
    }

    /// Returns a slice of all warning messages generated before the failure.
    ///
    /// # Returns
    ///
    /// A slice containing all warning messages. Returns an empty slice if no warnings
    /// were generated.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Returns a slice of all structured messages generated before the failure.
    ///
    /// # Returns
    ///
    /// A slice containing all `TaskMessage` instances. Returns an empty slice if no
    /// messages were generated.
    pub fn messages(&self) -> &[TaskMessage] {
        &self.messages
    }

    /// Returns the task execution start timestamp, if available.
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if the start timestamp was set, `None` otherwise.
    pub fn started_at(&self) -> Option<SystemTime> {
        self.started_at
    }

    /// Returns the task execution finish timestamp, if available.
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if the finish timestamp was set, `None` otherwise.
    pub fn finished_at(&self) -> Option<SystemTime> {
        self.finished_at
    }

    /// Returns the task execution duration in milliseconds, if available.
    ///
    /// # Returns
    ///
    /// `Some(u128)` if the duration was set, `None` otherwise.
    pub fn duration_ms(&self) -> Option<u128> {
        self.duration_ns
            .map(|duration_ns| duration_ns / 1_000_000)
            .or(self.duration_ms)
    }

    /// Returns the task execution duration in nanoseconds, if available.
    pub fn duration_ns(&self) -> Option<u128> {
        self.duration_ns.or_else(|| {
            self.duration_ms
                .map(|duration_ms| duration_ms.saturating_mul(1_000_000))
        })
    }

    /// Returns the task execution start timestamp in RFC 3339 format, if available.
    pub fn started_at_display(&self) -> Option<String> {
        self.started_at.map(format_timestamp_display)
    }

    /// Returns the task execution finish timestamp in RFC 3339 format, if available.
    pub fn finished_at_display(&self) -> Option<String> {
        self.finished_at.map(format_timestamp_display)
    }

    /// Returns the task execution duration in a human-readable format, if available.
    pub fn duration_display(&self) -> Option<String> {
        self.duration_ns().map(format_duration_display)
    }
}

/// Represents information about a skipped task execution.
///
/// `TaskSkip` captures details about why a task was not executed on a host.
/// Tasks can be skipped for various reasons, such as conditional logic (when clauses),
/// failed dependencies, maintenance mode, or other runtime conditions that prevent
/// execution. This structure provides both a machine-readable reason and a human-readable
/// message to explain the skip.
///
/// # Fields
///
/// * `reason` - An optional machine-readable reason code or identifier explaining why
///   the task was skipped (e.g., "parent_failed", "condition_not_met", "maintenance_mode").
///
/// * `message` - An optional human-readable message providing additional context about
///   why the task was skipped.
///
/// # Example
///
/// ```rust
/// use genja_core::task::TaskSkip;
///
/// let skip = TaskSkip::new()
///     .with_reason("condition_not_met")
///     .with_message("Host is not in the target environment");
///
/// assert_eq!(skip.reason(), Some("condition_not_met"));
/// assert_eq!(skip.message(), Some("Host is not in the target environment"));
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskSkip {
    reason: Option<String>,
    message: Option<String>,
}

impl TaskSkip {
    /// Creates a new `TaskSkip` instance with default values.
    ///
    /// This constructor initializes a `TaskSkip` with no reason or message set.
    /// Both fields will be `None` until explicitly set using the builder methods.
    ///
    /// # Returns
    ///
    /// A new `TaskSkip` instance with default values (no reason or message).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the machine-readable reason code explaining why the task was skipped.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The reason should be a concise identifier that
    /// can be used programmatically to categorize or filter skipped tasks.
    ///
    /// # Parameters
    ///
    /// * `reason` - A machine-readable reason code or identifier. Can be any type that
    ///   implements `Into<String>`, such as `&str`, `String`, or other string-like types.
    ///   Common examples include "condition_not_met", "parent_failed", or "maintenance_mode".
    ///
    /// # Returns
    ///
    /// The modified `TaskSkip` instance with the reason set.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Sets a human-readable message providing additional context about why the task was skipped.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The message should provide clear, user-friendly
    /// information about why the task was not executed.
    ///
    /// # Parameters
    ///
    /// * `message` - A human-readable explanation message. Can be any type that implements
    ///   `Into<String>`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// The modified `TaskSkip` instance with the message set.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Returns the machine-readable reason code, if available.
    ///
    /// # Returns
    ///
    /// `Some(&str)` if a reason was set, `None` otherwise.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Returns the human-readable message, if available.
    ///
    /// # Returns
    ///
    /// `Some(&str)` if a message was set, `None` otherwise.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

/// Represents a structured message generated during task execution.
///
/// `TaskMessage` captures detailed execution information with an associated severity level,
/// allowing tasks to emit structured logs, warnings, errors, and debug information during
/// execution. These messages provide a detailed execution trace that can be used for
/// debugging, auditing, and understanding task behavior beyond simple success or failure.
///
/// Messages can optionally include a machine-readable code for categorization and a
/// timestamp indicating when the message was generated.
///
/// # Fields
///
/// * `level` - The severity level of the message (Info, Warning, Error, Debug).
///
/// * `text` - The human-readable message text describing what occurred.
///
/// * `code` - An optional machine-readable code or identifier for categorizing or
///   filtering messages (e.g., "CONFIG_001", "WARN_DEPRECATED").
///
/// * `timestamp` - An optional timestamp indicating when the message was generated.
///
/// # Example
///
/// ```rust
/// use genja_core::task::{TaskMessage, MessageLevel};
/// use std::time::SystemTime;
///
/// let message = TaskMessage::new(MessageLevel::Warning, "Using deprecated API")
///     .with_code("WARN_DEPRECATED")
///     .with_timestamp(SystemTime::now());
///
/// assert_eq!(message.text(), "Using deprecated API");
/// assert_eq!(message.code(), Some("WARN_DEPRECATED"));
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct TaskMessage {
    level: MessageLevel,
    text: String,
    code: Option<String>,
    timestamp: Option<SystemTime>,
}

impl TaskMessage {
    /// Creates a new `TaskMessage` with the specified severity level and message text.
    ///
    /// This constructor initializes a `TaskMessage` with the provided level and text,
    /// with no code or timestamp set. Additional metadata can be added using the
    /// builder methods `with_code()` and `with_timestamp()`.
    ///
    /// # Parameters
    ///
    /// * `level` - The severity level of the message (Info, Warning, Error, or Debug).
    /// * `text` - The human-readable message text. Can be any type that implements
    ///   `Into<String>`, such as `&str`, `String`, or other string-like types.
    ///
    /// # Returns
    ///
    /// A new `TaskMessage` instance with the specified level and text, and no code
    /// or timestamp set.
    pub fn new(level: MessageLevel, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
            code: None,
            timestamp: None,
        }
    }

    /// Sets a machine-readable code for categorizing or filtering the message.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The code can be used to programmatically identify
    /// specific types of messages or group related messages together.
    ///
    /// # Parameters
    ///
    /// * `code` - A machine-readable code or identifier. Can be any type that implements
    ///   `Into<String>`, such as `&str`, `String`, or other string-like types. Common
    ///   examples include "CONFIG_001", "WARN_DEPRECATED", or "ERR_TIMEOUT".
    ///
    /// # Returns
    ///
    /// The modified `TaskMessage` instance with the code set.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Sets the timestamp indicating when the message was generated.
    ///
    /// This is a builder method that consumes `self` and returns the modified instance,
    /// allowing for method chaining. The timestamp helps track when events occurred
    /// during task execution.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - The timestamp when the message was generated.
    ///
    /// # Returns
    ///
    /// The modified `TaskMessage` instance with the timestamp set.
    pub fn with_timestamp(mut self, timestamp: SystemTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Returns the severity level of the message.
    ///
    /// # Returns
    ///
    /// A reference to the `MessageLevel` indicating the severity of this message.
    pub fn level(&self) -> &MessageLevel {
        &self.level
    }

    /// Returns the human-readable message text.
    ///
    /// # Returns
    ///
    /// A string slice containing the message text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the machine-readable code, if available.
    ///
    /// # Returns
    ///
    /// `Some(&str)` if a code was set, `None` otherwise.
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the timestamp when the message was generated, if available.
    ///
    /// # Returns
    ///
    /// `Some(SystemTime)` if a timestamp was set, `None` otherwise.
    pub fn timestamp(&self) -> Option<SystemTime> {
        self.timestamp
    }
}

/// Represents the severity level of a task message.
///
/// `MessageLevel` categorizes messages generated during task execution by their importance
/// and purpose. This allows consumers to filter, route, or display messages appropriately
/// based on their severity. The levels follow common logging conventions, from informational
/// messages to debug output.
///
/// # Variants
///
/// * `Info` - Informational messages that describe normal task execution progress or state.
///   These messages provide context about what the task is doing but don't indicate any issues.
///
/// * `Warning` - Warning messages that indicate potential issues or non-ideal conditions that
///   don't prevent task success. These should be reviewed but don't require immediate action.
///
/// * `Error` - Error messages that indicate serious problems, typically associated with task
///   failures. These messages describe what went wrong during execution.
///
/// * `Debug` - Debug messages that provide detailed technical information useful for
///   troubleshooting and development. These are typically more verbose and technical than
///   other message types.
///
/// # Example
///
/// ```rust
/// use genja_core::task::{TaskMessage, MessageLevel};
///
/// let info = TaskMessage::new(MessageLevel::Info, "Starting configuration deployment");
/// let warning = TaskMessage::new(MessageLevel::Warning, "Using deprecated API endpoint");
/// let error = TaskMessage::new(MessageLevel::Error, "Failed to connect to device");
/// let debug = TaskMessage::new(MessageLevel::Debug, "Raw response: {...}");
/// ```
#[derive(Debug, Clone, Serialize)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Debug,
}

/// Categorizes the type of failure that occurred during task execution.
///
/// `TaskFailureKind` provides a classification system for task failures, allowing
/// error handling logic to distinguish between different categories of errors and
/// respond appropriately. This classification helps with error reporting, retry
/// logic, and determining whether failures are transient or permanent.
///
/// # Variants
///
/// * `Connection` - The task failed due to a connection error, such as network
///   unreachability, connection refused, or connection dropped. These failures
///   are often transient and may succeed on retry.
///
/// * `Authentication` - The task failed due to authentication or authorization
///   issues, such as invalid credentials, expired tokens, or insufficient
///   permissions. These typically require credential updates or permission changes.
///
/// * `Validation` - The task failed due to validation errors in input data,
///   configuration, or parameters. This indicates that the task cannot proceed
///   with the provided data and requires correction.
///
/// * `Timeout` - The task failed because it exceeded a time limit. This could
///   indicate slow network conditions, an overloaded target system, or an
///   operation that takes longer than expected. Often retryable.
///
/// * `Command` - The task failed during command execution on the target system,
///   such as a command returning a non-zero exit code or producing unexpected
///   output. This indicates the operation itself failed on the remote system.
///
/// * `Unsupported` - The task failed because the requested operation is not
///   supported by the target system, plugin, or current configuration. This
///   typically indicates a permanent failure that won't succeed on retry.
///
/// * `Internal` - The task failed due to a Genja/framework internal error, such
///   as a programming error, resource exhaustion, or unexpected engine state.
///
/// * `External` - The task failed because a task implementation, plugin, or
///   external dependency returned an error that Genja captured and stored as a
///   host failure.
///
/// # Example
///
/// ```rust
/// use genja_core::task::{TaskFailure, TaskFailureKind};
/// use std::io;
///
/// let connection_failure = TaskFailure::new(
///     io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused")
/// )
/// .with_kind(TaskFailureKind::Connection)
/// .with_retryable(true);
///
/// let auth_failure = TaskFailure::new(
///     io::Error::new(io::ErrorKind::PermissionDenied, "invalid credentials")
/// )
/// .with_kind(TaskFailureKind::Authentication)
/// .with_retryable(false);
///
/// assert!(matches!(connection_failure.kind(), TaskFailureKind::Connection));
/// assert!(matches!(auth_failure.kind(), TaskFailureKind::Authentication));
/// ```
#[derive(Debug, Clone, Serialize)]
pub enum TaskFailureKind {
    Connection,
    Authentication,
    Validation,
    Timeout,
    Command,
    Unsupported,
    Internal,
    External,
}

/// Task metadata required for execution.
///
/// When using `#[derive(Task)]`, this trait is implemented automatically.
/// You do not need to import `TaskInfo` unless you reference it explicitly.
/// You still must implement `Task` manually to provide `start()`.
pub trait TaskInfo {
    /// Return the task's name.
    fn name(&self) -> &str;

    /// Return the task's plugin name.
    fn plugin_name(&self) -> &str;

    /// Build the task's connection key for a host.
    fn get_connection_key(&self, hostname: &str) -> crate::inventory::ConnectionKey;

    /// Return the task's options payload, if set.
    fn options(&self) -> Option<&Value>;
}

/// Sub-task provider interface.
pub trait SubTasks {
    /// Return any sub-tasks for this task.
    fn sub_tasks(&self) -> Vec<Arc<dyn Task>>;
}

/// Core task interface required for execution.
///
/// # Example
/// ```rust
/// use genja_core::task::Task;
/// use genja_core_derive::Task as TaskDerive;
///
/// #[derive(TaskDerive)]
/// struct MyTask {
///     name: String,
///     plugin_name: Option<String>,
/// }
///
/// impl Task for MyTask {
///     fn start(
///         &self,
///         _host: &genja_core::inventory::Host,
///     ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
///         Ok(genja_core::task::HostTaskResult::passed(
///             genja_core::task::TaskSuccess::new(),
///         ))
///     }
/// }
/// ```
pub trait Task: TaskInfo + SubTasks + Send + Sync {
    /// Start executing the task.
    fn start(&self, host: &Host) -> Result<HostTaskResult, TaskError>;
}

/// A wrapper around a task implementation that enforces the task trait flow.
///
/// `TaskDefinition` encapsulates a task that implements the `Task` trait, providing
/// a unified interface for task execution and management. This wrapper enables
/// polymorphic task handling while maintaining type safety through trait objects.
///
/// The wrapper provides access to the underlying task through trait object references,
/// allowing the task to be executed and queried without knowing its concrete type.
/// This is particularly useful for storing heterogeneous collections of tasks and
/// executing them uniformly.
///
/// # Fields
///
/// * `inner` - A boxed trait object containing the actual task implementation.
///   The task must implement the `Task` trait, which includes `TaskInfo` and
///   `SubTasks` traits, providing metadata, execution logic, and sub-task management.
///
/// # Example
///
/// ```rust
/// use genja_core::task::{Task, TaskDefinition, TaskInfo, SubTasks, HostTaskResult, TaskSuccess};
/// use genja_core::inventory::Host;
/// use std::sync::Arc;
/// use serde_json::Value;
///
/// struct MyTask {
///     name: String,
/// }
///
/// impl TaskInfo for MyTask {
///     fn name(&self) -> &str {
///         &self.name
///     }
///
///     fn plugin_name(&self) -> &str {
///         "ssh"
///     }
///
///     fn get_connection_key(&self, hostname: &str) -> genja_core::inventory::ConnectionKey {
///         genja_core::inventory::ConnectionKey::new(hostname, "ssh")
///     }
///
///     fn options(&self) -> Option<&Value> {
///         None
///     }
/// }
///
/// impl SubTasks for MyTask {
///     fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
///         Vec::new()
///     }
/// }
///
/// impl Task for MyTask {
///     fn start(&self, _host: &Host) -> Result<HostTaskResult, genja_core::task::TaskError> {
///         Ok(HostTaskResult::passed(TaskSuccess::new()))
///     }
/// }
///
/// let task = MyTask { name: "deploy".to_string() };
/// let definition = TaskDefinition::new(task);
/// assert_eq!(definition.name(), "deploy");
/// ```
#[derive(Clone)]
pub struct TaskDefinition {
    inner: Arc<dyn Task>,
}

impl TaskDefinition {
    /// Wrap a user-defined task that implements the Task trait.
    pub fn new<T: Task + 'static>(task: T) -> Self {
        Self {
            inner: Arc::new(task),
        }
    }

    /// Borrow the inner task as a trait object.
    pub fn as_task(&self) -> &dyn Task {
        self.inner.as_ref()
    }
}

impl TaskDefinition {
    /// Execute this task and all its sub-tasks recursively up to a maximum depth.
    ///
    /// This method starts the task execution by calling the task's `start()` method,
    /// then recursively executes all sub-tasks returned by `sub_tasks()`. The recursion
    /// is limited by the `max_depth` parameter to prevent infinite loops or excessive
    /// nesting.
    ///
    /// # Parameters
    ///
    /// * `max_depth` - The maximum depth of task nesting allowed. Depth is zero-based:
    ///   the root task runs at depth `0`, its immediate sub-tasks at depth `1`, and so on.
    ///   This means `max_depth = 0` allows only the root task, `max_depth = 1` allows
    ///   the root task plus one level of sub-tasks, and so on.
    ///
    /// # Returns
    ///
    /// Inserts the provided host's result into the shared `TaskResults` tree and
    /// recursively does the same for any sub-tasks. The parent task result is recorded
    /// before sub-task execution starts. Returns `Err(GenjaError)` if the maximum depth
    /// is exceeded.
    ///
    /// # Errors
    ///
    /// * Returns `GenjaError::Message` if the task nesting exceeds `max_depth`.
    pub fn start(
        &self,
        hostname: &str,
        host: &Host,
        results: &mut TaskResults,
        max_depth: usize,
    ) -> Result<(), crate::GenjaError> {
        Self::start_with_depth(self.inner.as_ref(), hostname, host, results, None, 0, max_depth)
    }

    /// Recursively executes a task and its sub-tasks with depth tracking.
    ///
    /// This internal helper method performs the actual recursive task execution,
    /// tracking the current depth to enforce the maximum depth limit. It executes
    /// the task by calling its `start()` method, stores the result, then recursively
    /// processes all sub-tasks returned by `sub_tasks()`.
    ///
    /// Sub-tasks are executed in iteration order. Results are grouped by task name, so
    /// a sub-task named `"validate"` produces a single `TaskResults` node containing
    /// host results for every host on which that sub-task ran.
    ///
    /// The method ensures that task nesting doesn't exceed the specified maximum
    /// depth, preventing infinite recursion or excessive nesting that could lead
    /// to stack overflow or performance issues.
    ///
    /// # Parameters
    ///
    /// * `task` - A reference to the task to execute, provided as a trait object.
    ///   This allows handling any type that implements the `Task` trait.
    ///
    /// * `hostname` - The name of the host on which the task is being executed.
    ///   Used as the key when storing task results.
    ///
    /// * `host` - A reference to the `Host` object representing the target system.
    ///   This is passed to the task's `start()` method for execution.
    ///
    /// * `results` - A mutable reference to the `TaskResults` structure where
    ///   execution results for this task and its sub-tasks will be stored.
    ///
    /// * `depth` - The current depth in the task execution tree. The root task
    ///   starts at depth 0, its immediate sub-tasks are at depth 1, and so on.
    ///
    /// * `max_depth` - The maximum allowed depth for task nesting. If `depth`
    ///   exceeds this value, the method returns an error and stops execution.
    ///   Because the check is `depth > max_depth`, a task at depth exactly equal
    ///   to `max_depth` is still allowed to run.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the task and all its sub-tasks executed successfully within
    ///   the depth limit.
    ///
    /// * `Err(GenjaError::Message)` if the current depth exceeds `max_depth`,
    ///   indicating that the task nesting is too deep.
    ///
    /// # Errors
    ///
    /// Returns `GenjaError::Message` with a descriptive error message if the
    /// task nesting depth exceeds the specified `max_depth` limit. The error
    /// message includes the maximum depth value that was exceeded.
    fn start_with_depth(
        task: &dyn Task,
        hostname: &str,
        host: &Host,
        results: &mut TaskResults,
        parent_task_name: Option<&str>,
        depth: usize,
        max_depth: usize,
    ) -> Result<(), crate::GenjaError> {
        if depth > max_depth {
            let started_at = SystemTime::now();
            let finished_at = started_at;
            let error =
                crate::GenjaError::Message(format!("max task depth exceeded: {}", max_depth));
            warn!(
                "max task depth exceeded for task '{}' at depth {} with max_depth {}",
                task.name(),
                depth,
                max_depth
            );
            results.record_execution_timing(started_at, finished_at);
            results.insert_host_result(
                hostname,
                HostTaskResult::failed(
                    TaskFailure::new(error)
                        .with_kind(TaskFailureKind::Internal)
                        .with_started_at(started_at)
                        .with_finished_at(finished_at)
                        .with_duration_ns(0),
                ),
            );
            return Ok(());
        }

        let started_at = SystemTime::now();
        let parent_task = parent_task_name.unwrap_or("none");
        debug!(
            "starting task '{}' for host '{}' parent_task='{}' depth={}",
            task.name(),
            hostname,
            parent_task,
            depth
        );
        let host_result = task.start(host);
        let finished_at = SystemTime::now();
        let duration_ns = finished_at
            .duration_since(started_at)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        results.record_execution_timing(started_at, finished_at);

        let host_result = match host_result {
            Ok(host_result) => host_result,
            Err(error) => {
                let failure = TaskFailure::from_task_error(error)
                    .with_started_at(started_at)
                    .with_finished_at(finished_at)
                    .with_duration_ns(duration_ns);
                warn!(
                    "task '{}' failed for host '{}': {}",
                    task.name(),
                    hostname,
                    failure.message()
                );
                let duration_display = failure
                    .duration_display()
                    .unwrap_or_else(|| format_duration_display(duration_ns));
                info!(
                    "finished task '{}' for host '{}' with status=failed duration_ms={} duration={}",
                    task.name(),
                    hostname,
                    duration_ns / 1_000_000,
                    duration_display
                );
                results.insert_host_result(hostname, HostTaskResult::failed(failure));
                return Ok(());
            }
        };

        if let Some(failure) = host_result.failure() {
            warn!(
                "task '{}' failed for host '{}': {}",
                task.name(),
                hostname,
                failure.message()
            );
        }

        if let Some(skip) = host_result.skipped_detail() {
            info!(
                "task '{}' skipped for host '{}' reason='{}' message='{}'",
                task.name(),
                hostname,
                skip.reason().unwrap_or("none"),
                skip.message().unwrap_or("")
            );
        }

        let status = if host_result.is_passed() {
            "passed"
        } else if host_result.is_failed() {
            "failed"
        } else {
            "skipped"
        };

        let host_result = host_result.with_execution_timing(started_at, finished_at, duration_ns);
        let duration_display = match &host_result {
            HostTaskResult::Passed(success) => success
                .duration_display()
                .unwrap_or_else(|| format_duration_display(duration_ns)),
            HostTaskResult::Failed(failure) => failure
                .duration_display()
                .unwrap_or_else(|| format_duration_display(duration_ns)),
            HostTaskResult::Skipped(_) => format_duration_display(duration_ns),
        };

        info!(
            "finished task '{}' for host '{}' with status={} duration_ms={} duration={}",
            task.name(),
            hostname,
            status,
            duration_ns / 1_000_000,
            duration_display
        );

        results.insert_host_result(hostname, host_result);

        for sub in task.sub_tasks() {
            let sub_task_name = sub.name().to_string();
            if results.sub_task(&sub_task_name).is_none() {
                results.insert_sub_task(sub_task_name.clone(), TaskResults::new(&sub_task_name));
            }
            let sub_results = results
                .sub_task_mut(&sub_task_name)
                .expect("sub task results should exist after insertion");
            Self::start_with_depth(
                sub.as_ref(),
                hostname,
                host,
                sub_results,
                Some(task.name()),
                depth + 1,
                max_depth,
            )?;
        }

        Ok(())
    }
}

impl TaskInfo for TaskDefinition {
    /// Returns the name of the task.
    ///
    /// This method delegates to the inner task's `name()` implementation, providing
    /// access to the task's identifier through the `TaskDefinition` wrapper.
    ///
    /// # Returns
    ///
    /// A string slice containing the task's name.
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns the name of the plugin associated with this task.
    ///
    /// This method delegates to the inner task's `plugin_name()` implementation,
    /// providing access to the plugin identifier that will handle the task's execution.
    ///
    /// # Returns
    ///
    /// A string slice containing the plugin's name (e.g., "ssh", "netconf", "restconf").
    fn plugin_name(&self) -> &str {
        self.inner.plugin_name()
    }

    /// Builds a connection key for the specified host.
    ///
    /// This method delegates to the inner task's `get_connection_key()` implementation,
    /// constructing a unique identifier that combines the hostname with the plugin name
    /// to identify the connection to be used for task execution.
    ///
    /// # Parameters
    ///
    /// * `hostname` - The name of the host for which to build the connection key.
    ///
    /// # Returns
    ///
    /// A `ConnectionKey` that uniquely identifies the connection to the specified host
    /// using this task's plugin.
    fn get_connection_key(&self, hostname: &str) -> crate::inventory::ConnectionKey {
        self.inner.get_connection_key(hostname)
    }

    /// Returns the task's options payload, if available.
    ///
    /// This method delegates to the inner task's `options()` implementation, providing
    /// access to any structured configuration or parameters associated with the task.
    ///
    /// # Returns
    ///
    /// `Some(&Value)` if the task has options configured, `None` otherwise.
    fn options(&self) -> Option<&Value> {
        self.inner.options()
    }
}

impl SubTasks for TaskDefinition {
    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.inner.sub_tasks()
    }
}

#[derive(Default)]
pub struct Tasks(Vec<TaskDefinition>);

impl Tasks {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add_task<T: Task + 'static>(&mut self, task: T) {
        self.0.push(TaskDefinition::new(task));
    }
}

impl Deref for Tasks {
    type Target = Vec<TaskDefinition>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::{BaseBuilderHost, ConnectionKey, Host};
    use log::{LevelFilter, Log, Metadata, Record};
    use serde_json::json;
    use std::sync::{Mutex, OnceLock};
    use std::fmt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct TestTaskFailureError;

    impl fmt::Display for TestTaskFailureError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "task failure test error")
        }
    }

    impl Error for TestTaskFailureError {}

    #[derive(Debug)]
    struct ExternalFailurePayload {
        code: u16,
    }

    impl fmt::Display for ExternalFailurePayload {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "external failure code {}", self.code)
        }
    }

    struct TestTask {
        name: &'static str,
        subs: Vec<Arc<dyn Task>>,
        counter: Arc<AtomicUsize>,
    }

    struct FailingTask;

    struct SkippingTask;

    impl TaskInfo for TestTask {
        fn name(&self) -> &str {
            self.name
        }

        fn plugin_name(&self) -> &str {
            "ssh"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, "ssh")
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for TestTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            self.subs.clone()
        }
    }

    impl Task for TestTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    impl TaskInfo for FailingTask {
        fn name(&self) -> &str {
            "failing"
        }

        fn plugin_name(&self) -> &str {
            "ssh"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, "ssh")
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for FailingTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            Vec::new()
        }
    }

    impl Task for FailingTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::failed(TaskFailure::new(
                TestTaskFailureError,
            )))
        }
    }

    impl TaskInfo for SkippingTask {
        fn name(&self) -> &str {
            "skipping"
        }

        fn plugin_name(&self) -> &str {
            "ssh"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, "ssh")
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for SkippingTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            Vec::new()
        }
    }

    impl Task for SkippingTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::Skipped(
                TaskSkip::new().with_reason("filtered"),
            ))
        }
    }

    #[derive(Default)]
    struct TestLogger {
        entries: Mutex<Vec<String>>,
    }

    impl TestLogger {
        fn clear(&self) {
            self.entries.lock().expect("logger lock should not be poisoned").clear();
        }

        fn entries(&self) -> Vec<String> {
            self.entries
                .lock()
                .expect("logger lock should not be poisoned")
                .clone()
        }
    }

    impl Log for TestLogger {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &Record<'_>) {
            if self.enabled(record.metadata()) {
                self.entries
                    .lock()
                    .expect("logger lock should not be poisoned")
                    .push(format!("{} {}", record.level(), record.args()));
            }
        }

        fn flush(&self) {}
    }

    fn test_logger() -> &'static TestLogger {
        static LOGGER: OnceLock<&'static TestLogger> = OnceLock::new();
        LOGGER.get_or_init(|| {
            let logger = Box::leak(Box::new(TestLogger::default()));
            let _ = log::set_logger(logger);
            log::set_max_level(LevelFilter::Debug);
            logger
        })
    }

    fn log_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn chain(depth: usize, counter: Arc<AtomicUsize>) -> Arc<dyn Task> {
        if depth == 1 {
            return Arc::new(TestTask {
                name: "leaf",
                subs: Vec::new(),
                counter,
            });
        }

        let child = chain(depth - 1, counter.clone());
        Arc::new(TestTask {
            name: "node",
            subs: vec![child],
            counter,
        })
    }

    #[test]
    fn start_runs_within_max_depth() {
        let counter = Arc::new(AtomicUsize::new(0));
        let root = chain(3, counter.clone());
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: vec![root],
            counter: counter.clone(),
        });
        let host = Host::builder().hostname("router1").build();

        let mut results = TaskResults::new("root");
        task.start("router1", &host, &mut results, 4)
            .expect("start should succeed");
        assert_eq!(counter.load(Ordering::SeqCst), 4);
        assert!(results.host_result("router1").is_some());
        assert!(results.sub_task("node").is_some());
        assert!(results.started_at().is_some());
        assert!(results.finished_at().is_some());
        assert!(results.duration_display().is_some());
        let node_results = results
            .sub_task("node")
            .expect("sub-task results should exist after execution");
        assert!(node_results.started_at().is_some());
        assert!(node_results.finished_at().is_some());
        assert!(node_results.duration_display().is_some());
    }

    #[test]
    fn start_captures_host_failure_when_depth_exceeds_limit() {
        let counter = Arc::new(AtomicUsize::new(0));
        let root = chain(5, counter.clone());
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: vec![root],
            counter: counter.clone(),
        });
        let host = Host::builder().hostname("router1").build();

        let mut results = TaskResults::new("root");
        task.start("router1", &host, &mut results, 4)
            .expect("start should capture depth overflow as a host failure");

        assert_eq!(counter.load(Ordering::SeqCst), 5);

        let level_one = results
            .sub_task("node")
            .expect("first nested node should exist");
        let level_two = level_one
            .sub_task("node")
            .expect("second nested node should exist");
        let level_three = level_two
            .sub_task("node")
            .expect("third nested node should exist");
        let level_four = level_three
            .sub_task("node")
            .expect("fourth nested node should exist");
        let level_five = level_four
            .sub_task("leaf")
            .expect("leaf task should capture failure");

        let failure = level_five
            .host_result("router1")
            .and_then(HostTaskResult::failure)
            .expect("depth overflow should be recorded as a host failure");
        assert!(failure.message().contains("max task depth exceeded"));
        assert!(matches!(failure.kind(), TaskFailureKind::Internal));
        assert!(failure.started_at().is_some());
        assert!(failure.finished_at().is_some());
        assert_eq!(failure.duration_ns(), Some(0));
    }

    #[test]
    fn start_attaches_timing_to_passed_host_results() {
        let counter = Arc::new(AtomicUsize::new(0));
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: Vec::new(),
            counter,
        });
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("root");

        task.start("router1", &host, &mut results, 0)
            .expect("start should succeed");

        let success = results
            .host_result("router1")
            .and_then(HostTaskResult::success)
            .expect("host result should be passed");
        assert!(success.started_at().is_some());
        assert!(success.finished_at().is_some());
        assert!(success.duration_ns().is_some());
        assert!(success.duration_display().is_some());
    }

    #[test]
    fn start_attaches_timing_to_failed_host_results() {
        let task = TaskDefinition::new(FailingTask);
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("failing");

        task.start("router1", &host, &mut results, 0)
            .expect("start should record a failed result");

        let failure = results
            .host_result("router1")
            .and_then(HostTaskResult::failure)
            .expect("host result should be failed");
        assert!(failure.started_at().is_some());
        assert!(failure.finished_at().is_some());
        assert!(failure.duration_ns().is_some());
        assert!(failure.duration_display().is_some());
    }

    #[test]
    fn start_does_not_attach_timing_to_skipped_host_results() {
        let task = TaskDefinition::new(SkippingTask);
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("skipping");

        task.start("router1", &host, &mut results, 0)
            .expect("start should record a skipped result");

        let skip = results
            .host_result("router1")
            .and_then(HostTaskResult::skipped_detail)
            .expect("host result should be skipped");
        assert_eq!(skip.reason(), Some("filtered"));
        assert_eq!(skip.message(), None);
    }

    #[test]
    fn start_logs_per_host_finish_for_passed_results() {
        let _guard = log_lock().lock().expect("log lock should not be poisoned");
        let logger = test_logger();
        logger.clear();

        let counter = Arc::new(AtomicUsize::new(0));
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: Vec::new(),
            counter,
        });
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("root");

        task.start("router1", &host, &mut results, 0)
            .expect("start should succeed");

        let entries = logger.entries();
        assert!(entries.iter().any(|entry| {
            entry.contains("DEBUG starting task 'root' for host 'router1' parent_task='none' depth=0")
        }));
        assert!(entries.iter().any(|entry| {
            entry.contains("INFO finished task 'root' for host 'router1' with status=passed duration_ms=")
                && entry.contains(" duration=")
        }));
    }

    #[test]
    fn start_logs_per_host_failure_warning_and_finish() {
        let _guard = log_lock().lock().expect("log lock should not be poisoned");
        let logger = test_logger();
        logger.clear();

        let task = TaskDefinition::new(FailingTask);
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("failing");

        task.start("router1", &host, &mut results, 0)
            .expect("start should record a failed result");

        let entries = logger.entries();
        assert!(entries.iter().any(|entry| {
            entry == "WARN task 'failing' failed for host 'router1': task failure test error"
        }));
        assert!(entries.iter().any(|entry| {
            entry.contains("INFO finished task 'failing' for host 'router1' with status=failed duration_ms=")
                && entry.contains(" duration=")
        }));
    }

    #[test]
    fn start_logs_per_host_skip_event_and_finish() {
        let _guard = log_lock().lock().expect("log lock should not be poisoned");
        let logger = test_logger();
        logger.clear();

        let task = TaskDefinition::new(SkippingTask);
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("skipping");

        task.start("router1", &host, &mut results, 0)
            .expect("start should record a skipped result");

        let entries = logger.entries();
        assert!(entries.iter().any(|entry| {
            entry == "INFO task 'skipping' skipped for host 'router1' reason='filtered' message=''"
        }));
        assert!(entries.iter().any(|entry| {
            entry.contains("INFO finished task 'skipping' for host 'router1' with status=skipped duration_ms=")
                && entry.contains(" duration=")
        }));
    }

    #[test]
    fn task_failure_preserves_metadata_and_supports_downcast() {
        let failure = TaskFailure::new(TestTaskFailureError)
            .with_kind(TaskFailureKind::Connection)
            .with_retryable(true)
            .with_details(json!({"port": 22}))
            .with_warning("intermittent reachability")
            .with_message(TaskMessage::new(MessageLevel::Error, "ssh session failed"));

        assert_eq!(failure.message(), "task failure test error");
        assert_eq!(failure.error().to_string(), "task failure test error");
        assert!(matches!(failure.kind(), TaskFailureKind::Connection));
        assert!(failure.retryable());
        assert_eq!(failure.details(), Some(&json!({"port": 22})));
        assert_eq!(failure.warnings(), ["intermittent reachability"]);
        assert_eq!(failure.messages()[0].text(), "ssh session failed");
        assert!(failure
            .error_type()
            .ends_with("task::tests::TestTaskFailureError"));
        assert!(failure.downcast_ref::<TestTaskFailureError>().is_some());
    }

    #[test]
    fn task_failure_capture_supports_non_error_payloads() {
        let failure = TaskFailure::capture(ExternalFailurePayload { code: 42 })
            .with_kind(TaskFailureKind::Internal);

        assert_eq!(failure.message(), "external failure code 42");
        assert!(failure
            .error()
            .to_string()
            .contains("external failure code 42"));
        assert!(failure
            .error_type()
            .ends_with("task::tests::ExternalFailurePayload"));
        let payload = failure
            .downcast_ref::<ExternalFailurePayload>()
            .expect("captured payload should be downcastable");
        assert_eq!(payload.code, 42);
    }

    #[derive(Debug)]
    struct ExternalTaskError;

    impl fmt::Display for ExternalTaskError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "external task error")
        }
    }

    impl Error for ExternalTaskError {}

    struct ErroringTask;

    impl TaskInfo for ErroringTask {
        fn name(&self) -> &str {
            "erroring"
        }

        fn plugin_name(&self) -> &str {
            "ssh"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, "ssh")
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for ErroringTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            Vec::new()
        }
    }

    impl Task for ErroringTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Err(TaskError::new(ExternalTaskError))
        }
    }

    #[test]
    fn start_captures_task_errors_as_external_failures() {
        let task = TaskDefinition::new(ErroringTask);
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("erroring");

        task.start("router1", &host, &mut results, 0)
            .expect("start should capture task error as host failure");

        let failure = results
            .host_result("router1")
            .and_then(HostTaskResult::failure)
            .expect("task error should be recorded as failure");
        assert_eq!(failure.message(), "external task error");
        assert!(matches!(failure.kind(), TaskFailureKind::External));
        assert!(failure
            .error_type()
            .ends_with("task::tests::ExternalTaskError"));
    }

    #[test]
    fn task_success_builders_expose_extended_metadata() {
        let started_at = SystemTime::UNIX_EPOCH;
        let finished_at = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(2))
            .expect("valid timestamp");
        let success = TaskSuccess::new()
            .with_result(json!({"ok": true}))
            .with_changed(true)
            .with_diff("updated config")
            .with_summary("task completed")
            .with_warning("minor drift")
            .with_message(
                TaskMessage::new(MessageLevel::Info, "commit complete").with_code("commit_ok"),
            )
            .with_metadata(json!({"version": 1}))
            .with_started_at(started_at)
            .with_finished_at(finished_at)
            .with_duration_ms(2000);

        assert_eq!(success.result(), Some(&json!({"ok": true})));
        assert!(success.changed());
        assert_eq!(success.diff(), Some("updated config"));
        assert_eq!(success.summary(), Some("task completed"));
        assert_eq!(success.warnings(), ["minor drift"]);
        assert_eq!(success.messages()[0].text(), "commit complete");
        assert_eq!(success.messages()[0].code(), Some("commit_ok"));
        assert!(matches!(success.messages()[0].level(), MessageLevel::Info));
        assert_eq!(success.metadata(), Some(&json!({"version": 1})));
        assert_eq!(success.started_at(), Some(started_at));
        assert_eq!(success.finished_at(), Some(finished_at));
        assert_eq!(success.duration_ms(), Some(2000));
    }

    #[test]
    fn task_skip_and_host_task_result_expose_skip_metadata() {
        let skipped = HostTaskResult::Skipped(
            TaskSkip::new()
                .with_reason("filtered")
                .with_message("host excluded by selector"),
        );

        assert!(skipped.is_skipped());
        assert_eq!(
            skipped.skipped_detail().and_then(TaskSkip::reason),
            Some("filtered")
        );
        assert_eq!(
            skipped.skipped_detail().and_then(TaskSkip::message),
            Some("host excluded by selector")
        );

        let skipped_with_reason = HostTaskResult::skipped_with_reason("parent_failed");
        assert_eq!(
            skipped_with_reason
                .skipped_detail()
                .and_then(TaskSkip::reason),
            Some("parent_failed")
        );
    }

    #[test]
    fn task_message_builders_expose_message_metadata() {
        let timestamp = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(1))
            .expect("valid timestamp");
        let message = TaskMessage::new(MessageLevel::Warning, "latency threshold exceeded")
            .with_code("latency_warn")
            .with_timestamp(timestamp);

        assert!(matches!(message.level(), MessageLevel::Warning));
        assert_eq!(message.text(), "latency threshold exceeded");
        assert_eq!(message.code(), Some("latency_warn"));
        assert_eq!(message.timestamp(), Some(timestamp));
    }

    #[test]
    fn task_results_builders_expose_summary_and_timing_metadata() {
        let started_at = SystemTime::UNIX_EPOCH;
        let finished_at = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(3))
            .expect("valid timestamp");
        let results = TaskResults::new("deploy")
            .with_summary("deploy finished")
            .with_started_at(started_at)
            .with_finished_at(finished_at)
            .with_duration_ms(3000);

        assert_eq!(results.task_name(), "deploy");
        assert_eq!(results.summary(), Some("deploy finished"));
        assert_eq!(results.started_at(), Some(started_at));
        assert_eq!(results.finished_at(), Some(finished_at));
        assert_eq!(results.duration_ns(), Some(3_000_000_000));
        assert_eq!(results.duration_ms(), Some(3000));
        assert_eq!(
            results.started_at_display(),
            Some("1970-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            results.finished_at_display(),
            Some("1970-01-01T00:00:03Z".to_string())
        );
        assert_eq!(results.duration_display(), Some("3s".to_string()));

        let json = results
            .to_json_string()
            .expect("human json should serialize");
        assert!(json.contains("\"started_at\":\"1970-01-01T00:00:00Z\""));
        assert!(json.contains("\"finished_at\":\"1970-01-01T00:00:03Z\""));
        assert!(json.contains("\"duration\":\"3s\""));
        assert!(!json.contains("\"duration_ns\":"));
        assert!(!json.contains("\"duration_ms\":"));

        let raw_json = results
            .to_raw_json_string()
            .expect("raw json should serialize");
        assert!(!raw_json.contains("\"duration\":\"3s\""));
        assert!(raw_json.contains("\"duration_ns\":3000000000"));
    }

    #[test]
    fn task_results_human_json_serializes_recursive_sub_tasks() {
        let child = TaskResults::new("child").with_duration_ms(250);
        let mut root = TaskResults::new("root").with_duration_ms(2000);
        root.insert_sub_task("child", child);

        let json = root
            .to_json_string()
            .expect("human json should serialize recursively");

        assert!(json.contains("\"task_name\":\"root\""));
        assert!(json.contains("\"sub_tasks\":{\"child\":{\"task_name\":\"child\""));
        assert!(json.contains("\"duration\":\"2s\""));
        assert!(json.contains("\"duration\":\"250ms\""));
    }

    #[test]
    fn sub_task_results_human_json_includes_aggregate_timing() {
        let counter = Arc::new(AtomicUsize::new(0));
        let root = chain(2, counter.clone());
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: vec![root],
            counter,
        });
        let host = Host::builder().hostname("router1").build();
        let mut results = TaskResults::new("root");

        task.start("router1", &host, &mut results, 3)
            .expect("start should succeed");

        let json = results
            .to_json_string()
            .expect("human json should serialize sub-task timing");

        assert!(json.contains("\"sub_tasks\":{\"node\":{"));
        assert!(json.contains("\"started_at\":\""));
        assert!(json.contains("\"finished_at\":\""));
        assert!(json.contains("\"duration\":\""));
    }

    #[test]
    fn task_results_human_json_formats_host_timing_uniformly() {
        let started_at = SystemTime::UNIX_EPOCH;
        let finished_at = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_millis(2))
            .expect("valid timestamp");
        let mut results = TaskResults::new("deploy");
        results.insert_host_result(
            "router1",
            HostTaskResult::passed(
                TaskSuccess::new()
                    .with_summary("ok")
                    .with_started_at(started_at)
                    .with_finished_at(finished_at)
                    .with_duration_ns(2_000_000),
            ),
        );
        results.insert_host_result(
            "router2",
            HostTaskResult::failed(
                TaskFailure::new(TestTaskFailureError)
                    .with_started_at(started_at)
                    .with_finished_at(finished_at)
                    .with_duration_ns(250_000),
            ),
        );
        results.insert_host_result(
            "router3",
            HostTaskResult::Skipped(TaskSkip::new().with_reason("filtered")),
        );

        let json = results
            .to_json_string()
            .expect("human json should serialize host timing");

        assert!(json.contains("\"router1\":{\"Passed\":{"));
        assert!(json.contains("\"summary\":\"ok\""));
        assert!(json.contains("\"started_at\":\"1970-01-01T00:00:00Z\""));
        assert!(json.contains("\"finished_at\":\"1970-01-01T00:00:00Z\""));
        assert!(json.contains("\"duration\":\"2ms\""));
        assert!(json.contains("\"router2\":{\"Failed\":"));
        assert!(json.contains("\"duration\":\"250us\""));
        assert!(json.contains("\"router3\":{\"Skipped\":{\"reason\":\"filtered\""));
        assert!(!json.contains("\"router3\":{\"Skipped\":{\"started_at\""));
        assert!(!json.contains("\"duration_ns\""));
        assert!(!json.contains("\"duration_ms\""));
    }

    #[test]
    fn task_results_duration_display_preserves_sub_millisecond_precision() {
        let micros = TaskResults::new("micros").with_duration_ns(250_000);
        let nanos = TaskResults::new("nanos").with_duration_ns(250);
        let millis = TaskResults::new("millis").with_duration_ns(2_500_000);
        let seconds = TaskResults::new("seconds").with_duration_ns(1_500_587_737);

        assert_eq!(micros.duration_ns(), Some(250_000));
        assert_eq!(micros.duration_ms(), Some(0));
        assert_eq!(micros.duration_display(), Some("250us".to_string()));

        assert_eq!(nanos.duration_ns(), Some(250));
        assert_eq!(nanos.duration_ms(), Some(0));
        assert_eq!(nanos.duration_display(), Some("250ns".to_string()));

        assert_eq!(millis.duration_display(), Some("2.5ms".to_string()));
        assert_eq!(seconds.duration_display(), Some("1.5s".to_string()));
    }

    #[test]
    fn task_results_store_recursive_host_and_sub_task_results() {
        let mut root = TaskResults::new("deploy").with_summary("deploy completed");
        root.insert_host_result(
            "router1",
            HostTaskResult::passed(
                TaskSuccess::new()
                    .with_result(json!({"deployed": true}))
                    .with_changed(true)
                    .with_summary("config deployed")
                    .with_warning("candidate config had comments")
                    .with_message(TaskMessage::new(
                        MessageLevel::Info,
                        "candidate config committed",
                    ))
                    .with_metadata(json!({"version": "1.2.3"})),
            ),
        );
        root.insert_host_result(
            "router2",
            HostTaskResult::failed(
                TaskFailure::new(TestTaskFailureError)
                    .with_kind(TaskFailureKind::Connection)
                    .with_retryable(true),
            ),
        );

        let mut validate = TaskResults::new("validate");
        validate.insert_host_result(
            "router1",
            HostTaskResult::passed(TaskSuccess::new().with_result(json!({"valid": true}))),
        );
        validate.insert_host_result(
            "router2",
            HostTaskResult::Skipped(
                TaskSkip::new()
                    .with_reason("parent_failed")
                    .with_message("validation skipped because deploy failed"),
            ),
        );

        let mut collect_logs = TaskResults::new("collect_logs");
        collect_logs.insert_host_result(
            "router1",
            HostTaskResult::passed(TaskSuccess::new().with_diff("captured logs")),
        );
        collect_logs.insert_host_result(
            "router2",
            HostTaskResult::skipped_with_reason("parent_failed"),
        );

        validate.insert_sub_task("collect_logs", collect_logs);
        root.insert_sub_task("validate", validate);

        assert_eq!(root.task_name(), "deploy");
        assert_eq!(
            root.passed_hosts()
                .into_iter()
                .map(|host| host.as_str())
                .collect::<Vec<_>>(),
            vec!["router1"]
        );
        assert_eq!(
            root.failed_hosts()
                .into_iter()
                .map(|host| host.as_str())
                .collect::<Vec<_>>(),
            vec!["router2"]
        );

        let validate = root
            .sub_task("validate")
            .expect("validate sub task should exist");
        assert_eq!(validate.task_name(), "validate");
        assert_eq!(root.summary(), Some("deploy completed"));
        assert_eq!(
            root.host_result("router1")
                .and_then(HostTaskResult::success)
                .and_then(TaskSuccess::summary),
            Some("config deployed")
        );
        assert_eq!(
            root.host_result("router1")
                .and_then(HostTaskResult::success)
                .map(TaskSuccess::warnings)
                .map(|warnings| warnings.len()),
            Some(1)
        );
        assert!(validate
            .host_result("router2")
            .expect("router2 validate result should exist")
            .is_skipped());
        assert_eq!(
            validate
                .host_result("router2")
                .and_then(HostTaskResult::skipped_detail)
                .and_then(TaskSkip::reason),
            Some("parent_failed")
        );

        let collect_logs = validate
            .sub_task("collect_logs")
            .expect("collect_logs sub task should exist");
        assert_eq!(collect_logs.task_name(), "collect_logs");
        assert_eq!(
            collect_logs
                .host_result("router1")
                .and_then(HostTaskResult::success)
                .and_then(TaskSuccess::diff),
            Some("captured logs")
        );
        assert_eq!(
            root.host_result("router2")
                .and_then(HostTaskResult::failure)
                .map(TaskFailure::retryable),
            Some(true)
        );

        let root_summary = root.task_summary();
        assert_eq!(root_summary.task_name(), "deploy");
        assert_eq!(root_summary.hosts().passed(), 1);
        assert_eq!(root_summary.hosts().failed(), 1);
        assert_eq!(root_summary.hosts().skipped(), 0);
        assert_eq!(root_summary.hosts().total(), 2);
        assert_eq!(root_summary.duration_ms(), None);

        let validate_summary = root_summary
            .sub_tasks()
            .get("validate")
            .expect("validate summary should exist");
        assert_eq!(validate_summary.task_name(), "validate");
        assert_eq!(validate_summary.hosts().passed(), 1);
        assert_eq!(validate_summary.hosts().failed(), 0);
        assert_eq!(validate_summary.hosts().skipped(), 1);
        assert_eq!(validate_summary.duration_ms(), None);

        let collect_logs_summary = validate_summary
            .sub_tasks()
            .get("collect_logs")
            .expect("collect_logs summary should exist");
        assert_eq!(collect_logs_summary.task_name(), "collect_logs");
        assert_eq!(collect_logs_summary.hosts().passed(), 1);
        assert_eq!(collect_logs_summary.hosts().failed(), 0);
        assert_eq!(collect_logs_summary.hosts().skipped(), 1);
    }
}
