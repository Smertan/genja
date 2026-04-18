use crate::inventory::Host;
use crate::types::{CustomTreeMap, NatString};
use log::warn;
use serde::Serialize;
use serde_json::Value;
use std::any::type_name;
use std::error::Error;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::SystemTime;

pub type TaskError = Arc<dyn Error + Send + Sync + 'static>;

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
    duration_ms: Option<u128>,
    summary: Option<String>,
    hosts: CustomTreeMap<HostTaskResult>,
    sub_tasks: CustomTreeMap<TaskResults>,
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
        self.duration_ms = Some(duration_ms);
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
        self.duration_ms
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
        self.duration_ms = Some(duration_ms);
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
        self.duration_ms
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
    kind: TaskFailureKind,
    error_type: String,
    message: String,
    retryable: bool,
    details: Option<Value>,
    warnings: Vec<String>,
    messages: Vec<TaskMessage>,
    started_at: Option<SystemTime>,
    finished_at: Option<SystemTime>,
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
        Self {
            kind: TaskFailureKind::Internal,
            error_type: type_name::<E>().to_string(),
            message: error.to_string(),
            error: Arc::new(error),
            retryable: false,
            details: None,
            warnings: Vec::new(),
            messages: Vec::new(),
            started_at: None,
            finished_at: None,
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
        self.duration_ms = Some(duration_ms);
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
        E: Error + 'static,
    {
        self.error.downcast_ref::<E>()
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
        self.error.as_ref()
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
        self.duration_ms
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
/// * `Internal` - The task failed due to an internal error in the task
///   implementation or framework, such as a programming error, resource
///   exhaustion, or unexpected state. This is the default classification.
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
///     fn start(&self, _host: &genja_core::inventory::Host) -> genja_core::task::HostTaskResult {
///         genja_core::task::HostTaskResult::passed(genja_core::task::TaskSuccess::new())
///     }
/// }
/// ```
pub trait Task: TaskInfo + SubTasks {
    /// Start executing the task.
    fn start(&self, host: &Host) -> HostTaskResult;

    // TODO: should have a function to execute the task with args,
    // (host: Host, args, serde_json::value))
    // fn start
    // Based on a per host basis#
}

/// A task wrapper that enforces the task trait flow.
pub struct TaskDefinition {
    inner: Box<dyn Task>,
}

impl TaskDefinition {
    /// Wrap a user-defined task that implements the Task trait.
    pub fn new<T: Task + 'static>(task: T) -> Self {
        Self {
            inner: Box::new(task),
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
    /// * `max_depth` - The maximum depth of task nesting allowed. A depth of 1 means
    ///   only the root task will execute. A depth of 2 allows the root task plus one
    ///   level of sub-tasks, and so on.
    ///
    /// # Returns
    ///
    /// Inserts the provided host's result into the shared `TaskResults` tree and
    /// recursively does the same for any sub-tasks. Returns `Err(GenjaError)` if
    /// the maximum depth is exceeded.
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
        Self::start_with_depth(self.inner.as_ref(), hostname, host, results, 0, max_depth)
    }

    fn start_with_depth(
        task: &dyn Task,
        hostname: &str,
        host: &Host,
        results: &mut TaskResults,
        depth: usize,
        max_depth: usize,
    ) -> Result<(), crate::GenjaError> {
        if depth > max_depth {
            warn!(
                "max task depth exceeded for task '{}' at depth {} with max_depth {}",
                task.name(),
                depth,
                max_depth
            );
            return Err(crate::GenjaError::Message(format!(
                "max task depth exceeded: {}",
                max_depth
            )));
        }

        results.insert_host_result(hostname, task.start(host));

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
                depth + 1,
                max_depth,
            )?;
        }

        Ok(())
    }
}

impl TaskInfo for TaskDefinition {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn plugin_name(&self) -> &str {
        self.inner.plugin_name()
    }

    fn get_connection_key(&self, hostname: &str) -> crate::inventory::ConnectionKey {
        self.inner.get_connection_key(hostname)
    }

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
    use serde_json::json;
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

    struct TestTask {
        name: &'static str,
        subs: Vec<Arc<dyn Task>>,
        counter: Arc<AtomicUsize>,
    }

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
        fn start(&self, _host: &Host) -> HostTaskResult {
            self.counter.fetch_add(1, Ordering::SeqCst);
            HostTaskResult::passed(TaskSuccess::new())
        }
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
    }

    #[test]
    fn start_fails_when_depth_exceeds_limit() {
        let counter = Arc::new(AtomicUsize::new(0));
        let root = chain(5, counter.clone());
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: vec![root],
            counter: counter.clone(),
        });
        let host = Host::builder().hostname("router1").build();

        let mut results = TaskResults::new("root");
        let err = task
            .start("router1", &host, &mut results, 4)
            .expect_err("start should fail at depth > 4");
        match err {
            crate::GenjaError::Message(msg) => {
                assert!(msg.contains("max task depth exceeded"));
            }
            _ => panic!("unexpected error variant"),
        }
        assert_eq!(counter.load(Ordering::SeqCst), 5);
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
    }
}
