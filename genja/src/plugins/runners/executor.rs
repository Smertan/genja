use genja_core::inventory::{Host, Hosts};
use genja_core::task::{TaskConnectionResolver, TaskDefinition, TaskInfo, TaskResults};
use genja_core::{GenjaError, NatString};
use std::sync::Arc;
use std::time::SystemTime;


/// Shared execution helper for built-in runner plugins.
///
/// `TaskExecutor` provides a reusable mechanism for executing task definitions
/// across multiple hosts with configurable connection resolution and recursion depth.
///
/// # Fields
///
/// * `hosts` - A reference to the collection of hosts on which tasks will be executed.
/// * `connection_resolver` - An optional connection resolver for establishing connections to hosts.
///   If `None`, tasks will execute without custom connection resolution.
/// * `max_depth` - The maximum recursion depth allowed for nested task execution,
///   preventing infinite loops in task dependencies.
#[derive(Debug)]
pub(crate) struct TaskExecutor<'a> {
    hosts: &'a Hosts,
    connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
    max_depth: usize,
}

impl<'a> TaskExecutor<'a> {
    /// Creates a new `TaskExecutor` instance.
    ///
    /// Initializes a task executor with the specified hosts, connection resolver, and recursion depth limit.
    /// This executor can then be used to run task definitions across the provided host inventory.
    ///
    /// # Parameters
    ///
    /// * `hosts` - A reference to the collection of hosts on which tasks will be executed.
    /// * `connection_resolver` - An optional connection resolver for establishing connections to hosts.
    ///   If `None`, tasks will execute without custom connection resolution.
    /// * `max_depth` - The maximum recursion depth allowed for nested task execution,
    ///   preventing infinite loops in task dependencies.
    ///
    /// # Returns
    ///
    /// Returns a new `TaskExecutor` instance configured with the provided parameters.
    pub(crate) fn new(
        hosts: &'a Hosts,
        connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
        max_depth: usize,
    ) -> Self {
        Self {
            hosts,
            connection_resolver,
            max_depth,
        }
    }

    /// Executes a task definition across all hosts in the inventory.
    ///
    /// This method orchestrates the execution of a task definition by:
    /// 1. Processing the task start lifecycle hook
    /// 2. Running the task on each host in the inventory sequentially
    /// 3. Merging individual host results into a consolidated result set
    /// 4. Processing the task finish lifecycle hook
    /// 5. Recording timing information for the entire execution
    ///
    /// # Parameters
    ///
    /// * `task_definition` - The task definition to execute, containing the task configuration,
    ///   lifecycle hooks, and execution logic.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TaskResults)` containing the aggregated results from all hosts, including
    /// execution timing and status information. Returns `Err(GenjaError)` if any lifecycle hook
    /// fails or if host execution encounters an error.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The task start processing fails
    /// * Any host execution fails
    /// * The task finish processing fails
    pub(crate) async fn run_definition(
        &self,
        task_definition: &TaskDefinition,
    ) -> Result<TaskResults, GenjaError> {
        let started_at = SystemTime::now();
        let mut results = TaskResults::new(task_definition.name()).with_started_at(started_at);
        task_definition.process_task_start(&mut results)?;

        for (host_id, host) in self.hosts.iter() {
            results.merge(Self::run_host(
                task_definition,
                host_id,
                host,
                self.connection_resolver.clone(),
                self.max_depth,
            )
            .await?);
        }

        let finished_at = SystemTime::now();
        let duration_ns = finished_at
            .duration_since(started_at)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        results = results
            .with_finished_at(finished_at)
            .with_duration_ns(duration_ns);
        task_definition.process_task_finish(&mut results)?;
        Ok(results)
    }

    /// Executes a task definition on a single host.
    ///
    /// This method runs a task definition on a specific host by delegating to the task's
    /// `start_with_connection_resolver` method. It initializes a new result set for the host
    /// and populates it with the execution outcome.
    ///
    /// # Parameters
    ///
    /// * `task_definition` - The task definition to execute, containing the task configuration
    ///   and execution logic.
    /// * `host_id` - The unique identifier of the host on which to execute the task.
    /// * `host` - A reference to the host object containing host-specific configuration and state.
    /// * `connection_resolver` - An optional connection resolver for establishing connections to the host.
    ///   If `None`, the task will execute without custom connection resolution.
    /// * `max_depth` - The maximum recursion depth allowed for nested task execution,
    ///   preventing infinite loops in task dependencies.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TaskResults)` containing the execution results for the specified host,
    /// including status information and any output generated during execution.
    /// Returns `Err(GenjaError)` if the task execution fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The connection to the host cannot be established
    /// * The task execution fails on the host
    /// * The maximum recursion depth is exceeded
    pub(crate) async fn run_host(
        task_definition: &TaskDefinition,
        host_id: &NatString,
        host: &Host,
        connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        let mut results = TaskResults::new(task_definition.name());
        task_definition.start_with_connection_resolver(
            host_id.as_str(),
            host,
            &mut results,
            connection_resolver.as_deref(),
            max_depth,
        )
        .await?;
        Ok(results)
    }
}
